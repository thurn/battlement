use std::{
  collections::BTreeMap,
  env, fs,
  io::Cursor,
  path::{Path, PathBuf},
  process::{Command, Output},
};

use serde_json::Value;

type AlphaBounds = (u32, u32, u32, u32);
type RenderRecord = (u32, u32, AlphaBounds);

#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn real_browser_batch_emits_deterministic_rgba_png_metadata_for_every_paint_family() {
  let fixture = Fixture::new();
  fixture.install_dependencies();
  fixture.write_source(PAINT_BATCH);
  let report_path = fixture.root.join("batch.json");

  let first = fixture.generate(&report_path);

  assert!(first.status.success(), "{}", stderr(&first));
  let work = report(&report_path);
  assert_eq!(work["browserLaunches"], 1);
  assert_eq!(work["browserContextsCreated"], 1);
  assert_eq!(work["dependencyFileOpens"], 2);
  assert!(stdout(&first).contains("session-requests=8"));
  let cache = cache_keys(&first);
  let renders = render_records(&first);
  assert_eq!(cache.len(), 8);
  assert_eq!(renders.len(), 8);
  let dimensions = renders
    .values()
    .map(|render| (render.0, render.1))
    .collect::<std::collections::BTreeSet<_>>();
  assert_eq!(dimensions, [(64, 48), (96, 72)].into_iter().collect());
  for (address, key) in &cache {
    let path = fixture.cache_path(key);
    let inspected = inspect_png(&path);
    let reported = renders.get(address).unwrap();
    assert_eq!(
      (inspected.width, inspected.height),
      (reported.0, reported.1)
    );
    assert_eq!(inspected.alpha, reported.2);
    assert_eq!(inspected.chunks, ["IHDR", "sRGB", "IDAT", "IEND"]);
  }

  let warm_report = fixture.root.join("warm.json");
  let warm = fixture.generate(&warm_report);
  assert!(warm.status.success(), "{}", stderr(&warm));
  assert!(stdout(&warm).contains("session-requests=0"));
  assert_eq!(cache_keys(&warm), cache);
  assert_eq!(render_records(&warm), renders);
  assert_eq!(report(&warm_report)["browserLaunches"], 0);

  let check_report = fixture.root.join("check.json");
  let checked = fixture.check(&check_report);
  assert!(checked.status.success(), "{}", stderr(&checked));
  assert_eq!(report(&check_report)["browserLaunches"], 0);
  assert_eq!(report(&check_report)["filesWritten"], 0);

  fixture.write_source(&PAINT_BATCH.replacen(
    "linear-gradient(red, blue)",
    "linear-gradient(red, green)",
    1,
  ));
  let source_report = fixture.root.join("source-change.json");
  let source_changed = fixture.generate(&source_report);
  assert!(
    source_changed.status.success(),
    "{}",
    stderr(&source_changed)
  );
  assert!(stdout(&source_changed).contains("session-requests=1"));
  assert_eq!(report(&source_report)["browserLaunches"], 1);
  let source_cache = cache_keys(&source_changed);
  assert_eq!(source_cache.len(), 8);
  assert_eq!(
    cache
      .keys()
      .filter(|address| source_cache.contains_key(*address))
      .count(),
    7
  );

  fs::copy(
    Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"))
      .join("../../samples/ui/Assets/Original/Rocket Emoji.png"),
    fixture.project.join("Assets/Textures/source.png"),
  )
  .unwrap();
  let dependency_report = fixture.root.join("dependency-change.json");
  let dependency_changed = fixture.generate(&dependency_report);
  assert!(
    dependency_changed.status.success(),
    "{}",
    stderr(&dependency_changed)
  );
  assert!(stdout(&dependency_changed).contains("session-requests=1"));
  assert_eq!(report(&dependency_report)["browserLaunches"], 1);
  let dependency_cache = cache_keys(&dependency_changed);
  assert_eq!(
    dependency_cache.keys().collect::<Vec<_>>(),
    source_cache.keys().collect::<Vec<_>>()
  );
  assert_eq!(
    dependency_cache
      .iter()
      .filter(|(address, key)| source_cache.get(*address) != Some(*key))
      .count(),
    1
  );

  let manifest: Value = serde_json::from_slice(
    &fs::read(
      fixture
        .project
        .join("Assets/Generated/BattlementReactant/manifest.json"),
    )
    .unwrap(),
  )
  .unwrap();
  let png = fixture
    .project
    .join("Assets/Generated/BattlementReactant")
    .join(manifest["assets"][0]["png"].as_str().unwrap());
  fs::write(&png, b"corrupt PNG").unwrap();
  let corrupt_report = fixture.root.join("corrupt.json");
  let corrupt = fixture.check(&corrupt_report);
  assert!(!corrupt.status.success());
  assert!(stdout(&corrupt).contains("status=corrupt"));
  assert_eq!(report(&corrupt_report)["filesWritten"], 0);
  let repaired_report = fixture.root.join("repaired.json");
  let repaired = fixture.generate(&repaired_report);
  assert!(repaired.status.success(), "{}", stderr(&repaired));
  assert_eq!(report(&repaired_report)["browserLaunches"], 0);
}

