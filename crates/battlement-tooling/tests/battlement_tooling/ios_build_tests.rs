#![cfg(target_os = "macos")]

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

use battlement_tooling::{
  build_cache::{BUILD_LOG_FILE, BuildCache, BuildHandle, SOURCE_MANIFEST_FILE},
  build_control::BuildControl,
  build_identity::CaptureAdapter,
  ios_build::{
    IosBuildOutcome, IosBuildRequest, IosBuildResult, IosBuildTools, STARTUP_IDENTITY_FILE,
    ios_startup_identity, player_app, select_ios_player,
  },
  ios_device::{self, IosDeviceRequest, IosDeviceTools},
  ios_target::{IosSigning, IosTarget},
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[test]
fn clean_fixture_builds_fixed_app_and_exactly_reuses_it() {
  let fixture = Fixture::new();
  let result = select_ios_player(&fixture.request(), true, BuildControl::default()).unwrap();
  if let IosBuildResult::Failed(failure) = &result {
    panic!(
      "fixture build failed: {}",
      fs::read_to_string(&failure.log_path).unwrap()
    );
  }
  let IosBuildResult::Ready { build, outcome } = result else {
    panic!("fixture unexpectedly required a build")
  };
  assert_eq!(outcome, IosBuildOutcome::Created);
  assert!(player_app(&build).unwrap().join("Info.plist").is_file());
  assert!(build.path().join(BUILD_LOG_FILE).is_file());
  assert!(build.path().join(SOURCE_MANIFEST_FILE).is_file());
  assert!(build.path().join(STARTUP_IDENTITY_FILE).is_file());
  let startup = ios_startup_identity(&build).unwrap();
  assert_eq!(startup.platform, "ios-simulator");
  assert!(startup.diagnostics);
  drop(build);

  let IosBuildResult::Ready { outcome, .. } =
    select_ios_player(&fixture.request(), true, BuildControl::default()).unwrap()
  else {
    panic!("exact reuse failed")
  };
  assert_eq!(outcome, IosBuildOutcome::Reused);
  let transcript = fs::read_to_string(&fixture.transcript).unwrap();
  assert_eq!(transcript.matches("cargo\n").count(), 1);
  assert_eq!(transcript.matches("unity\n").count(), 1);
  assert_eq!(transcript.matches("xcodebuild\n").count(), 1);
  assert!(transcript.contains("ARCHS=arm64\n"));
  assert!(transcript.contains("ONLY_ACTIVE_ARCH=YES\n"));
  assert!(
    !fixture
      .path("repo/game/Assets/Plugins/iOS/libbattlement_rules.a")
      .exists()
  );
  assert!(
    !fixture
      .path("repo/game/Assets/Resources/BattlementDittoBuildIdentity.json")
      .exists()
  );
}

#[test]
fn no_build_reports_required_and_rust_failure_is_terminal() {
  let fixture = Fixture::new();
  assert!(matches!(
    select_ios_player(&fixture.request(), false, BuildControl::default()).unwrap(),
    IosBuildResult::Required { .. }
  ));
  fs::write(
    fixture.path("repo/rules/src/lib.rs"),
    "COMPILATION_FAILURE\n",
  )
  .unwrap();
  let IosBuildResult::Failed(failure) =
    select_ios_player(&fixture.request(), true, BuildControl::default()).unwrap()
  else {
    panic!("broken source unexpectedly built")
  };
  assert_eq!(failure.phase, "rust");
  assert_eq!(failure.error_ids, ["E0308"]);
  assert!(
    fs::read_to_string(failure.log_path)
      .unwrap()
      .contains("error[E0308]")
  );
}

#[test]
fn device_builds_use_device_sdk_and_do_not_reuse_simulator_or_unsigned_signatures() {
  let fixture = Fixture::new();
  let simulator =
    match select_ios_player(&fixture.request(), true, BuildControl::default()).unwrap() {
      IosBuildResult::Ready { build, .. } => build,
      other => panic!("unexpected result: {other:?}"),
    };
  let mut request = fixture.request();
  request.target = IosTarget::Device {
    bundle_identifier: "dev.example.game".to_owned(),
    signing: None,
  };
  let unsigned = match select_ios_player(&request, true, BuildControl::default()).unwrap() {
    IosBuildResult::Ready {
      build,
      outcome: IosBuildOutcome::Created,
    } => build,
    other => panic!("unexpected result: {other:?}"),
  };
  assert_ne!(
    simulator.metadata().identity.fingerprint,
    unsigned.metadata().identity.fingerprint
  );
  assert_eq!(
    ios_startup_identity(&unsigned).unwrap().platform,
    "ios-device"
  );
  let transcript = fs::read_to_string(&fixture.transcript).unwrap();
  assert!(transcript.contains("aarch64-apple-ios\n"));
  assert!(transcript.contains("iphoneos\n"));
  assert!(transcript.contains("CODE_SIGNING_ALLOWED=NO"));
  request.target = IosTarget::Device {
    bundle_identifier: "dev.example.game".to_owned(),
    signing: Some(IosSigning {
      team: "SUPPLIED_TEAM".to_owned(),
      identity: "Supplied Development Identity".to_owned(),
      profile_uuid: "SUPPLIED_PROFILE".to_owned(),
      profile_fingerprint: Sha256::digest(b"profile")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect(),
    }),
  };
  let signed = match select_ios_player(&request, true, BuildControl::default()).unwrap() {
    IosBuildResult::Ready {
      build,
      outcome: IosBuildOutcome::Created,
    } => build,
    other => panic!("unexpected result: {other:?}"),
  };
  let mut other_app = request.clone();
  if let IosTarget::Device {
    bundle_identifier, ..
  } = &mut other_app.target
  {
    *bundle_identifier = "dev.example.other".to_owned();
  }
  assert!(matches!(
    select_ios_player(&other_app, false, BuildControl::default()).unwrap(),
    IosBuildResult::Required { .. }
  ));
  assert_ne!(
    unsigned.metadata().identity.fingerprint,
    signed.metadata().identity.fingerprint
  );
  let transcript = fs::read_to_string(&fixture.transcript).unwrap();
  assert!(transcript.contains("CODE_SIGN_STYLE=Manual"));
  assert!(transcript.contains("profile=SUPPLIED_PROFILE"));
  assert!(!transcript.contains("allowProvisioningUpdates"));
  assert!(transcript.contains("bundle=dev.example.game"));
  self::device_delivery(&fixture, &simulator, &unsigned, &signed);
}

fn device_delivery(
  fixture: &Fixture,
  simulator: &BuildHandle,
  unsigned: &BuildHandle,
  signed: &BuildHandle,
) {
  fixture.executable("tools/codesign", "#!/bin/sh\nexit 0\n");
  fixture.executable(
    "tools/devicectl",
    r#"#!/bin/sh
set -eu
while [ "$#" -gt 0 ]; do
  case "$1" in
    --json-output) result="$2"; shift 2 ;;
    --environment-variables) environment="$2"; shift 2 ;;
    *) shift ;;
  esac
