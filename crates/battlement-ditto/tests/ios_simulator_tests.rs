#![cfg(target_os = "macos")]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

use battlement_ditto::{
  config::model::Orientation,
  ios_simulator::{IosSimulator, SimulatorTools, orient_display},
  player_supervision::SimulatorApp,
};
use tempfile::TempDir;

#[test]
fn exact_device_boots_launches_copies_media_and_deletes() {
  let fixture = Fixture::new();
  let (mut simulator, portrait) =
    IosSimulator::create(fixture.tools(), "iPhone 16 Pro", "01234567-89ab-cdef").unwrap();
  assert_eq!(simulator.udid(), "AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE");
  assert!(simulator.name().starts_with("Battlement Ditto "));
  assert_eq!(portrait.display.width, 1206);
  assert_eq!(portrait.display.height, 2622);
  assert_eq!(portrait.display.scale, 3.0);
  let landscape = orient_display(portrait, Orientation::LandscapeLeft);
  assert_eq!(landscape.display.width, 2622);
  assert_eq!(landscape.display.height, 1206);

  simulator
    .install_and_launch(
      &fixture.app,
      "http://127.0.0.1:43123/ditto/token",
      Orientation::LandscapeLeft,
    )
    .unwrap();
  assert!(simulator.is_running().unwrap());
  let log = fixture.path("retained.log");
  simulator.retain_logs(&log).unwrap();
  assert!(fs::read_to_string(log).unwrap().contains("scoped app log"));
  let recording = fixture.path("host/video.raw");
  simulator
    .copy_recording("Documents/video.raw", &recording)
    .unwrap();
  assert_eq!(fs::read(recording).unwrap(), b"frames");
  simulator.terminate().unwrap();

  let transcript = fs::read_to_string(&fixture.transcript).unwrap();
  assert!(transcript.contains("launch --terminate-running-process AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE com.example.ditto --battlement-ditto-url http://127.0.0.1:43123/ditto/token --battlement-ditto-orientation landscape-left"));
  assert!(transcript.contains("env-url=http://127.0.0.1:43123/ditto/token"));
  assert!(transcript.contains("spawn AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE /bin/kill -0 4242"));
  assert!(transcript.contains("delete AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"));
}

#[test]
fn unavailable_device_lists_installed_alternatives_without_creating() {
  let fixture = Fixture::new();
  let error = match IosSimulator::create(fixture.tools(), "Imaginary Phone", "session") {
    Ok(_) => panic!("unavailable device unexpectedly resolved"),
    Err(error) => error,
  };
  assert!(error.to_string().contains("iPhone 16 Pro"));
  assert!(
    !fs::read_to_string(&fixture.transcript)
      .unwrap()
      .contains("create ")
  );
}

#[test]
fn boot_failure_deletes_the_created_device() {
  let fixture = Fixture::new();
  fs::write(fixture.path("fail-boot"), []).unwrap();
  let error = match IosSimulator::create(fixture.tools(), "iPhone 16 Pro", "session") {
    Ok(_) => panic!("failed boot unexpectedly succeeded"),
    Err(error) => error,
  };
  assert!(error.to_string().contains("device was deleted"));
  let transcript = fs::read_to_string(&fixture.transcript).unwrap();
  assert!(transcript.contains("shutdown AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"));
  assert!(transcript.contains("delete AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE"));
}

struct Fixture {
  root: TempDir,
  transcript: PathBuf,
  xcrun: PathBuf,
  plutil: PathBuf,
  app: PathBuf,
}

impl Fixture {
  fn new() -> Self {
    let root = TempDir::new().unwrap();
    let transcript = root.path().join("transcript");
    fs::write(&transcript, []).unwrap();
    let xcrun = root.path().join("xcrun");
    let plutil = root.path().join("plutil");
    let app = root.path().join("BattlementDitto.app");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("Info.plist"), "fixture").unwrap();
    fs::create_dir_all(root.path().join("data/Documents")).unwrap();
    fs::write(root.path().join("data/Documents/video.raw"), b"frames").unwrap();
    Self::executable(
      &xcrun,
      &format!(
        r#"#!/bin/sh
set -eu
root='{}'
shift
printf '%s\n' "$*" >> "$root/transcript"
if [ "$1" = "launch" ]; then
  printf 'env-url=%s\n' "${{SIMCTL_CHILD_BATTLEMENT_DITTO_URL-}}" >> "$root/transcript"
fi
case "$1" in
  list)
    if [ "$2" = "--json" ]; then
      printf '%s\n' '{{"runtimes":[{{"name":"iOS 26.4","identifier":"com.apple.CoreSimulator.SimRuntime.iOS-26-4","version":"26.4","isAvailable":true}}],"devicetypes":[{{"name":"iPhone 16 Pro","identifier":"com.apple.CoreSimulator.SimDeviceType.iPhone-16-Pro"}}]}}'
    else
      printf '%s\n' '{{"devices":{{"runtime":[{{"state":"Booted"}}]}}}}'
    fi ;;
  create) printf '%s\n' 'AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE' ;;
  boot) [ ! -f "$root/fail-boot" ] ;;
  getenv)
    case "$3" in
      SIMULATOR_MAINSCREEN_WIDTH) printf '1206\n' ;;
      SIMULATOR_MAINSCREEN_HEIGHT) printf '2622\n' ;;
      SIMULATOR_MAINSCREEN_SCALE) printf '3.0\n' ;;
    esac ;;
  install|terminate|shutdown|delete) : ;;
  launch) printf 'com.example.ditto: 4242\n' ;;
  spawn)
    if [ "$3" = "log" ]; then printf 'scoped app log\n'; fi ;;
  get_app_container) printf '%s/data\n' "$root" ;;
esac
"#,
        root.path().display()
      ),
    );
    Self::executable(&plutil, "#!/bin/sh\nprintf 'com.example.ditto\n'\n");
    Self {
      root,
      transcript,
      xcrun,
      plutil,
      app,
    }
  }

  fn tools(&self) -> SimulatorTools {
    SimulatorTools {
      xcrun: self.xcrun.clone(),
      plutil: self.plutil.clone(),
      command_timeout: Duration::from_secs(10),
      boot_timeout: Duration::from_secs(2),
    }
  }

  fn path(&self, relative: &str) -> PathBuf {
    self.root.path().join(relative)
  }

  fn executable(path: &std::path::Path, contents: &str) {
    fs::write(path, contents).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
  }
}
