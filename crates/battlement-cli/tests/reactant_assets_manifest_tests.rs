use std::{
  collections::BTreeSet,
  fs,
  path::{Path, PathBuf},
  process::{Command, Output},
};

use serde_json::Value;
use sha2::{Digest, Sha256};

#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn generated_tree_is_canonical_complete_and_strictly_checked() {
  let fixture = Fixture::new();
  let generated = fixture.generate();
  assert!(generated.status.success(), "{}", stderr(&generated));

  let root = fixture.generated_root();
  let manifest_path = root.join("manifest.json");
  let sidecar_path = root.join("Resources/BattlementReactantAssetCatalog.json");
  let manifest_bytes = fs::read(&manifest_path).unwrap();
  let sidecar_bytes = fs::read(&sidecar_path).unwrap();
  let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
  let sidecar: Value = serde_json::from_slice(&sidecar_bytes).unwrap();
  assert_sorted_keys(&manifest);
  assert_sorted_keys(&sidecar);
  assert!(manifest_bytes.ends_with(b"\n"));
  assert!(sidecar_bytes.ends_with(b"\n"));
  assert_eq!(manifest["assets"].as_array().unwrap().len(), 1);
  let asset = &manifest["assets"][0];
  assert_eq!(asset["kind"], "nineSlice");
  assert_eq!(asset["rasterScale"], 3);
  assert_eq!(
    asset["rasterSize"],
    serde_json::json!({"height": 36, "width": 48})
  );
  assert_eq!(asset["import"]["compression"], "lossyHigh");
  assert_eq!(asset["import"]["filterMode"], "nearest");
  assert_eq!(asset["import"]["wrapMode"], "repeat");
  assert_eq!(asset["dependencies"][0]["kind"], "image");
  assert_eq!(
    sidecar["manifestSha256"],
    hex(&Sha256::digest(&manifest_bytes))
  );
  assert_eq!(sidecar["addresses"][0], asset["address"]);
  let hash = asset["canonicalRequestSha256"].as_str().unwrap();
  assert_eq!(
    tree(&fixture.project),
    [
      "Assets/Generated/BattlementReactant.meta".to_owned(),
      "Assets/Generated/BattlementReactant/Resources".to_owned(),
      "Assets/Generated/BattlementReactant/Resources.meta".to_owned(),
      "Assets/Generated/BattlementReactant/Resources/BattlementReactantAssetCatalog.json"
        .to_owned(),
      "Assets/Generated/BattlementReactant/Resources/BattlementReactantAssetCatalog.json.meta"
        .to_owned(),
      "Assets/Generated/BattlementReactant/manifest.json".to_owned(),
      "Assets/Generated/BattlementReactant/manifest.json.meta".to_owned(),
      "Assets/Generated/BattlementReactant/textures".to_owned(),
      "Assets/Generated/BattlementReactant/textures.meta".to_owned(),
      format!("Assets/Generated/BattlementReactant/textures/{hash}.png"),
      format!("Assets/Generated/BattlementReactant/textures/{hash}.png.meta"),
    ]
    .into_iter()
    .collect()
  );

  fs::write(&manifest_path, &manifest_bytes).unwrap();
  assert_success(fixture.check());

  for (name, mutate) in [
    (
      "asset",
      mutate(&["assets", "0", "address"], Value::String("wrong".into())),
    ),
    (
      "browser",
      mutate(&["browser", "version"], Value::String(String::new())),
    ),
    (
      "dependency",
      mutate(
        &["assets", "0", "dependencies", "0", "path"],
        Value::String("../escape.png".into()),
      ),
    ),
    (
      "import",
      mutate(&["assets", "0", "import", "mipmaps"], Value::Bool(true)),
    ),
    (
      "geometry",
      mutate(&["assets", "0", "rasterScale"], Value::from(9)),
    ),
  ] {
    fs::write(&manifest_path, &manifest_bytes).unwrap();
    let mut changed: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    mutate(&mut changed);
    write_json(&manifest_path, &changed);
    assert_stale(fixture.check(), name);
  }

  fs::write(&manifest_path, &manifest_bytes).unwrap();
  let mut unknown: Value = serde_json::from_slice(&manifest_bytes).unwrap();
  unknown
    .as_object_mut()
    .unwrap()
    .insert("unknown".into(), Value::Null);
  write_json(&manifest_path, &unknown);
  assert_stale(fixture.check(), "schema");

  fs::write(&manifest_path, &manifest_bytes).unwrap();
  let mut bad_sidecar: Value = serde_json::from_slice(&sidecar_bytes).unwrap();
  bad_sidecar["manifestSha256"] = Value::String("0".repeat(64));
  write_json(&sidecar_path, &bad_sidecar);
  assert_stale(fixture.check(), "sidecar");
  fs::write(&sidecar_path, &sidecar_bytes).unwrap();

  let texture_meta = root.join(format!("textures/{hash}.png.meta"));
  let metadata = fs::read_to_string(&texture_meta).unwrap();
  let reordered = metadata.replacen(
    "  isReadable: 0\n  streamingMipmaps: 0",
    "  streamingMipmaps: 0\n  isReadable: 0",
    1,
  );
  fs::write(&texture_meta, reordered).unwrap();
  assert_success(fixture.check());

  let override_added = metadata.replace(
    "  spriteSheet:",
    "  - buildTarget: Standalone\n    overridden: 1\n  spriteSheet:",
  );
  fs::write(&texture_meta, override_added).unwrap();
  assert_stale(fixture.check(), "platform override");
  let labeled = metadata.replace("  assetBundleName:\n", "  assetBundleName: generated\n");
  fs::write(&texture_meta, labeled).unwrap();
  assert_stale(fixture.check(), "label");

  fs::write(&texture_meta, metadata).unwrap();
  fs::write(root.join("unknown.txt"), "stale").unwrap();
  assert_stale(fixture.check(), "tree");
}