#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn valid_renders_report_each_stable_warning_category() {
  let fixture = Fixture::new();
  fixture.write_source(WARNING_BATCH);
  let report_path = fixture.root.join("warnings.json");

  let output = fixture.generate(&report_path);

  assert!(output.status.success(), "{}", stderr(&output));
  let transcript = stdout(&output);
  for category in [
    "warning[large-raster-allocation]",
    "warning[lossy-translucent-compression]",
    "warning[near-permitted-edge]",
  ] {
    assert!(
      transcript.contains(category),
      "missing {category}:\n{transcript}"
    );
  }
  assert_eq!(report(&report_path)["browserLaunches"], 1);
}

#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn one_failed_request_publishes_no_render_cache_entries() {
  let fixture = Fixture::new();
  fixture.write_source(
    "battlement_reactant::asset_generator::generate! {\n\
       @background SAFE { @canvas 16px 16px; @subject 2px 2px 12px 12px; background: linear-gradient(red, blue); }\n\
     }\n\
     battlement_reactant::asset_generator::generate! {\n\
       @background CLIPPED { @canvas 16px 16px; background: linear-gradient(red, blue); }\n\
     }\n",
  );
  let report_path = fixture.root.join("failed.json");

  let output = fixture.generate(&report_path);

  assert!(!output.status.success());
  assert!(stderr(&output).contains("unpermitted"));
  let cache = fixture
    .project
    .join("Library/BattlementReactant/asset-generator-cache");
  assert!(!cache.exists() || fs::read_dir(cache).unwrap().next().is_none());
}

#[test]
#[ignore = "run by scripts/reactant_asset_validation.py"]
fn browser_shaping_rejects_an_ignored_joiner_without_publishing_pixels() {
  let fixture = Fixture::new();
  fixture.install_dependencies();
  fixture.write_source(
    r#"battlement_reactant::asset_generator::generate! {
      @text-image JOINED {
        @canvas 32px 24px;
        @subject 3px 3px 26px 18px;
        @font-file unity("Assets/Fonts/command.ttf");
        content: "A\u{200d}B";
        font-size: 8px;
        color: transparent;
        background: linear-gradient(red, blue);
        background-clip: text;
      }
    }"#,
  );
  let report_path = fixture.root.join("shaping.json");

  let output = fixture.generate(&report_path);

  assert!(!output.status.success());
  assert!(
    stderr(&output).contains("ignored a variation selector or joiner"),
    "{}",
    stderr(&output)
  );
  assert!(
    !fixture
      .project
      .join("Library/BattlementReactant/asset-generator-cache")
      .exists()
  );
}

