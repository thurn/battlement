#![cfg(target_os = "macos")]

use std::{
  env, fs,
  path::{Path, PathBuf},
  process::Command,
};

const INFO_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>BattlementFixture</string>
    <key>CFBundleIdentifier</key>
    <string>dev.battlement.cli-fixture</string>
</dict>
</plist>
"#;

#[test]
fn install_and_restore_keep_the_application_validly_signed() {
  let temporary = tempfile::tempdir().unwrap();
  let app = temporary.path().join("Battlement Fixture.app");
  let plugin = build_fixture(temporary.path());
  create_app(&app, &plugin);
  codesign(&["--force", "--deep", "--sign", "-"], &app);

  cargo_battlement(&["plugin", "install"], &[&app, &plugin]);
  codesign(&["--verify", "--deep", "--strict"], &app);
  assert!(backup(&app).is_file());

  cargo_battlement(&["plugin", "restore"], &[&app]);
  codesign(&["--verify", "--deep", "--strict"], &app);
  assert!(!backup(&app).exists());
}

fn build_fixture(temporary: &Path) -> PathBuf {
  let workspace =
    Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"))
      .join("../..");
  let target = temporary.join("fixture-target");
  let status = Command::new(env!("CARGO"))
    .args([
      "build",
      "--quiet",
      "--package",
      "battlement-native-export-fixture",
      "--target-dir",
    ])
    .arg(&target)
    .current_dir(workspace)
    .status()
    .unwrap();
  assert!(status.success());
  target.join("debug/libbattlement_rules.dylib")
}

fn create_app(app: &Path, plugin: &Path) {
  let contents = app.join("Contents");
  fs::create_dir_all(contents.join("MacOS")).unwrap();
  fs::create_dir_all(contents.join("PlugIns")).unwrap();
  fs::write(contents.join("Info.plist"), INFO_PLIST).unwrap();
  fs::copy("/usr/bin/true", contents.join("MacOS/BattlementFixture")).unwrap();
  fs::copy(plugin, contents.join("PlugIns/libbattlement_rules.dylib")).unwrap();
}

fn cargo_battlement(arguments: &[&str], paths: &[&Path]) {
  let status = Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
    .args(arguments)
    .args(paths)
    .status()
    .unwrap();
  assert!(status.success());
}

fn codesign(arguments: &[&str], path: &Path) {
  let status = Command::new("codesign")
    .args(arguments)
    .arg(path)
    .status()
    .unwrap();
  assert!(status.success());
}

fn backup(app: &Path) -> PathBuf {
  PathBuf::from(format!("{}.battlement-backup", app.display())).join("libbattlement_rules.dylib")
}
