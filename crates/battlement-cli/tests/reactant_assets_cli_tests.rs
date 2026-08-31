use std::{
  fs,
  path::{Path, PathBuf},
  process::{Command, Output},
};

use serde_json::Value;

#[test]
fn empty_commands_resolve_project_and_remove_only_generated_output() {
  let fixture = Fixture::new();
  fixture.write_generated_output();
  let report = fixture.root.join("generate-report.json");

  let generated = fixture.run_from(
    &fixture.root,
    [
      "reactant",
      "assets",
      "generate",
      "--project",
      "game",
      "--work-report",
      report.to_str().unwrap(),
    ],
  );

  assert!(generated.status.success(), "{}", stderr(&generated));
  assert!(stdout(&generated).contains("browser not started"));
  assert!(!fixture.generated_root().exists());
  assert!(!fixture.generated_meta().exists());
  let report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
  assert_eq!(report["cargoMetadataRuns"], 2);
  assert_eq!(report["browserLaunches"], 0);
  assert_eq!(report["browserContextsCreated"], 0);
  assert_eq!(report["filesWritten"], 2);

  let nested = fixture.project.join("Assets/Nested/Deeper");
  fs::create_dir_all(&nested).unwrap();
  let checked = fixture.run_from(&nested, ["reactant", "assets", "check"]);
  assert!(checked.status.success(), "{}", stderr(&checked));
}

#[test]
fn check_is_read_only_and_reports_stale_empty_output() {
  let fixture = Fixture::new();
  fixture.write_generated_output();
  let manifest = fixture.generated_root().join("manifest.json");
  let before = fs::read(&manifest).unwrap();
  let report = fixture.root.join("check-report.json");

  let checked = fixture.run_from(
    &fixture.project,
    [
      "reactant",
      "assets",
      "check",
      "--work-report",
      report.to_str().unwrap(),
    ],
  );

  assert!(!checked.status.success());
  assert!(stderr(&checked).contains("assets are stale"));
  assert_eq!(fs::read(manifest).unwrap(), before);
  assert_eq!(
    serde_json::from_slice::<Value>(&fs::read(report).unwrap()).unwrap()["filesWritten"],
    0
  );
}

#[test]
fn selections_reject_non_projects_and_escaped_rules_manifests() {
  let fixture = Fixture::new();
  let outside = fixture.root.join("outside");
  write_rules(&outside);
  let escaped = fixture.run_from(
    &fixture.project,
    [
      "reactant",
      "assets",
      "generate",
      "--manifest-path",
      outside.join("Cargo.toml").to_str().unwrap(),
    ],
  );
  assert!(!escaped.status.success());
  assert!(stderr(&escaped).contains("must be contained by Unity project"));

  let not_project = fixture.run_from(
    &fixture.root,
    ["reactant", "assets", "generate", "--project", "outside"],
  );
  assert!(!not_project.status.success());
  assert!(stderr(&not_project).contains("is not a Unity project"));
}

#[test]
fn asset_command_help_exposes_the_shared_selection_contract() {
  let output = Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
    .args(["reactant", "assets", "generate", "--help"])
    .output()
    .unwrap();
  assert!(output.status.success(), "{}", stderr(&output));
  let help = stdout(&output);
  for option in [
    "--project",
    "--manifest-path",
    "--features",
    "--all-features",
    "--no-default-features",
    "--browser",
    "--work-report",
  ] {
    assert!(help.contains(option), "missing {option} in:\n{help}");
  }
}

#[cfg(target_os = "macos")]
#[test]
fn empty_preview_uses_the_system_opener_without_a_renderer() {
  let fixture = Fixture::new();
  let report = fixture.root.join("preview-report.json");
  let previewed = fixture.run_from(
    &fixture.project,
    [
      "reactant",
      "assets",
      "preview",
      "--work-report",
      report.to_str().unwrap(),
    ],
  );

  assert!(previewed.status.success(), "{}", stderr(&previewed));
  let report: Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
  assert_eq!(report["browserLaunches"], 0);
  assert_eq!(report["browserContextsCreated"], 0);
  assert_eq!(report["subprocessesStarted"], 4);
}