const PAINT_BATCH: &str = r#"
battlement_reactant::asset_generator::generate! {
  @background GRADIENT { @canvas 32px 24px; @subject 4px 4px 24px 16px; background: linear-gradient(red, blue); }
}
battlement_reactant::asset_generator::generate! {
  @background CLIP { @canvas 32px 24px; @subject 4px 4px 24px 16px; background: linear-gradient(red, blue); clip-path: circle(40% at 50% 50%); }
}
battlement_reactant::asset_generator::generate! {
  @background MASK { @canvas 32px 24px; @subject 4px 4px 24px 16px; background: linear-gradient(red, blue); mask: linear-gradient(red, transparent) alpha; }
}
battlement_reactant::asset_generator::generate! {
  @background SHADOW { @canvas 32px 24px; @subject 6px 6px 20px 12px; background: linear-gradient(red, blue); box-shadow: 1px 1px 2px red, inset 1px 1px 2px blue; }
}
battlement_reactant::asset_generator::generate! {
  @background EFFECTS { @canvas 32px 24px; @subject 6px 6px 20px 12px; background: linear-gradient(red, blue); filter: blur(1px) saturate(1.2); transform: rotate(5deg); transform-origin: left top; }
}
battlement_reactant::asset_generator::generate! {
  @nine-slice FRAME { @canvas 32px 24px; @subject 4px 4px 24px 16px; @slices 3px 3px 3px 3px; @raster-scale 3; background: linear-gradient(red, blue); border: 1px dashed white; }
}
battlement_reactant::asset_generator::generate! {
  @background LOCAL { @canvas 32px 24px; @subject 4px 4px 24px 16px; background: unity-url("Assets/Textures/source.png") center / 8px 8px no-repeat; box-shadow: 1px 1px 1px red; }
}
battlement_reactant::asset_generator::generate! {
  @text-image TEXT { @canvas 32px 24px; @subject 3px 3px 26px 18px; @font-file unity("Assets/Fonts/command.ttf"); content: "e\u{301}"; font-size: 8px; color: transparent; background: linear-gradient(red, blue); background-clip: text; text-shadow: 1px 1px 1px blue; }
}
"#;

const WARNING_BATCH: &str = r#"
battlement_reactant::asset_generator::generate! {
  @background EDGE { @canvas 16px 16px; @subject 0px 2px 12px 12px; @allow-clipping left; background: linear-gradient(red, blue); }
}
battlement_reactant::asset_generator::generate! {
  @background LOSSY { @canvas 16px 16px; @subject 2px 2px 12px 12px; @compression lossy-low; background: linear-gradient(rgba(255, 0, 0, 0.5), rgba(0, 0, 255, 0.5)); }
}
battlement_reactant::asset_generator::generate! {
  @background LARGE { @canvas 2049px 2048px; @subject 8px 8px 32px 32px; @raster-scale 1; background: linear-gradient(red, blue); }
}
"#;

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
    for directory in ["Assets", "Packages", "ProjectSettings", "rules/src"] {
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
        "[package]\nname = \"render-fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[dependencies]\nbattlement-reactant = {{ path = {:?} }}\n",
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

  fn install_dependencies(&self) {
    fs::create_dir_all(self.project.join("Assets/Textures")).unwrap();
    fs::create_dir_all(self.project.join("Assets/Fonts")).unwrap();
    let repository =
      Path::new(&env::var("CARGO_MANIFEST_DIR").expect("Cargo provides the manifest directory"))
        .join("../..");
    fs::copy(
      repository.join("samples/ui/Assets/Original/Signal Texture.png"),
      self.project.join("Assets/Textures/source.png"),
    )
    .unwrap();
    fs::copy(
      repository.join("samples/ui/Assets/Original/Command Mono.ttf"),
      self.project.join("Assets/Fonts/command.ttf"),
    )
    .unwrap();
  }

  fn write_source(&self, source: &str) {
    fs::write(self.project.join("rules/src/lib.rs"), source).unwrap();
  }

  fn generate(&self, report: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .args([
        "reactant",
        "assets",
        "generate",
        "--work-report",
        report.to_str().unwrap(),
      ])
      .current_dir(&self.project)
      .output()
      .unwrap()
  }

  fn check(&self, report: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cargo-battlement"))
      .args([
        "reactant",
        "assets",
        "check",
        "--work-report",
        report.to_str().unwrap(),
      ])
      .current_dir(&self.project)
      .output()
      .unwrap()
  }

  fn cache_path(&self, key: &str) -> PathBuf {
    self
      .project
      .join("Library/BattlementReactant/asset-generator-cache")
      .join(format!("{key}.png"))
  }
}

