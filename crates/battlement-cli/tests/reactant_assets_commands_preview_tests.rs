use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
  process::{Command, Output},
  time::SystemTime,
};

use serde_json::Value;

#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn commands_classify_without_repair_and_render_only_changed_requests() {
  let fixture = Fixture::new();
  let generated = fixture.run("generate", "cold");
  assert_success(&generated);
  assert_counts(&generated, 3, 2, 0, 2, 2);
  assert_eq!(stdout(&generated).matches("status=added").count(), 2);

  let installed = snapshot(&fixture.generated_root());
  let checked = fixture.run("check", "current");
  assert_success(&checked);
  assert_counts(&checked, 3, 2, 2, 0, 0);
  assert!(stdout(&checked).contains("browser not started"));
  assert_eq!(snapshot(&fixture.generated_root()), installed);
  assert_report_read_only(fixture.report("current"));

  let dependency = fixture.project.join("Assets/Textures/panel.png");
  let original_dependency = fs::read(&dependency).unwrap();
  fs::copy(fixture.asset("Rocket Emoji.png"), &dependency).unwrap();
  let changed = fixture.run("check", "changed");
  assert_failure(&changed);
  assert_eq!(stdout(&changed).matches("status=changed").count(), 1);
  assert_counts(&changed, 3, 2, 1, 0, 1);
  assert_eq!(snapshot(&fixture.generated_root()), installed);
  assert_report_read_only(fixture.report("changed"));
  fs::write(&dependency, &original_dependency).unwrap();

  let manifest_path = fixture.generated_root().join("manifest.json");
  let manifest_bytes = fs::read(&manifest_path).unwrap();
  let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
  let png = fixture
    .generated_root()
    .join(manifest["assets"][0]["png"].as_str().unwrap());
  let png_bytes = fs::read(&png).unwrap();

  fs::remove_file(&png).unwrap();
  let missing_tree = snapshot(&fixture.generated_root());
  let missing = fixture.run("check", "missing");
  assert_failure(&missing);
  assert!(stdout(&missing).contains("status=missing"));
  assert_eq!(snapshot(&fixture.generated_root()), missing_tree);
  fs::write(&png, &png_bytes).unwrap();

  fs::write(&png, b"not a PNG").unwrap();
  let corrupt_tree = snapshot(&fixture.generated_root());
  let corrupt = fixture.run("check", "corrupt");
  assert_failure(&corrupt);
  assert!(stdout(&corrupt).contains("status=corrupt"));
  assert_eq!(snapshot(&fixture.generated_root()), corrupt_tree);
  fs::write(&png, &png_bytes).unwrap();

  let mut stale_manifest = manifest.clone();
  stale_manifest["rendererIdentity"] = Value::String("obsolete-renderer".to_owned());
  write_json(&manifest_path, &stale_manifest);
  let stale_tree = snapshot(&fixture.generated_root());
  let stale = fixture.run("check", "stale");
  assert_failure(&stale);
  assert_eq!(stdout(&stale).matches("status=stale").count(), 2);
  assert_eq!(snapshot(&fixture.generated_root()), stale_tree);
  fs::write(&manifest_path, &manifest_bytes).unwrap();

  fs::copy(fixture.asset("Rocket Emoji.png"), &dependency).unwrap();
  let repaired = fixture.run("generate", "selective");
  assert_success(&repaired);
  assert_counts(&repaired, 3, 2, 1, 1, 1);
  assert!(stdout(&repaired).contains("session-requests=1"));
  assert_eq!(stdout(&repaired).matches("status=changed").count(), 1);

  let repaired_tree = snapshot(&fixture.generated_root());
  let warm = fixture.run("generate", "warm");
  assert_success(&warm);
  assert_counts(&warm, 3, 2, 2, 0, 0);
  assert!(stdout(&warm).contains("browser not started"));
  assert_eq!(snapshot(&fixture.generated_root()), repaired_tree);
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn preview_first_generates_every_declaration_and_complete_metadata() {
  let fixture = Fixture::new();
  let previewed = fixture.run("preview", "preview");
  assert_success(&previewed);
  assert_counts(&previewed, 3, 2, 0, 2, 2);

  let html = fs::read_to_string(
    fixture
      .project
      .join("Library/BattlementReactant/asset-preview/index.html"),
  )
  .unwrap();
  assert_eq!(html.matches("<article class=\"card\"").count(), 3);
  for text in [
    "PANEL",
    "PANEL_DUPLICATE",
    "BADGE",
    "checker",
    "data:image/png;base64",
    "Logical canvas",
    "Raster output",
    "Subject bounds",
    "Alpha bounds",
    "Allowed clipping",
    "Edge diagnostics",
    "Authored properties",
    "Dependencies",
    "Assets/Textures/panel.png",
    "renderer ",
    "rules/src/lib.rs",
    "data-slice",
    "data-width",
    "data-height",
  ] {
    assert!(html.contains(text), "preview omitted {text}");
  }
  let report: Value =
    serde_json::from_slice(&fs::read(fixture.report("preview")).unwrap()).unwrap();
  assert_eq!(report["browserLaunches"], 1);
  assert_eq!(report["browserContextsCreated"], 1);
}

