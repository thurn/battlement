#![cfg(unix)]

use std::{
  fs,
  os::unix::fs::PermissionsExt,
  path::{Path, PathBuf},
  process::{Command, Output},
};

use serde_json::{Value, json};

#[test]
fn generate_creates_typed_hierarchical_constants_and_check_detects_drift() {
  let fixture = Fixture::new(vec![
    entry("content", "Scene", "UnityEditor.SceneAsset"),
    entry("white/king", "Prefab", "UnityEngine.GameObject"),
    entry("music/theme", "AudioClip", "UnityEngine.AudioClip"),
    entry("custom/data", "Untyped", "Example.CustomAsset"),
  ]);

  let generated = fixture.generate(&[]);
  assert!(generated.status.success(), "{}", stderr(&generated));
  assert!(
    read(&fixture.output)
      .contains("pub const KING: PrefabAddress = PrefabAddress::from_static(\"white/king\");")
  );
  assert!(read(&fixture.output).contains("pub mod white {"));
  assert!(read(&fixture.output).contains("AudioClipAddress"));
  assert!(read(&fixture.output).contains("UntypedAssetAddress"));
  assert!(read(&fixture.output).contains("pub const ASSET_CATALOG: &[PreparedAsset] = &["));
  assert!(read(&fixture.output).contains("PreparedAsset::Scene(CONTENT)"));
  assert!(read(&fixture.output).contains("PreparedAsset::AudioClip(music::THEME)"));
  assert!(!read(&fixture.output).contains("PreparedAsset::Untyped"));

  let checked = fixture.generate(&["--check"]);
  assert!(checked.status.success(), "{}", stderr(&checked));
  fs::write(
    &fixture.output,
    format!("{}// stale\n", read(&fixture.output)),
  )
  .unwrap();
  let stale = fixture.generate(&["--check"]);
  assert!(!stale.status.success());
  assert!(stderr(&stale).contains("is stale"));
  let refreshed = fixture.generate(&[]);
  assert!(refreshed.status.success(), "{}", stderr(&refreshed));
  assert!(!read(&fixture.output).contains("// stale"));
}

#[test]
fn generate_discovers_the_project_and_normalizes_keywords() {
  let fixture = Fixture::new(vec![entry(
    "type/match-value",
    "Material",
    "UnityEngine.Material",
  )]);
  let nested = fixture.project.join("Assets/Nested");
  fs::create_dir_all(&nested).unwrap();

  let output = fixture.generate_from(&nested, &[]);

  assert!(output.status.success(), "{}", stderr(&output));
  assert!(read(&fixture.output).contains("pub mod r#type {"));
  assert!(read(&fixture.output).contains("pub const MATCH_VALUE: MaterialAddress"));
}

#[test]
fn generate_refuses_to_replace_an_unmarked_file() {
  let fixture = Fixture::new(vec![entry("asset", "Texture", "UnityEngine.Texture2D")]);
  fs::create_dir_all(fixture.output.parent().unwrap()).unwrap();
  fs::write(&fixture.output, "keep me\n").unwrap();

  let output = fixture.generate(&[]);

  assert!(!output.status.success());
  assert!(stderr(&output).contains("refusing to replace unmarked file"));
  assert_eq!(read(&fixture.output), "keep me\n");
}

#[test]
fn generate_rejects_identifier_collisions() {
  let fixture = Fixture::new(vec![
    entry("pieces/white-king", "Prefab", "UnityEngine.GameObject"),
    entry("pieces/white_king", "Prefab", "UnityEngine.GameObject"),
  ]);

  let output = fixture.generate(&[]);

  assert!(!output.status.success());
  assert!(stderr(&output).contains("map to the same Rust constant WHITE_KING"));
}

#[test]
fn ten_thousand_flat_addresses_are_generated_deterministically_in_one_file() {
  let fixture = Fixture::new(
    (0..10_000)
      .map(|index| {
        entry(
          &format!("asset-{index:05}"),
          "Texture",
          "UnityEngine.Texture2D",
        )
      })
      .collect(),
  );

  let output = fixture.generate(&[]);

  assert!(output.status.success(), "{}", stderr(&output));
  assert_eq!(
    read(&fixture.output).matches(": TextureAddress =").count(),
    10_000
  );
  let checked = fixture.generate(&["--check"]);
  assert!(checked.status.success(), "{}", stderr(&checked));
}

fn entry(address: &str, kind: &str, unity_type: &str) -> Value {
  json!({
      "Address": address,
      "Kind": kind,
      "Group": "Fixture",
      "AssetPath": format!("Assets/{address}.asset"),
      "UnityType": unity_type,
  })
}

struct Fixture {
  _temporary: tempfile::TempDir,
  project: PathBuf,
  output: PathBuf,
  editor: PathBuf,
  export: PathBuf,
}

impl Fixture {
  fn new(entries: Vec<Value>) -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    let output = project.join("rules/src/assets.rs");
    fs::create_dir_all(project.join("Assets")).unwrap();
    fs::create_dir_all(project.join("ProjectSettings")).unwrap();
    fs::write(
      project.join("ProjectSettings/ProjectVersion.txt"),
      "m_EditorVersion: fixture\n",
    )
    .unwrap();
    let export = temporary.path().join("export.json");
    fs::write(&export, json!({ "Entries": entries }).to_string()).unwrap();
    let editor = temporary.path().join("fake-unity");
    fs::write(
      &editor,
      "#!/bin/sh\ncp \"$BATTLEMENT_FAKE_EXPORT\" \"$BATTLEMENT_ADDRESS_EXPORT_PATH\"\n",
    )
    .unwrap();
    let mut permissions = fs::metadata(&editor).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&editor, permissions).unwrap();
    Self {
      _temporary: temporary,
      project,
      output,
      editor,
      export,
    }
  }

  fn generate(&self, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .arg("generate")
      .arg(&self.project)
      .args(extra)
      .env("UNITY_EDITOR", &self.editor)
      .env("BATTLEMENT_FAKE_EXPORT", &self.export)
      .output()
      .unwrap()
  }

  fn generate_from(&self, current_dir: &Path, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .arg("generate")
      .args(extra)
      .current_dir(current_dir)
      .env("UNITY_EDITOR", &self.editor)
      .env("BATTLEMENT_FAKE_EXPORT", &self.export)
      .output()
      .unwrap()
  }
}

fn read(path: impl AsRef<Path>) -> String {
  fs::read_to_string(path).unwrap()
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}
