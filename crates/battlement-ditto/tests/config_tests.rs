use std::{fs, path::PathBuf};

use battlement_ditto::config::{
  self,
  model::{Baseline, Motion, Profile, StepKind, VideoStep},
};

#[test]
fn complete_suite_applies_member_defaults_and_preserves_exact_decimals() {
  let fixture = Fixture::new(FULL_SUITE);
  let suite = fixture.load().unwrap();
  assert_eq!(suite.name, "complete");
  assert_eq!(suite.timeouts.run.as_millis(), 60_000);
  assert_eq!(suite.timeouts.build.as_millis(), 900_000);
  assert_eq!(suite.defaults.step_timeout.as_millis(), 750);
  assert_eq!(suite.defaults.scenario_timeout.as_millis(), 12_000);
  assert_eq!(suite.defaults.motion, Motion::Instant);
  assert_eq!(suite.scenarios[0].fixture, None);
  assert_eq!(
    suite.defaults.comparison.threshold.as_str(),
    "0.10000000000000001"
  );
  assert_eq!(
    suite.defaults.comparison.max_changed_percent.as_str(),
    "0.01"
  );
  assert!(!suite.defaults.comparison.anti_alias);
  assert!(matches!(
    suite.profiles["macos-local"],
    Profile::Macos { .. }
  ));
  assert!(matches!(suite.profiles["web-ci"], Profile::Webgl { .. }));
  assert!(matches!(
    suite.profiles["iphone-ci"],
    Profile::IosSimulator { .. }
  ));
  assert_eq!(suite.scenarios[0].steps.len(), 13);
  assert!(matches!(
    suite.scenarios[0].steps[10].action,
    StepKind::Video(VideoStep::Start { .. })
  ));
  let StepKind::Screenshot(screenshot) = &suite.scenarios[0].steps[9].action else {
    panic!("expected screenshot");
  };
  assert_eq!(screenshot.comparison.threshold.as_str(), "0.05");
  assert_eq!(screenshot.comparison.max_changed_percent.as_str(), "0");
}

#[test]
fn filesystem_baseline_may_resolve_outside_the_repository() {
  let temporary = tempfile::tempdir().unwrap();
  let external = temporary.path().join("baselines");
  fs::create_dir(&external).unwrap();
  let external = external.canonicalize().unwrap();
  let suite = MINIMAL_SUITE.replace(
    R2_BASELINE,
    &format!(
      "[baseline]\nkind = \"filesystem\"\nnamespace = \"example/local\"\nroot = {:?}\n",
      external
    ),
  );
  let fixture = Fixture::new(&suite);
  assert!(matches!(
    fixture.load().unwrap().baseline,
    Some(Baseline::Filesystem { root, .. }) if root == external
  ));
}

#[test]
fn representative_invalid_suites_have_actionable_diagnostics() {
  let cases = [
    (
      MINIMAL_SUITE.replace("width = 1280", "widht = 1280"),
      "did you mean `width`?",
    ),
    (
      MINIMAL_SUITE.replace(
        "screenshot = { name = \"connected\" }",
        "screenshot = { name = \"connected\" }\nclick = { target = \"item\" }",
      ),
      "step must contain exactly one action",
    ),
    (
      MINIMAL_SUITE.replace("scenario_timeout = \"10s\"", "scenario_timeout = \"1s\""),
      "step timeout may not exceed the scenario timeout",
    ),
    (
      MINIMAL_SUITE.replace("threshold = 0.1", "threshold = 1.0000000000000000001"),
      "decimal must be from 0 through 1",
    ),
    (
      MINIMAL_SUITE.replace("target = \"macos\"", "target = \"windows\""),
      "unknown variant",
    ),
    (
      MINIMAL_SUITE.replace("4aac8ca0-af3d-409e-958e-62954e6cb3d1", "not-a-uuid"),
      "alias value must be a UUID",
    ),
    (
      MINIMAL_SUITE.replace("key = \"Enter\"", "key = \"Enter-Key\""),
      "key must be a Unity Input System Key enum name",
    ),
  ];
  for (source, expected) in cases {
    let error = Fixture::new(&source).load().unwrap_err().to_string();
    assert!(
      error.contains(expected),
      "expected {expected:?} in {error:?}"
    );
    assert!(error.contains(".toml:"), "{error}");
    assert!(error.contains('['), "{error}");
  }
}