type Mutation = Box<dyn Fn(&mut Value)>;

fn mutate(path: &'static [&'static str], replacement: Value) -> Mutation {
  Box::new(move |value| set(value, path, replacement.clone()))
}

fn set(value: &mut Value, path: &[&str], replacement: Value) {
  let mut selected = value;
  for part in &path[..path.len() - 1] {
    selected = if let Ok(index) = part.parse::<usize>() {
      &mut selected.as_array_mut().unwrap()[index]
    } else {
      selected.as_object_mut().unwrap().get_mut(*part).unwrap()
    };
  }
  selected
    .as_object_mut()
    .unwrap()
    .insert(path.last().unwrap().to_string(), replacement);
}

fn assert_sorted_keys(value: &Value) {
  match value {
    Value::Array(values) => values.iter().for_each(assert_sorted_keys),
    Value::Object(values) => {
      let keys = values.keys().collect::<Vec<_>>();
      assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
      values.values().for_each(assert_sorted_keys);
    }
    _ => {}
  }
}

fn write_json(path: &Path, value: &Value) {
  let mut bytes = serde_json::to_vec_pretty(value).unwrap();
  bytes.push(b'\n');
  fs::write(path, bytes).unwrap();
}

fn tree(project: &Path) -> BTreeSet<String> {
  let mut paths = BTreeSet::new();
  let generated = project.join("Assets/Generated/BattlementReactant");
  walk(project, &generated, &mut paths);
  paths.insert("Assets/Generated/BattlementReactant.meta".to_owned());
  paths
}

fn walk(project: &Path, path: &Path, output: &mut BTreeSet<String>) {
  for entry in fs::read_dir(path).unwrap() {
    let path = entry.unwrap().path();
    output.insert(
      path
        .strip_prefix(project)
        .unwrap()
        .to_string_lossy()
        .into_owned(),
    );
    if path.is_dir() {
      walk(project, &path, output);
    }
  }
}

fn assert_success(output: Output) {
  assert!(output.status.success(), "{}", stderr(&output));
}

fn assert_stale(output: Output, family: &str) {
  assert!(!output.status.success(), "{family} corruption was accepted");
  assert!(
    stderr(&output).contains("assets are stale"),
    "{}",
    stderr(&output)
  );
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
    let reactant = Path::new(env!("CARGO_MANIFEST_DIR"))
      .parent()
      .unwrap()
      .join("battlement-reactant");
    fs::write(
      project.join("rules/Cargo.toml"),
      format!(
        "[package]\nname = \"manifest-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
        reactant
      ),
    )
    .unwrap();
    fs::copy(
      Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/ui/Assets/Original/Signal Texture.png"),
      project.join("Assets/Textures/panel.png"),
    )
    .unwrap();
    fs::write(
      project.join("rules/src/lib.rs"),
      r#"battlement_reactant::asset_generator::generate! {
        @nine-slice PANEL {
          @canvas 16px 12px;
          @slices 2px 2px 2px 2px;
          @allow-clipping top right bottom left;
          @raster-scale 3;
          @filter-mode nearest;
          @wrap-mode repeat;
          @compression lossy-high;
          background: unity-url("Assets/Textures/panel.png") center / cover;
          box-shadow: inset 1px 1px 1px red;
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

  fn generate(&self) -> Output {
    self.run("generate")
  }

  fn check(&self) -> Output {
    self.run("check")
  }

  fn run(&self, command: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .args(["reactant", "assets", command])
      .current_dir(&self.project)
      .output()
      .unwrap()
  }
}

fn hex(bytes: &[u8]) -> String {
  const DIGITS: &[u8; 16] = b"0123456789abcdef";

  let mut output = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    output.push(char::from(DIGITS[usize::from(byte >> 4)]));
    output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
  }
  output
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}
