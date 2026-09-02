use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
  process::{Command, Output},
  time::SystemTime,
};

#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn install_replacement_and_noop_are_complete_and_transactional() {
  let fixture = Fixture::new();
  assert_success(fixture.generate());
  assert_success(fixture.check());
  fixture.assert_no_transaction_artifacts();

  let installed = snapshot(&fixture.project);
  std::thread::sleep(std::time::Duration::from_millis(20));
  assert_success(fixture.generate());
  assert_eq!(snapshot(&fixture.project), installed);

  let stale = fixture.generated_root().join("textures/stale.png");
  fs::write(&stale, b"stale").unwrap();
  fixture.write_declaration(24);
  assert_success(fixture.generate());
  assert!(!stale.exists());
  assert_ne!(snapshot(&fixture.project), installed);
  assert_success(fixture.check());
  fixture.assert_no_transaction_artifacts();
}

#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn ordinary_pre_swap_failures_preserve_the_installed_set() {
  let fixture = Fixture::new();
  assert_success(fixture.generate());
  let installed = snapshot(&fixture.project);

  fs::write(fixture.source(), "this is not Rust").unwrap();
  assert_failure(fixture.generate(), "parse");
  assert_eq!(snapshot(&fixture.project), installed);

  fixture.write_declaration(16);
  let dependency = fixture.project.join("Assets/Textures/panel.png");
  let displaced = fixture.project.join("Assets/Textures/panel.saved");
  fs::rename(&dependency, &displaced).unwrap();
  assert_failure(fixture.generate(), "dependency");
  assert_eq!(snapshot(&fixture.project), installed);
  fs::rename(&displaced, &dependency).unwrap();

  let non_browser = fixture.project.join("not-a-browser");
  fs::write(&non_browser, b"not a browser").unwrap();
  assert_failure(fixture.generate_with_browser(&non_browser), "browser");
  assert_eq!(snapshot(&fixture.project), installed);

  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;

    fixture.write_declaration(28);
    let generated = fixture.project.join("Assets/Generated");
    let permissions = fs::metadata(&generated).unwrap().permissions();
    fs::set_permissions(&generated, fs::Permissions::from_mode(0o555)).unwrap();
    let failed = fixture.generate();
    fs::set_permissions(&generated, permissions).unwrap();
    assert_failure(failed, "permission");
    assert_eq!(snapshot(&fixture.project), installed);
  }
}

#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn startup_recovers_staged_and_backup_roots_without_touching_the_set() {
  let fixture = Fixture::new();
  assert_success(fixture.generate());
  let installed = snapshot(&fixture.project);
  let staged = fixture.transaction_path(".BattlementReactant.staged");
  let backup = fixture.transaction_path(".BattlementReactant.backup");

  fs::rename(fixture.generated_root(), &staged).unwrap();
  assert_success(fixture.generate());
  assert_eq!(snapshot(&fixture.project), installed);
  fixture.assert_no_transaction_artifacts();

  fs::rename(fixture.generated_root(), &backup).unwrap();
  fs::create_dir_all(&staged).unwrap();
  fs::write(staged.join("manifest.json"), b"incomplete\n").unwrap();
  assert_success(fixture.generate());
  assert_eq!(snapshot(&fixture.project), installed);
  fixture.assert_no_transaction_artifacts();

  fs::create_dir_all(&staged).unwrap();
  fs::write(staged.join("partial"), b"partial").unwrap();
  assert_success(fixture.generate());
  assert_eq!(snapshot(&fixture.project), installed);
  fixture.assert_no_transaction_artifacts();
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotEntry {
  bytes: Option<Vec<u8>>,
  modified: SystemTime,
}

fn snapshot(project: &Path) -> BTreeMap<String, SnapshotEntry> {
  let mut entries = BTreeMap::new();
  let root = project.join("Assets/Generated/BattlementReactant");
  capture(project, &root, &mut entries);
  capture(
    project,
    &project.join("Assets/Generated/BattlementReactant.meta"),
    &mut entries,
  );
  entries
}

