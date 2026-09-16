use std::{fs, process::Command};

#[test]
fn explicit_suite_is_forwarded_with_ditto_output_and_exit_status() {
  let temporary = tempfile::tempdir().unwrap();
  let root = temporary.path();
  fs::create_dir_all(root.join("Assets/Scenes")).unwrap();
  fs::create_dir_all(root.join("rules")).unwrap();
  fs::write(root.join("ditto.toml"), SUITE).unwrap();
  assert!(
    Command::new("git")
      .args(["init", "--quiet"])
      .current_dir(root)
      .status()
      .unwrap()
      .success()
  );

  let listed = Command::new(env!("CARGO_BIN_EXE_rt"))
    .args(["ditto", "--config"])
    .arg(root.join("ditto.toml"))
    .arg("list")
    .current_dir(temporary.path())
    .output()
    .unwrap();
  assert!(listed.status.success());
  assert!(
    String::from_utf8(listed.stdout)
      .unwrap()
      .contains("Suite: minimal")
  );

  fs::write(root.join("invalid.toml"), "not a suite").unwrap();
  let invalid = Command::new(env!("CARGO_BIN_EXE_rt"))
    .args(["ditto", "--config"])
    .arg(root.join("invalid.toml"))
    .arg("list")
    .output()
    .unwrap();
  assert_eq!(invalid.status.code(), Some(2));
}

#[test]
fn ditto_help_is_available_without_project_discovery() {
  for arguments in [
    vec!["ditto", "--help"],
    vec!["ditto", "run", "--help"],
    vec!["ditto", "capture", "--help"],
  ] {
    let output = Command::new(env!("CARGO_BIN_EXE_rt"))
      .args(&arguments)
      .output()
      .unwrap();
    assert!(output.status.success(), "{arguments:?}");
  }
}

const SUITE: &str = r#"name = "minimal"
default_profile = "macos-local"

[player]
reactant = false
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[profiles.macos-local]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[[scenarios]]
name = "connected scene"

[[scenarios.steps]]
screenshot = { name = "connected" }
"#;