#[test]
fn declarations_are_discovered_across_modules_and_reachable_packages() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  fs::create_dir_all(fixture.project.join("rules/src/nested")).unwrap();
  fs::create_dir_all(fixture.project.join("rules/asset-pack/src")).unwrap();
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [features]\ndefault = []\nart = [\"dep:asset-pack\"]\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\nasset-pack = {{ path = \"asset-pack\", optional = true }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/src/lib.rs"),
    "mod nested;\npub fn rules() {}\n",
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/src/nested/mod.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @background PANEL { @canvas 20px 10px; background: linear-gradient(red, blue); }\n\
     }\n",
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/asset-pack/Cargo.toml"),
    format!(
      "[package]\nname = \"asset-pack\"\nversion = \"0.2.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/asset-pack/src/lib.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @nine-slice FRAME { @canvas 30px 18px; @slices 2px 2px 2px 2px; border: 1px dashed red; }\n\
     }\n",
  )
  .unwrap();

  let output = fixture.run_from(
    &fixture.project,
    ["reactant", "assets", "generate", "--features", "art"],
  );

  assert!(output.status.success(), "{}", stderr(&output));
  assert!(stdout(&output).contains("discovered=2 deduplicated=2"));
}

#[test]
fn git_dependency_declarations_are_discovered_with_portable_coordinates() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  let git_assets = fixture.root.join("git-assets");
  fs::create_dir_all(git_assets.join("src")).unwrap();
  fs::write(
    git_assets.join("Cargo.toml"),
    format!(
      "[package]\nname = \"git-assets\"\nversion = \"0.3.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    git_assets.join("src/lib.rs"),
    "battlement_reactant::asset_generator::generate! {\n\
       @background GIT_PANEL { @canvas 12px 8px; background: linear-gradient(red, blue); }\n\
     }\n",
  )
  .unwrap();
  for arguments in [
    vec!["init", "-q"],
    vec!["add", "."],
    vec![
      "-c",
      "user.name=Fixture",
      "-c",
      "user.email=fixture@example.invalid",
      "commit",
      "-qm",
      "fixture",
    ],
  ] {
    assert!(
      Command::new("git")
        .args(arguments)
        .current_dir(&git_assets)
        .status()
        .unwrap()
        .success()
    );
  }
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\ngit-assets = {{ git = {:?} }}\n",
      format!("file://{}", git_assets.display())
    ),
  )
  .unwrap();

  let output = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);

  assert!(output.status.success(), "{}", stderr(&output));
  assert!(stdout(&output).contains("discovered=1 deduplicated=1"));
}

#[test]
fn discovery_rejects_indirection_conditionals_and_target_graph_drift() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  let cases = [
    (
      "use battlement_reactant::asset_generator;\nasset_generator::generate! { anything }\n",
      "imports or reexports",
    ),
    (
      "fn nested() { battlement_reactant::asset_generator::generate! { anything } }\n",
      "is nested",
    ),
    (
      "#[cfg(any())]\nbattlement_reactant::asset_generator::generate! { anything }\n",
      "conditionally compiled",
    ),
    (
      "macro_rules! wrapped { () => { battlement_reactant::asset_generator::generate! { anything } } }\n",
      "macro wrapper",
    ),
  ];
  for (source, diagnostic) in cases {
    fs::write(fixture.project.join("rules/src/lib.rs"), source).unwrap();
    let output = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains(diagnostic), "{}", stderr(&output));
  }

  fs::create_dir_all(fixture.project.join("rules/host-art/src")).unwrap();
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n\
       [target.'cfg(not(target_arch = \"wasm32\"))'.dependencies]\nhost-art = {{ path = \"host-art\" }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/src/lib.rs"),
    "pub fn rules() {}\n",
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/host-art/Cargo.toml"),
    format!(
      "[package]\nname = \"host-art\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(
    fixture.project.join("rules/host-art/src/lib.rs"),
    "pub fn art() {}\n",
  )
  .unwrap();
  let output = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!output.status.success());
  assert!(stderr(&output).contains("reachable declaration packages differ"));
  assert!(stderr(&output).contains("host="));
  assert!(stderr(&output).contains("WebAssembly="));
}