done
printf '{"info":{"outcome":"success"},"result":{"process":{"processIdentifier":123}}}' > "$result"
printf '%s' "${environment:-}" > "$result.environment"
"#,
  );
  let tools = IosDeviceTools {
    xcrun: fixture.path("tools/devicectl"),
    codesign: fixture.path("tools/codesign"),
    plutil: PathBuf::from("/usr/bin/plutil"),
  };
  let request = IosDeviceRequest {
    device: "explicit-device".to_owned(),
    evidence: fixture.path("delivery"),
    launch: true,
    fixture: Some("motion".to_owned()),
  };
  for invalid in [simulator, unsigned] {
    assert!(ios_device::install(invalid, &tools, &request, BuildControl::default()).is_err());
  }
  assert!(!request.evidence.exists());
  let receipt = ios_device::install(signed, &tools, &request, BuildControl::default()).unwrap();
  assert_eq!(
    receipt.build_fingerprint,
    signed.metadata().identity.fingerprint
  );
  assert_eq!(receipt.bundle_identifier, "dev.example.game");
  assert!(receipt.launch.is_some());
  assert!(request.evidence.join("receipt.json").is_file());
  assert_eq!(
    serde_json::from_slice::<serde_json::Value>(
      &fs::read(request.evidence.join("launch.json.environment")).unwrap()
    )
    .unwrap(),
    serde_json::json!({"BATTLEMENT_DITTO_SEMANTIC_FIXTURE":"motion"})
  );
  assert!(ios_device::install(signed, &tools, &request, BuildControl::default()).is_err());
  fixture.executable("tools/codesign", "#!/bin/sh\nexit 1\n");
  let invalid_signature = IosDeviceRequest {
    evidence: fixture.path("invalid-signature"),
    ..request
  };
  assert!(
    ios_device::install(signed, &tools, &invalid_signature, BuildControl::default()).is_err()
  );
  assert!(!invalid_signature.evidence.exists());
}

