use std::{
  env, fs,
  path::{Path, PathBuf},
};

use battlement_ditto::coverage_ledger::{self, SampleStatus};

#[test]
fn repository_report_discovers_every_pending_migration() {
  let repository = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../..");
  let report = coverage_ledger::check_repository(&repository).unwrap();
  assert_eq!(
    report
      .samples
      .iter()
      .map(|sample| (sample.sample.as_str(), sample.state_count, &sample.status))
      .collect::<Vec<_>>(),
    vec![
      ("basic", 7, &SampleStatus::Complete),
      ("chess", 17, &SampleStatus::Complete),
      ("chess-ui", 74, &SampleStatus::Complete),
      ("reactant", 48, &SampleStatus::Complete),
      ("tictactoe", 7, &SampleStatus::Complete),
      ("ui", 88, &SampleStatus::Complete),
    ]
  );
}

#[test]
fn canonical_capture_dimensions_follow_the_declared_profile() {
  let fixture = Fixture::new();
  fixture.replace(
    "samples/fixture/ditto.toml",
    "width = 1280, height = 720",
    "width = 1024, height = 1536",
  );
  let error = coverage_ledger::check_repository(fixture.root())
    .unwrap_err()
    .to_string();
  assert!(error.contains("dimensions must match"), "{error}");
  for _ in 0..2 {
    fixture.replace(
      "samples/fixture/ditto.lock",
      "width = 1280\nheight = 720",
      "width = 1024\nheight = 1536",
    );
  }
  fixture.check();
  fixture.replace("samples/fixture/ditto.toml", "scale = 1.0", "scale = 2.0");
  let error = coverage_ledger::check_repository(fixture.root())
    .unwrap_err()
    .to_string();
  assert!(error.contains("must be macOS at scale 1"), "{error}");
}

#[test]
fn supplemental_baselines_require_known_profiles_and_keep_canonical_coverage() {
  let fixture = Fixture::new();
  let extra = LOCK
    .split("[[baselines]]")
    .nth(1)
    .unwrap()
    .replace("profile = \"macos\"", "profile = \"web\"");
  fs::write(
    fixture.root().join("samples/fixture/ditto.lock"),
    format!("{LOCK}\n[[baselines]]{extra}"),
  )
  .unwrap();
  fixture.check();
  fixture.replace(
    "samples/fixture/ditto.lock",
    "profile = \"web\"",
    "profile = \"unknown\"",
  );
  let error = coverage_ledger::check_repository(fixture.root())
    .unwrap_err()
    .to_string();
  assert!(error.contains("unknown profile"), "{error}");
  fixture.replace(
    "samples/fixture/ditto.lock",
    "profile = \"unknown\"",
    "profile = \"web\"",
  );
  fixture.replace(
    "samples/fixture/ditto.lock",
    "profile = \"macos\"\nscenario = \"initial\"",
    "profile = \"macos\"\nscenario = \"absent\"",
  );
  let error = coverage_ledger::check_repository(fixture.root())
    .unwrap_err()
    .to_string();
  assert!(error.contains("missing baseline"), "{error}");
}

#[test]
fn synthetic_gap_matrix_names_every_missing_or_duplicate_fact() {
  Fixture::new().check();
  let cases = [
    (
      "samples/ditto-coverage.toml",
      "samples = [\"fixture\"]",
      "samples = [\"missing\"]",
      "missing sample",
    ),
    (
      "samples/ditto-coverage.toml",
      "samples = [\"fixture\"]",
      "samples = [\"fixture\", \"fixture\"]",
      "contains duplicates",
    ),
    (
      "samples/fixture/ditto-visual-states.toml",
      "[[states]]\nkey = \"screen.changed\"\nscreen = \"screen\"",
      "",
      "unknown transition destination",
    ),
    (
      "samples/fixture/ditto-visual-states.toml",
      "key = \"screen.changed\"",
      "key = \"screen.initial\"",
      "duplicate state",
    ),
    (
      "samples/fixture/ditto-coverage.toml",
      "[[transitions]]\nfrom = \"screen.changed\"\nto = \"screen.initial\"\n",
      "",
      "missing transition",
    ),
    (
      "samples/fixture/ditto.toml",
      "[[scenarios]]\nname = \"changed\"\n[[scenarios.steps]]\nscreenshot = { name = \"changed\" }",
      "",
      "references missing scenario",
    ),
    (
      "samples/fixture/ditto.toml",
      "name = \"changed\" }",
      "name = \"renamed\" }",
      "missing checkpoint",
    ),
    (
      "samples/fixture/ditto-coverage.toml",
      "owner = \"tests::changed\"",
      "owner = \"\"",
      "test owner",
    ),
    (
      "samples/fixture/ditto.lock",
      "scenario = \"changed\"\ncheckpoint = \"changed\"",
      "scenario = \"changed\"\ncheckpoint = \"missing\"",
      "missing baseline",
    ),
    (
      "samples/fixture/ditto.toml",
      "\n[profiles.web]\ntarget = \"webgl\"\ndisplay = { width = 1280, height = 720, scale = 1.0 }\n",
      "\n",
      "skip has unknown profile",
    ),
    (
      "samples/fixture/ditto-coverage.toml",
      "[[skips]]\nstate = \"screen.changed\"\nprofile = \"web\"\nreason = \"adapter smoke only\"\n",
      "",
      "missing platform skip",
    ),
    (
      "samples/fixture/ditto-coverage.toml",
      "reason = \"state is not exposed\"",
      "reason = \"\"",
      "requires a reason",
    ),
    (
      "samples/fixture/ditto-coverage.toml",
      "condition = \"failure.recoverable\"",
      "condition = \"Failure Recoverable\"",
      "is not canonical",
    ),
  ];
  for (path, original, replacement, expected) in cases {
    let fixture = Fixture::new();
    fixture.replace(path, original, replacement);
    let error = format!(
      "{:#}",
      coverage_ledger::check_repository(fixture.root()).unwrap_err()
    );
    assert!(
      error.contains(expected),
      "expected {expected:?} in {error:?}"
    );
  }
}