#[test]
fn discovery_rejects_renamed_reactant_and_nonportable_path_packages() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nreactant = {{ package = \"battlement-reactant\", path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  let renamed = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!renamed.status.success());
  assert!(stderr(&renamed).contains("aliases battlement-reactant as reactant"));

  let outside = fixture.root.join("outside-assets");
  fs::create_dir_all(outside.join("src")).unwrap();
  fs::write(
    outside.join("Cargo.toml"),
    format!(
      "[package]\nname = \"outside-assets\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  fs::write(outside.join("src/lib.rs"), "pub fn outside() {}\n").unwrap();
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
     [dependencies]\noutside-assets = { path = \"../../outside-assets\" }\n",
  )
  .unwrap();
  let nonportable = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
  assert!(!nonportable.status.success());
  assert!(stderr(&nonportable).contains("outside Unity project"));
  assert!(stderr(&nonportable).contains("no portable coordinate"));
}

#[test]
fn cli_discovery_preserves_shared_syntax_diagnostic_categories() {
  let fixture = Fixture::new();
  let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .unwrap()
    .join("battlement-reactant");
  fs::write(
    fixture.project.join("rules/Cargo.toml"),
    format!(
      "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
       [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
      reactant
    ),
  )
  .unwrap();
  for (declaration, category) in [
    (
      "@background PANEL { @canvas 10px 10px; @canvas 20px 20px; background: linear-gradient(red, blue); }",
      "duplicate-statement",
    ),
    (
      "@background PANEL { @canvas 0px 10px; background: linear-gradient(red, blue); }",
      "invalid-geometry",
    ),
    (
      "@background PANEL { @canvas 10px 10px; background: red; }",
      "native-only",
    ),
  ] {
    fs::write(
      fixture.project.join("rules/src/lib.rs"),
      format!("battlement_reactant::asset_generator::generate! {{ {declaration} }}\n"),
    )
    .unwrap();
    let output = fixture.run_from(&fixture.project, ["reactant", "assets", "check"]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains(category), "{}", stderr(&output));
  }
}

struct Fixture {
  _temporary: tempfile::TempDir,
  root: PathBuf,
  project: PathBuf,
}

impl Fixture {
  fn new() -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_owned();
    let project = root.join("game");
    fs::create_dir_all(project.join("Assets")).unwrap();
    fs::create_dir_all(project.join("Packages")).unwrap();
    fs::create_dir_all(project.join("ProjectSettings")).unwrap();
    fs::write(project.join("Packages/manifest.json"), "{}\n").unwrap();
    fs::write(
      project.join("ProjectSettings/ProjectVersion.txt"),
      "m_EditorVersion: fixture\n",
    )
    .unwrap();
    write_rules(&project.join("rules"));
    Self {
      _temporary: temporary,
      root,
      project,
    }
  }

  fn run_from<I, S>(&self, current: &Path, arguments: I) -> Output
  where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
  {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .args(arguments)
      .current_dir(current)
      .output()
      .unwrap()
  }

  fn generated_root(&self) -> PathBuf {
    self.project.join("Assets/Generated/BattlementReactant")
  }

  fn generated_meta(&self) -> PathBuf {
    self
      .project
      .join("Assets/Generated/BattlementReactant.meta")
  }

  fn write_generated_output(&self) {
    fs::create_dir_all(self.generated_root().join("Resources")).unwrap();
    fs::write(self.generated_root().join("manifest.json"), "manifest\n").unwrap();
    fs::write(
      self
        .generated_root()
        .join("Resources/BattlementReactantAssetCatalog.json"),
      "catalog\n",
    )
    .unwrap();
    fs::write(self.generated_meta(), "metadata\n").unwrap();
  }
}

fn write_rules(directory: &Path) {
  fs::create_dir_all(directory.join("src")).unwrap();
  fs::write(
    directory.join("Cargo.toml"),
    "[package]\nname = \"fixture-rules\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
  )
  .unwrap();
  fs::write(directory.join("src/lib.rs"), "pub fn empty() {}\n").unwrap();
}

fn stdout(output: &Output) -> String {
  String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}