struct Fixture {
  root: TempDir,
  transcript: PathBuf,
  cargo: PathBuf,
  unity: PathBuf,
  xcodebuild: PathBuf,
}

impl Fixture {
  fn new() -> Self {
    let root = TempDir::new().unwrap();
    let fixture = Self {
      transcript: root.path().join("transcript"),
      cargo: root.path().join("tools/cargo"),
      unity: root.path().join("tools/Unity"),
      xcodebuild: root.path().join("tools/xcodebuild"),
      root,
    };
    fixture.write("transcript", "");
    fixture.write("repo/Cargo.lock", "version = 4\n");
    fixture.write("repo/game/Assets/Scenes/Game.unity", "scene\n");
    fixture.write(
      "repo/game/Packages/manifest.json",
      r#"{"dependencies":{"com.battlement.client":"file:../../package"}}"#,
    );
    fixture.write("repo/game/Packages/packages-lock.json", "{}\n");
    fixture.write(
      "repo/game/ProjectSettings/ProjectVersion.txt",
      "m_EditorVersion: 6000.5.8f1\n",
    );
    fixture.write(
      "repo/game/ProjectSettings/ProjectSettings.asset",
      "settings\n",
    );
    fixture.write(
      "repo/package/package.json",
      "{\"name\":\"com.battlement.client\"}\n",
    );
    fixture.write("repo/package/Runtime/Player.cs", "public class Player {}\n");
    fixture.write(
      "repo/rules/Cargo.toml",
      "[package]\nname='rules'\nversion='0.1.0'\n[lib]\nname='battlement_rules'\n[workspace]\n",
    );
    fixture.write("repo/rules/src/lib.rs", "pub fn rules() {}\n");
    fixture.executable("tools/cargo", &fixture.cargo_script());
    fixture.executable("tools/Unity", &fixture.unity_script());
    fixture.executable("tools/xcodebuild", &fixture.xcode_script());
    fixture.write(
      "repo/scripts/unity_transaction.py",
      include_str!("../../../../scripts/unity_transaction.py"),
    );
    fixture.write(
      "repo/scripts/process_priority.py",
      include_str!("../../../../scripts/process_priority.py"),
    );
    fixture.write(
      "repo/scripts/unity_metadata.py",
      include_str!("../../../../scripts/unity_metadata.py"),
    );
    fixture.initialize_repository();
    fixture
  }

  fn initialize_repository(&self) {
    Command::new("git")
      .args(["init", "--quiet"])
      .current_dir(self.path("repo"))
      .status()
      .unwrap();
    Command::new("git")
      .args(["add", "."])
      .current_dir(self.path("repo"))
      .status()
      .unwrap();
    Command::new("git")
      .args([
        "-c",
        "user.name=CI Fixture",
        "-c",
        "user.email=ci@example.invalid",
        "commit",
        "--quiet",
        "-m",
        "fixture",
      ])
      .current_dir(self.path("repo"))
      .status()
      .unwrap();
  }