#[test]
fn suites_and_scenarios_enforce_fixed_collection_limits() {
  let prefix = MINIMAL_SUITE.split("[[scenarios]]").next().unwrap();
  let scenarios = (0..129)
    .map(|index| {
      format!(
        "[[scenarios]]\nname = \"scenario {index}\"\n[[scenarios.steps]]\nscreenshot = {{ name = \"screen\" }}\n"
      )
    })
    .collect::<String>();
  let error = Fixture::new(&format!("{prefix}{scenarios}"))
    .load()
    .unwrap_err()
    .to_string();
  assert!(error.contains("at most 128 scenarios"), "{error}");

  let mut steps = String::new();
  for index in 0..129 {
    steps.push_str(&format!(
      "[[scenarios.steps]]\nscreenshot = {{ name = \"screen-{index}\" }}\n"
    ));
  }
  let scenario_prefix =
    format!("{prefix}[[scenarios]]\nname = \"many steps\"\nmotion = \"controlled\"\n");
  let error = Fixture::new(&format!("{scenario_prefix}{steps}"))
    .load()
    .unwrap_err()
    .to_string();
  assert!(error.contains("1 through 128 steps"), "{error}");
}

#[test]
fn scenario_names_steps_checkpoints_keys_and_videos_are_bounded_and_balanced() {
  let cases = [
    (
      MINIMAL_SUITE.replace(
        "name = \"connected scene\"",
        &format!("name = {:?}", "x".repeat(129)),
      ),
      "name must contain 1 through 128 UTF-8 bytes",
    ),
    (
      format!(
        "{MINIMAL_SUITE}\n[[scenarios]]\nname = \"connected scene\"\n[[scenarios.steps]]\nscreenshot = {{ name = \"again\" }}\n"
      ),
      "duplicate scenario name",
    ),
    (
      MINIMAL_SUITE.replace(
        "screenshot = { name = \"connected\" }",
        "screenshot = { name = \"connected\" }\n\n[[scenarios.steps]]\nscreenshot = { name = \"connected\" }",
      ),
      "duplicate screenshot checkpoint",
    ),
    (
      MINIMAL_SUITE.replace(
        "screenshot = { name = \"connected\" }",
        "key = { key = \"Enter\", action = \"down\" }",
      ),
      "keys remain held",
    ),
    (
      MINIMAL_SUITE.replace(
        "screenshot = { name = \"connected\" }",
        "video = { action = \"start\", name = \"clip\" }",
      ),
      "has no stop step",
    ),
    (
      MINIMAL_SUITE.replace(
        "screenshot = { name = \"connected\" }",
        "video = { action = \"stop\" }",
      ),
      "video stop has no matching start",
    ),
  ];
  for (source, expected) in cases {
    let error = Fixture::new(&source).load().unwrap_err().to_string();
    assert!(
      error.contains(expected),
      "expected {expected:?} in {error:?}"
    );
  }
}

#[test]
fn targets_waits_profiles_and_baselines_reject_cross_field_errors() {
  let cases = [
    (
      MINIMAL_SUITE.replace("[0.25, 0.75]", "[1.25, 0.75]"),
      "input coordinates must be finite",
    ),
    (
      MINIMAL_SUITE.replace("motion = \"controlled\"", "motion = \"real-time\""),
      "frame wait requires controlled scenario motion",
    ),
    (
      MINIMAL_SUITE.replace(
        "display = { width = 1280, height = 720, scale = 1.0 }",
        "display = { width = 1280, height = 720, scale = 0.0 }",
      ),
      "display dimensions and scale must be finite and positive",
    ),
    (
      MINIMAL_SUITE.replace(
        "display = { width = 1280, height = 720, scale = 1.0 }",
        "display = { width = 1280, height = 720, scale = 1.0 }\ndevice = \"iPhone\"",
      ),
      "profile contains fields for a different target",
    ),
    (
      MINIMAL_SUITE.replace("namespace = \"valid/name\"", "namespace = \"bad//name\""),
      "namespace contains an invalid segment",
    ),
  ];
  for (source, expected) in cases {
    let error = Fixture::new(&source).load().unwrap_err().to_string();
    assert!(
      error.contains(expected),
      "expected {expected:?} in {error:?}"
    );
  }
}

#[cfg(unix)]
#[test]
fn player_paths_reject_symlink_and_lexical_repository_escapes() {
  use std::os::unix::fs::symlink;

  let lexical =
    Fixture::new(&MINIMAL_SUITE.replace("unity_project = \".\"", "unity_project = \"../outside\""));
  assert!(
    lexical
      .load()
      .unwrap_err()
      .to_string()
      .contains("path escapes repository root")
  );

  let fixture =
    Fixture::new(&MINIMAL_SUITE.replace("unity_project = \".\"", "unity_project = \"escape\""));
  let outside = tempfile::tempdir().unwrap();
  symlink(outside.path(), fixture.root.join("escape")).unwrap();
  assert!(
    fixture
      .load()
      .unwrap_err()
      .to_string()
      .contains("path escapes repository root")
  );
}