#[derive(Debug, Eq, PartialEq)]
struct InspectedPng {
  width: u32,
  height: u32,
  alpha: (u32, u32, u32, u32),
  chunks: Vec<String>,
}

fn inspect_png(path: &Path) -> InspectedPng {
  let bytes = fs::read(path).unwrap();
  let chunks = png_chunks(&bytes);
  let mut reader = png::Decoder::new(Cursor::new(&bytes)).read_info().unwrap();
  assert_eq!(reader.info().color_type, png::ColorType::Rgba);
  assert_eq!(reader.info().bit_depth, png::BitDepth::Eight);
  assert!(reader.info().srgb.is_some());
  let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
  let output = reader.next_frame(&mut pixels).unwrap();
  pixels.truncate(output.buffer_size());
  let mut alpha = (output.width, output.height, 0, 0);
  for (index, pixel) in pixels.as_chunks::<4>().0.iter().enumerate() {
    if pixel[3] == 0 {
      continue;
    }
    let x = index as u32 % output.width;
    let y = index as u32 / output.width;
    alpha = (
      alpha.0.min(x),
      alpha.1.min(y),
      alpha.2.max(x),
      alpha.3.max(y),
    );
  }
  InspectedPng {
    width: output.width,
    height: output.height,
    alpha,
    chunks,
  }
}

fn png_chunks(bytes: &[u8]) -> Vec<String> {
  let mut offset = 8;
  let mut chunks = Vec::new();
  while offset < bytes.len() {
    let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
    chunks.push(String::from_utf8(bytes[offset + 4..offset + 8].to_vec()).unwrap());
    offset += 12 + length;
  }
  chunks
}

fn cache_keys(output: &Output) -> BTreeMap<String, String> {
  stdout(output)
    .lines()
    .filter_map(|line| line.strip_prefix("cache="))
    .map(|line| {
      let mut fields = line.split_whitespace();
      (
        fields.next().unwrap().to_owned(),
        fields
          .next()
          .unwrap()
          .strip_prefix("key=")
          .unwrap()
          .to_owned(),
      )
    })
    .collect()
}

fn render_records(output: &Output) -> BTreeMap<String, RenderRecord> {
  stdout(output)
    .lines()
    .filter_map(|line| line.strip_prefix("render="))
    .map(|line| {
      let mut fields = line.split_whitespace();
      let address = fields.next().unwrap().to_owned();
      let dimensions = fields
        .next()
        .unwrap()
        .strip_prefix("dimensions=")
        .unwrap()
        .split_once('x')
        .unwrap();
      let alpha = fields.next().unwrap().strip_prefix("alpha=").unwrap();
      let alpha = alpha
        .split(',')
        .map(|value| value.parse::<u32>().unwrap())
        .collect::<Vec<_>>();
      (
        address,
        (
          dimensions.0.parse().unwrap(),
          dimensions.1.parse().unwrap(),
          (alpha[0], alpha[1], alpha[2], alpha[3]),
        ),
      )
    })
    .collect()
}

fn report(path: &Path) -> Value {
  serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn stdout(output: &Output) -> String {
  String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
  String::from_utf8_lossy(&output.stderr).into_owned()
}