  fn request(&self) -> IosBuildRequest {
    IosBuildRequest {
      target: IosTarget::Simulator,
      repository: self.path("repo"),
      unity_project: self.path("repo/game"),
      rust_manifest: self.path("repo/rules/Cargo.toml"),
      scene: self.path("repo/game/Assets/Scenes/Game.unity"),
      suite: "fixture".to_owned(),
      diagnostics: true,
      generated_inputs: Vec::new(),
      native_inputs: Vec::new(),
      capture_adapter: CaptureAdapter {
        name: "native-screen-capture".to_owned(),
        version: "1".to_owned(),
      },
      tools: IosBuildTools {
        unity_editor: self.unity.clone(),
        unity_version: "6000.5.8f1".to_owned(),
        cargo: self.cargo.clone(),
        cargo_version: "cargo 1.92.0".to_owned(),
        rustc_version: "rustc 1.92.0".to_owned(),
        architecture: "arm64".to_owned(),
        xcodebuild: self.xcodebuild.clone(),
        xcode_version: "Xcode 26.4".to_owned(),
        sdk_version: "26.4".to_owned(),
      },
      resource_slots: self.path("slots"),
      cache: BuildCache::open(self.path("cache"), 1024 * 1024).unwrap(),
    }
  }

  fn cargo_script(&self) -> String {
    format!(
      r#"#!/bin/sh
set -eu
printf 'cargo\n' >> '{}'
manifest=''; target=''; target_dir=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --manifest-path) manifest="$2"; shift 2 ;;
    --target) target="$2"; shift 2 ;;
    --target-dir) target_dir="$2"; shift 2 ;;
    *) shift ;;
  esac
done
if grep -q COMPILATION_FAILURE "$(dirname "$manifest")/src/lib.rs"; then printf 'error[E0308]: failed\n' >&2; exit 1; fi
printf '%s\n' "$target" >> '{}'
mkdir -p "$target_dir/$target/release"
printf 'archive' > "$target_dir/$target/release/libbattlement_rules.a"
"#,
      self.transcript.display(),
      self.transcript.display()
    )
  }

  fn unity_script(&self) -> String {
    format!(
      r#"#!/bin/sh
set -eu
printf 'unity\n' >> '{}'
project=''; method=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -projectPath) project="$2"; shift 2 ;;
    -executeMethod) method="$2"; shift 2 ;;
    -logFile) log="$2"; shift 2 ;;
    *) shift ;;
  esac
done
case "$method" in
  Battlement.Editor.BattlementDittoBuild.BuildIosSimulator|Battlement.Editor.BattlementDittoBuild.BuildIosDevice) ;;
  *) exit 1 ;;
esac
[ "$BATTLEMENT_DITTO_IOS_SIMULATOR_ARCHITECTURE" = 'arm64' ]
printf 'profile=%s bundle=%s\n' "$BATTLEMENT_IOS_PROVISIONING_PROFILE" "$BATTLEMENT_IOS_BUNDLE_IDENTIFIER" >> '{}'
[ -f "$project/Assets/Plugins/iOS/libbattlement_rules.a" ]
[ -f "$project/Assets/Resources/BattlementDittoBuildIdentity.json" ]
mkdir -p "$BATTLEMENT_DITTO_BUILD_PATH/Unity-iPhone.xcodeproj"
printf '%s' "$BATTLEMENT_IOS_BUNDLE_IDENTIFIER" > "$BATTLEMENT_DITTO_BUILD_PATH/bundle-identifier"
printf 'unity log\n' > "$log"
"#,
      self.transcript.display(),
      self.transcript.display()
    )
  }

  fn xcode_script(&self) -> String {
    format!(
      r#"#!/bin/sh
set -eu
printf 'xcodebuild\n' >> '{}'
printf '%s\n' "$@" >> '{}'
products=''; sdk=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -sdk) sdk="$2"; shift 2 ;;
    SYMROOT=*) products="${{1#SYMROOT=}}"; shift ;;
    *) shift ;;
  esac
done
mkdir -p "$products/Release-$sdk/Fixture.app"
bundle=$(cat bundle-identifier)
printf '<plist version="1.0"><dict><key>CFBundleIdentifier</key><string>%s</string></dict></plist>' "$bundle" > "$products/Release-$sdk/Fixture.app/Info.plist"
printf 'profile' > "$products/Release-$sdk/Fixture.app/embedded.mobileprovision"
"#,
      self.transcript.display(),
      self.transcript.display()
    )
  }

  fn path(&self, relative: &str) -> PathBuf {
    self.root.path().join(relative)
  }

  fn write(&self, relative: &str, contents: &str) {
    let path = self.path(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
  }

  fn executable(&self, relative: &str, contents: &str) {
    self.write(relative, contents);
    let path = self.path(relative);
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
  }
}