struct Fixture {
  _temporary: tempfile::TempDir,
  #[cfg(unix)]
  root: PathBuf,
  config: PathBuf,
}

impl Fixture {
  fn new(source: &str) -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("repo");
    fs::create_dir_all(root.join("Assets/Scenes")).unwrap();
    fs::create_dir_all(root.join("rules")).unwrap();
    fs::write(root.join("Assets/Scenes/Game.unity"), "").unwrap();
    fs::write(root.join("rules/Cargo.toml"), "[package]\nname='rules'\n").unwrap();
    let config = root.join("ditto.toml");
    fs::write(&config, source).unwrap();
    assert!(
      std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&root)
        .status()
        .unwrap()
        .success()
    );
    Self {
      _temporary: temporary,
      #[cfg(unix)]
      root,
      config,
    }
  }

  fn load(&self) -> anyhow::Result<battlement_ditto::config::model::Suite> {
    config::load(Some(&self.config))
  }
}

const MINIMAL_SUITE: &str = r#"name = "minimal"
default_profile = "macos-local"

[timeouts]
run = "30s"

[defaults]
step_timeout = "2s"
scenario_timeout = "10s"
motion = "controlled"

[defaults.comparison]
threshold = 0.1

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[aliases]
item = "4aac8ca0-af3d-409e-958e-62954e6cb3d1"

[baseline]
kind = "r2"
namespace = "valid/name"
public_base_url = "https://example.invalid"
account_id_env = "ACCOUNT_ID"
bucket_env = "BUCKET"
access_key_id_env = "ACCESS_KEY_ID"
secret_access_key_env = "SECRET_ACCESS_KEY"

[profiles.macos-local]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[[scenarios]]
name = "connected scene"
motion = "controlled"

[[scenarios.steps]]
click = { target = [0.25, 0.75] }

[[scenarios.steps]]
wait = { frames = 1 }

[[scenarios.steps]]
key = { key = "Enter", action = "tap" }

[[scenarios.steps]]
screenshot = { name = "connected" }
"#;

const R2_BASELINE: &str = r#"[baseline]
kind = "r2"
namespace = "valid/name"
public_base_url = "https://example.invalid"
account_id_env = "ACCOUNT_ID"
bucket_env = "BUCKET"
access_key_id_env = "ACCESS_KEY_ID"
secret_access_key_env = "SECRET_ACCESS_KEY"
"#;

const FULL_SUITE: &str = r#"name = "complete"
default_profile = "macos-local"

[timeouts]
run = "1m"

[defaults]
step_timeout = "750ms"
scenario_timeout = "12s"

[defaults.comparison]
threshold = 0.10000000000000001
anti_alias = false

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[aliases]
item = "4aac8ca0-af3d-409e-958e-62954e6cb3d1"
other = "d1bbd0ad-fcb7-48d7-b409-d221adc9eac6"

[baseline]
kind = "r2"
namespace = "battlement/complete"
public_base_url = "https://example.invalid"
account_id_env = "ACCOUNT_ID"
bucket_env = "BUCKET"
access_key_id_env = "ACCESS_KEY_ID"
secret_access_key_env = "SECRET_ACCESS_KEY"

[profiles.macos-local]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[profiles.web-ci]
target = "webgl"
display = { width = 1280, height = 720, scale = 1.0 }
headless_command = ["chromium", "--headless", "{url}"]

[profiles.iphone-ci]
target = "ios-simulator"
device = "iPhone 16 Pro"
orientation = "portrait"

[[scenarios]]
name = "all step shapes"
motion = "controlled"

[[scenarios.steps]]
name = "click alias"
click = { target = "item" }

[[scenarios.steps]]
click = { target = [0.5, 0.75] }

[[scenarios.steps]]
hover = { target = "other" }

[[scenarios.steps]]
drag = { from = "item", to = [0.75, 0.75] }

[[scenarios.steps]]
key = { key = "Enter", action = "down" }

[[scenarios.steps]]
key = { key = "Enter", action = "up" }

[[scenarios.steps]]
wait = { frames = 3 }

[[scenarios.steps]]
timeout = "500ms"
wait = { object = "item", state = "visible" }

[[scenarios.steps]]
assert = { object = "other", state = "enabled" }

[[scenarios.steps]]
screenshot = { name = "strict", threshold = 0.0500, max_changed_percent = 0.0 }

[[scenarios.steps]]
video = { action = "start", name = "move", motion = "real-time", max_duration = "5s" }

[[scenarios.steps]]
click = { target = "other" }

[[scenarios.steps]]
video = { action = "stop" }
"#;
