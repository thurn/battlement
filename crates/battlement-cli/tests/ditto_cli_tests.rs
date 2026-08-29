use std::{fs, path::Path, process::Command};

#[test]
fn direct_and_cargo_delegated_list_output_is_identical() {
  let temporary = tempfile::tempdir().unwrap();
  let root = temporary.path().join("suite");
  let nested = root.join("nested");
  fs::create_dir_all(root.join("Assets/Scenes")).unwrap();
  fs::create_dir_all(root.join("rules")).unwrap();
  fs::create_dir_all(&nested).unwrap();
  fs::write(root.join("ditto.toml"), SUITE).unwrap();
  assert!(
    Command::new("git")
      .args(["init", "--quiet"])
      .current_dir(&root)
      .status()
      .unwrap()
      .success()
  );

  let direct = direct_ditto(&nested);
  let delegated = Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
    .args(["ditto", "list"])
    .current_dir(&nested)
    .output()
    .unwrap();
  assert!(direct.status.success());
  assert!(delegated.status.success());
  assert_eq!(delegated.stdout, direct.stdout);
  assert_eq!(delegated.stderr, direct.stderr);
}

#[test]
fn direct_and_delegated_parsers_have_the_same_public_matrix() {
  let temporary = tempfile::tempdir().unwrap();
  for arguments in [
    vec!["--help"],
    vec!["run", "--help"],
    vec!["capture", "--help"],
    vec!["fetch", "--help"],
    vec!["doctor", "--help"],
    vec!["clean", "--help"],
    vec!["storage", "--help"],
    vec!["review"],
    vec!["run", "--watch"],
  ] {
    let direct = direct_ditto_with(temporary.path(), &arguments);
    let delegated = Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .arg("ditto")
      .args(&arguments)
      .current_dir(temporary.path())
      .output()
      .unwrap();
    assert_eq!(
      delegated.status.code(),
      direct.status.code(),
      "{arguments:?}"
    );
    assert_eq!(delegated.stdout, direct.stdout, "{arguments:?}");
    assert_eq!(delegated.stderr, direct.stderr, "{arguments:?}");
  }
}

fn direct_ditto(directory: &Path) -> std::process::Output {
  direct_ditto_with(directory, &["list"])
}

fn direct_ditto_with(directory: &Path, arguments: &[&str]) -> std::process::Output {
  let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
  let mut command = Command::new(env!("CARGO"));
  command
    .args([
      "run",
      "--quiet",
      "--manifest-path",
      workspace.join("Cargo.toml").to_str().unwrap(),
      "--package",
      "battlement-ditto",
      "--bin",
      "ditto",
      "--",
    ])
    .args(arguments)
    .current_dir(directory)
    .output()
    .unwrap()
}

const SUITE: &str = r#"name = "minimal"
default_profile = "macos-local"

[player]
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
