use std::{
  env, fs,
  path::{Path, PathBuf},
  process::Command,
  sync::{LazyLock, Mutex},
};

static CURRENT_DIRECTORY: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[test]
fn list_discovers_the_same_suite_from_nested_directories() {
  let fixture = Fixture::new();
  let expected = fixture.list(&fixture.root, &[]);
  assert_eq!(fixture.list(&fixture.nested, &[]), expected);
  assert_eq!(
    fixture.list(&fixture.nested, &["--config", "../../ditto.toml"]),
    expected
  );
  assert!(expected.contains("* macos-local [macos] 1280x720 @ 1.0"));
  assert!(expected.contains("screenshot: connected"));
}

#[test]
fn suite_paths_cannot_escape_the_repository() {
  let fixture = Fixture::new();
  fs::write(
    fixture.root.join("ditto.toml"),
    SUITE.replace("unity_project = \".\"", "unity_project = \"../outside\""),
  )
  .unwrap();
  let error = fixture.list_error(&fixture.root, &[]);
  assert!(error.contains("path escapes repository root"), "{error}");
}

#[test]
fn parser_is_independent_of_the_current_directory_after_loading() {
  let fixture = Fixture::new();
  let _guard = CURRENT_DIRECTORY.lock().unwrap();
  let original = env::current_dir().unwrap();
  env::set_current_dir(&fixture.nested).unwrap();
  let suite = battlement_ditto::run_from(["ditto", "list"]);
  env::set_current_dir(original).unwrap();
  assert!(suite.is_ok());
}

struct Fixture {
  _temporary: tempfile::TempDir,
  root: PathBuf,
  nested: PathBuf,
}

impl Fixture {
  fn new() -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("repo");
    let nested = root.join("nested/deeper");
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir_all(root.join("Assets/Scenes")).unwrap();
    fs::create_dir_all(root.join("rules/src")).unwrap();
    fs::write(root.join("Assets/Scenes/Game.unity"), "").unwrap();
    fs::write(root.join("rules/Cargo.toml"), "[package]\nname='rules'\n").unwrap();
    fs::write(root.join("ditto.toml"), SUITE).unwrap();
    let status = Command::new("git")
      .args(["init", "--quiet"])
      .current_dir(&root)
      .status()
      .unwrap();
    assert!(status.success());
    Self {
      _temporary: temporary,
      root,
      nested,
    }
  }

  fn list(&self, directory: &Path, arguments: &[&str]) -> String {
    let output = self.command(directory, arguments).output().unwrap();
    assert!(
      output.status.success(),
      "{}",
      String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
  }

  fn list_error(&self, directory: &Path, arguments: &[&str]) -> String {
    let output = self.command(directory, arguments).output().unwrap();
    assert!(!output.status.success());
    String::from_utf8(output.stderr).unwrap()
  }

  fn command(&self, directory: &Path, arguments: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ditto"));
    command.arg("list").args(arguments).current_dir(directory);
    command
  }
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
