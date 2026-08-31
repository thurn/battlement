use std::{
  collections::BTreeMap,
  fs,
  path::{Path, PathBuf},
  process::{Command, Output},
};

use serde_json::Value;

#[test]
fn one_real_browser_context_serves_the_batch_and_warm_cache() {
  let fixture = Fixture::new();
  fixture.write_assets(false);
  let cold_report = fixture.root.join("cold.json");

  let cold = fixture.generate(&cold_report);

  assert!(cold.status.success(), "{}", stderr(&cold));
  let cold_stdout = stdout(&cold);
  assert!(
    [
      "product=Chrome/",
      "product=HeadlessChrome/",
      "product=Chromium/"
    ]
    .iter()
    .any(|product| cold_stdout.contains(product))
  );
  assert!(cold_stdout.contains("protocol="));
  assert!(cold_stdout.contains("session-requests=2"));
  assert_counts(&cold_report, 1, 1, 1);
  let expected = cache_records(&cold);
  assert_eq!(expected.len(), 2);

  let warm_report = fixture.root.join("warm.json");
  let warm = fixture.generate(&warm_report);
  assert!(warm.status.success(), "{}", stderr(&warm));
  assert!(stdout(&warm).contains("session-requests=0"));
  assert_counts(&warm_report, 0, 0, 0);
  assert_eq!(cache_records(&warm), expected);

  fixture.write_assets(true);
  fs::remove_dir_all(
    fixture
      .project
      .join("Library/BattlementReactant/asset-generator-state"),
  )
  .unwrap();
  let reversed_report = fixture.root.join("reversed.json");
  let reversed = fixture.generate(&reversed_report);
  assert!(reversed.status.success(), "{}", stderr(&reversed));
  assert!(stdout(&reversed).contains("session-requests=2"));
  assert_counts(&reversed_report, 1, 1, 1);
  assert_eq!(cache_records(&reversed), expected);
}

#[cfg(unix)]
#[test]
fn explicit_non_chrome_executable_is_rejected_by_the_protocol_contract() {
  let fixture = Fixture::new();
  fixture.write_assets(false);
  let report_path = fixture.root.join("rejected.json");
  let output = fixture.run([
    "reactant",
    "assets",
    "generate",
    "--browser",
    "/bin/sh",
    "--work-report",
    report_path.to_str().unwrap(),
  ]);

  assert!(!output.status.success());
  let diagnostic = stderr(&output);
  assert!(diagnostic.contains("explicit browser"), "{diagnostic}");
  assert!(
    diagnostic.contains("Chrome or Chromium debugging endpoint"),
    "{diagnostic}"
  );
  assert_counts(&report_path, 1, 0, 0);
}

#[test]
fn missing_explicit_browser_fails_without_starting_a_renderer() {
  let fixture = Fixture::new();
  fixture.write_assets(false);
  let report_path = fixture.root.join("missing.json");
  let output = fixture.run([
    "reactant",
    "assets",
    "generate",
    "--browser",
    "missing-browser",
    "--work-report",
    report_path.to_str().unwrap(),
  ]);

  assert!(!output.status.success());
  let diagnostic = stderr(&output);
  assert!(diagnostic.contains("explicit browser"), "{diagnostic}");
  assert!(diagnostic.contains("is not executable"), "{diagnostic}");
  assert_counts(&report_path, 0, 0, 0);
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
    fs::create_dir_all(project.join("rules/src")).unwrap();
    fs::write(project.join("Packages/manifest.json"), "{}\n").unwrap();
    fs::write(
      project.join("ProjectSettings/ProjectVersion.txt"),
      "m_EditorVersion: fixture\n",
    )
    .unwrap();
    let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
      .parent()
      .unwrap()
      .join("battlement-reactant");
    fs::write(
      project.join("rules/Cargo.toml"),
      format!(
        "[package]\nname = \"browser-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\
         [dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
        reactant
      ),
    )
    .unwrap();
    Self {
      _temporary: temporary,
      root,
      project,
    }
  }

  fn write_assets(&self, reversed: bool) {
    let declarations = [
      "@background FIRST { @canvas 8px 8px; background: linear-gradient(red, blue); }",
      "@background SECOND { @canvas 7px 6px; background: linear-gradient(blue, red); }",
    ];
    let ordered = if reversed {
      [declarations[1], declarations[0]]
    } else {
      declarations
    };
    fs::write(
      self.project.join("rules/src/lib.rs"),
      format!(
        "battlement_reactant::asset_generator::generate! {{\n  {}\n}}\n\
         battlement_reactant::asset_generator::generate! {{\n  {}\n}}\n",
        ordered[0], ordered[1]
      ),
    )
    .unwrap();
  }

  fn generate(&self, report: &Path) -> Output {
    self.run([
      "reactant",
      "assets",
      "generate",
      "--work-report",
      report.to_str().unwrap(),
    ])
  }

  fn run<I, S>(&self, arguments: I) -> Output
  where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
  {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .args(arguments)
      .current_dir(&self.project)
      .output()
      .unwrap()
  }
}

fn assert_counts(path: &Path, launches: u64, contexts: u64, executable_opens: u64) {
  let report: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
  assert_eq!(report["browserLaunches"], launches);
  assert_eq!(report["browserContextsCreated"], contexts);
  assert_eq!(report["browserExecutableOpens"], executable_opens);
}

fn cache_records(output: &Output) -> BTreeMap<String, (String, String)> {
  stdout(output)
    .lines()
    .filter_map(|line| line.strip_prefix("cache="))
    .map(|line| {
      let mut fields = line.split_whitespace();
      let address = fields.next().unwrap().to_owned();
      let key = fields
        .next()
        .unwrap()
        .strip_prefix("key=")
        .unwrap()
        .to_owned();
      let probe = fields
        .next()
        .unwrap()
        .strip_prefix("probe=")
        .unwrap()
        .to_owned();
      (address, (key, probe))
    })
    .collect()
}

fn stdout(output: &Output) -> String {
  String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}