fn capture(project: &Path, path: &Path, entries: &mut BTreeMap<String, SnapshotEntry>) {
  let metadata = fs::metadata(path).unwrap();
  entries.insert(
    path
      .strip_prefix(project)
      .unwrap()
      .to_string_lossy()
      .into_owned(),
    SnapshotEntry {
      bytes: metadata.is_file().then(|| fs::read(path).unwrap()),
      modified: metadata.modified().unwrap(),
    },
  );
  if metadata.is_dir() {
    let mut children = fs::read_dir(path)
      .unwrap()
      .map(|entry| entry.unwrap().path())
      .collect::<Vec<_>>();
    children.sort();
    for child in children {
      capture(project, &child, entries);
    }
  }
}

fn assert_success(output: Output) {
  assert!(output.status.success(), "{}", stderr(&output));
}

fn assert_failure(output: Output, family: &str) {
  assert!(!output.status.success(), "{family} failure was accepted");
}

struct Fixture {
  _temporary: tempfile::TempDir,
  project: PathBuf,
}

impl Fixture {
  fn new() -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("game");
    for directory in [
      "Assets/Textures",
      "Packages",
      "ProjectSettings",
      "rules/src",
    ] {
      fs::create_dir_all(project.join(directory)).unwrap();
    }
    fs::write(project.join("Packages/manifest.json"), "{}\n").unwrap();
    fs::write(
      project.join("ProjectSettings/ProjectVersion.txt"),
      "m_EditorVersion: fixture\n",
    )
    .unwrap();
    let reactant =
      Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"))
        .parent()
        .unwrap()
        .join("battlement-reactant");
    fs::write(
      project.join("rules/Cargo.toml"),
      format!(
        "[package]\nname = \"transaction-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
        reactant
      ),
    )
    .unwrap();
    fs::copy(
      Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"))
        .join("../../samples/ui/Assets/Original/Signal Texture.png"),
      project.join("Assets/Textures/panel.png"),
    )
    .unwrap();
    let fixture = Self {
      _temporary: temporary,
      project,
    };
    fixture.write_declaration(16);
    fixture
  }

  fn generated_root(&self) -> PathBuf {
    self.project.join("Assets/Generated/BattlementReactant")
  }

  fn source(&self) -> PathBuf {
    self.project.join("rules/src/lib.rs")
  }

  fn transaction_path(&self, name: &str) -> PathBuf {
    self.project.join("Assets/Generated").join(name)
  }

  fn write_declaration(&self, width: u32) {
    fs::write(
      self.source(),
      format!(
        r#"battlement_reactant::asset_generator::generate! {{
          @nine-slice PANEL {{
            @canvas {width}px 12px;
            @slices 2px 2px 2px 2px;
            @allow-clipping top right bottom left;
            background: unity-url("Assets/Textures/panel.png") center / cover;
            box-shadow: inset 1px 1px 1px red;
          }}
        }}"#
      ),
    )
    .unwrap();
  }

  fn generate(&self) -> Output {
    self.run(["reactant", "assets", "generate"])
  }

  fn generate_with_browser(&self, browser: &Path) -> Output {
    self.run([
      "reactant",
      "assets",
      "generate",
      "--browser",
      browser.to_str().unwrap(),
    ])
  }

  fn check(&self) -> Output {
    self.run(["reactant", "assets", "check"])
  }

  fn run<const N: usize>(&self, arguments: [&str; N]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .args(arguments)
      .current_dir(&self.project)
      .output()
      .unwrap()
  }

  fn assert_no_transaction_artifacts(&self) {
    for name in [
      ".BattlementReactant.staged",
      ".BattlementReactant.backup",
      ".BattlementReactant.meta.staged",
      ".BattlementReactant.meta.backup",
    ] {
      assert!(!self.transaction_path(name).exists(), "left {name}");
    }
  }
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}