#[test]
fn pending_ledger_rejects_conditional_omissions() {
  let fixture = Fixture::new();
  fs::write(
    fixture.root().join("samples/fixture/ditto-coverage.toml"),
    r#"version = 1
sample = "fixture"
pending_tasks = [43]

[[conditional_omissions]]
condition = "failure.recoverable"
reason = "state is not exposed"
"#,
  )
  .unwrap();

  let error = format!(
    "{:#}",
    coverage_ledger::check_repository(fixture.root()).unwrap_err()
  );
  assert!(
    error.contains("pending ledger contains conditional omissions"),
    "{error}"
  );
}

#[test]
fn a_new_convention_sample_requires_both_registry_and_ledger() {
  let fixture = Fixture::new();
  fixture.replace(
    "samples/ditto-coverage.toml",
    "samples = [\"fixture\"]",
    "samples = [\"fixture\", \"new-sample\"]",
  );
  let sample = fixture.root().join("samples/new-sample");
  fs::create_dir(&sample).unwrap();
  fs::write(
    sample.join("sample.toml"),
    "application = 'new'\nscene = 'new'\n",
  )
  .unwrap();
  let error = coverage_ledger::check_repository(fixture.root())
    .unwrap_err()
    .to_string();
  assert!(error.contains("ditto-visual-states.toml"), "{error}");
}

struct Fixture {
  temporary: tempfile::TempDir,
}

impl Fixture {
  fn new() -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    fs::create_dir_all(root.join("samples/fixture/Assets/Scenes")).unwrap();
    fs::create_dir_all(root.join("samples/fixture/rules")).unwrap();
    fs::write(root.join("samples/ditto-coverage.toml"), CATALOG).unwrap();
    fs::write(
      root.join("samples/fixture/sample.toml"),
      "application = 'fixture'\nscene = 'fixture'\n",
    )
    .unwrap();
    fs::write(
      root.join("samples/fixture/ditto-visual-states.toml"),
      REGISTRY,
    )
    .unwrap();
    fs::write(root.join("samples/fixture/ditto-coverage.toml"), LEDGER).unwrap();
    fs::write(root.join("samples/fixture/ditto.toml"), SUITE).unwrap();
    fs::write(root.join("samples/fixture/ditto.lock"), LOCK).unwrap();
    fs::write(root.join("samples/fixture/Assets/Scenes/Game.unity"), "").unwrap();
    fs::write(
      root.join("samples/fixture/rules/Cargo.toml"),
      "[package]\nname='fixture'\n",
    )
    .unwrap();
    assert!(
      std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(root)
        .status()
        .unwrap()
        .success()
    );
    Self { temporary }
  }

  fn root(&self) -> &Path {
    self.temporary.path()
  }

  fn replace(&self, relative: &str, original: &str, replacement: &str) {
    let path = self.root().join(relative);
    let source = fs::read_to_string(&path).unwrap();
    assert!(
      source.contains(original),
      "fixture source lacks {original:?}"
    );
    fs::write(path, source.replacen(original, replacement, 1)).unwrap();
  }

  fn check(&self) {
    let report = coverage_ledger::check_repository(self.root()).unwrap();
    assert_eq!(report.samples[0].status, SampleStatus::Complete);
  }
}

const CATALOG: &str = "version = 1\nsamples = [\"fixture\"]\n";

const REGISTRY: &str = r#"version = 1

[[states]]
key = "screen.initial"
screen = "screen"

[[states]]
key = "screen.changed"
screen = "screen"
unsupported_profiles = ["web"]

[[transitions]]
from = "screen.initial"
to = "screen.changed"

[[transitions]]
from = "screen.changed"
to = "screen.initial"
"#;

const LEDGER: &str = r#"version = 1
sample = "fixture"
canonical_profile = "macos"

[[mappings]]
state = "screen.initial"
scenario = "initial"
checkpoint = "initial"
owner = "tests::initial"

[[mappings]]
state = "screen.changed"
scenario = "changed"
checkpoint = "changed"
owner = "tests::changed"

[[transitions]]
from = "screen.initial"
to = "screen.changed"

[[transitions]]
from = "screen.changed"
to = "screen.initial"

[[skips]]
state = "screen.changed"
profile = "web"
reason = "adapter smoke only"

[[conditional_omissions]]
condition = "failure.recoverable"
reason = "state is not exposed"
"#;

const SUITE: &str = r#"name = "fixture"
default_profile = "macos"

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[profiles.macos]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[profiles.web]
target = "webgl"
display = { width = 1280, height = 720, scale = 1.0 }

[[scenarios]]
name = "initial"
[[scenarios.steps]]
screenshot = { name = "initial" }

[[scenarios]]
name = "changed"
[[scenarios.steps]]
screenshot = { name = "changed" }
"#;

const LOCK: &str = r#"suite = "fixture"
namespace = "fixture"

[[baselines]]
profile = "macos"
scenario = "initial"
checkpoint = "initial"
sha256 = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
width = 1280
height = 720
size_bytes = 1
source = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"

[[baselines]]
profile = "macos"
scenario = "changed"
checkpoint = "changed"
sha256 = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
width = 1280
height = 720
size_bytes = 1
source = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
"#;