#[derive(Debug, Eq, PartialEq)]
struct SnapshotEntry {
  bytes: Option<Vec<u8>>,
  modified: SystemTime,
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, SnapshotEntry> {
  let mut entries = BTreeMap::new();
  capture(root, root, &mut entries);
  entries
}

fn capture(root: &Path, path: &Path, entries: &mut BTreeMap<PathBuf, SnapshotEntry>) {
  let metadata = fs::metadata(path).unwrap();
  entries.insert(
    path.strip_prefix(root).unwrap().to_owned(),
    SnapshotEntry {
      bytes: metadata.is_file().then(|| fs::read(path).unwrap()),
      modified: metadata.modified().unwrap(),
    },
  );
  if metadata.is_dir() {
    for entry in fs::read_dir(path).unwrap() {
      capture(root, &entry.unwrap().path(), entries);
    }
  }
}

fn assert_counts(
  output: &Output,
  discovered: usize,
  deduplicated: usize,
  current: usize,
  rendered: usize,
  stale: usize,
) {
  assert!(
    stdout(output).contains(&format!(
      "discovered={discovered} deduplicated={deduplicated} current={current} rendered={rendered} stale={stale}"
    )),
    "{}",
    stdout(output)
  );
}

fn assert_report_read_only(path: PathBuf) {
  let report: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
  assert_eq!(report["filesWritten"], 0);
  assert_eq!(report["browserLaunches"], 0);
}

fn assert_success(output: &Output) {
  assert!(output.status.success(), "{}", stderr(output));
}

fn assert_failure(output: &Output) {
  assert!(!output.status.success(), "command unexpectedly succeeded");
  assert!(
    stderr(output).contains("assets are stale"),
    "{}",
    stderr(output)
  );
}

fn write_json(path: &Path, value: &Value) {
  let mut bytes = serde_json::to_vec_pretty(value).unwrap();
  bytes.push(b'\n');
  fs::write(path, bytes).unwrap();
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
        "[package]\nname = \"command-preview-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
        reactant
      ),
    )
    .unwrap();
    fs::copy(
      Self::repository_asset("Signal Texture.png"),
      project.join("Assets/Textures/panel.png"),
    )
    .unwrap();
    fs::write(
      project.join("rules/src/lib.rs"),
      r#"battlement_reactant::asset_generator::generate! {
        @nine-slice PANEL {
          @canvas 32px 20px;
          @slices 4px 5px 4px 5px; @allow-clipping top right bottom left;
          background: unity-url("Assets/Textures/panel.png") center / cover;
          box-shadow: inset 1px 1px 1px red;
        }
      }
      battlement_reactant::asset_generator::generate! {
        @nine-slice PANEL_DUPLICATE {
          @canvas 32px 20px;
          @slices 4px 5px 4px 5px; @allow-clipping top right bottom left;
          background: unity-url("Assets/Textures/panel.png") center / cover;
          box-shadow: inset 1px 1px 1px red;
        }
      }
      battlement_reactant::asset_generator::generate! {
        @background BADGE {
          @canvas 24px 16px; @subject 2px 2px 20px 12px;
          background: linear-gradient(135deg, #0ea5e9, #8b5cf6);
          border-radius: 4px;
        }
      }"#,
    )
    .unwrap();
    Self {
      _temporary: temporary,
      project,
    }
  }

  fn generated_root(&self) -> PathBuf {
    self.project.join("Assets/Generated/BattlementReactant")
  }

  fn report(&self, name: &str) -> PathBuf {
    self
      .project
      .parent()
      .unwrap()
      .join(format!("{name}-report.json"))
  }

  fn asset(&self, name: &str) -> PathBuf {
    Self::repository_asset(name)
  }

  fn repository_asset(name: &str) -> PathBuf {
    Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"))
      .join("../../samples/ui/Assets/Original")
      .join(name)
  }

  fn run(&self, command: &str, report: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .args([
        "reactant",
        "assets",
        command,
        "--work-report",
        self.report(report).to_str().unwrap(),
      ])
      .current_dir(&self.project)
      .output()
      .unwrap()
  }
}

fn stdout(output: &Output) -> String {
  String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}
