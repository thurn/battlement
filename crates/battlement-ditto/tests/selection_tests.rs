use std::{fs, path::PathBuf, process::Command};

use battlement_ditto::{
  config::{self, FragmentInput},
  selection::{self, Disposition, Options},
};

#[test]
fn every_target_materializes_capability_skips_in_stable_run_order() {
  let fixture = Fixture::new(SUITE);
  let suite = fixture.load();
  let cases = [
    ("macos-local", vec!["run", "run", "run"]),
    ("web-ci", vec!["run", "unsupported-step:video", "run"]),
    ("iphone-ci", vec!["unsupported-input:hover", "run", "run"]),
  ];
  for (profile, expected) in cases {
    let selection = selection::resolve(
      &suite,
      &Options {
        profile: Some(profile.to_owned()),
        ..Options::default()
      },
    )
    .unwrap();
    let actual: Vec<&str> = selection
      .scenarios
      .iter()
      .map(|scenario| match &scenario.disposition {
        Disposition::Runnable => "run",
        Disposition::Skipped { reason } => reason,
      })
      .collect();
    assert_eq!(actual, expected);
    assert_eq!(
      selection
        .scenarios
        .iter()
        .map(|scenario| scenario.run_index)
        .collect::<Vec<_>>(),
      vec![0, 1, 2]
    );
  }
}

#[test]
fn include_unions_and_excludes_preserve_suite_order_without_duplicates() {
  let suite = Fixture::new(SUITE).load();
  let selection = selection::resolve(
    &suite,
    &Options {
      includes: vec![
        "* case".to_owned(),
        "click*".to_owned(),
        "click*".to_owned(),
      ],
      excludes: vec!["video*".to_owned()],
      ..Options::default()
    },
  )
  .unwrap();
  assert_eq!(
    selection
      .scenarios
      .iter()
      .map(|scenario| (scenario.run_index, scenario.scenario.name.as_str()))
      .collect::<Vec<_>>(),
    vec![(0, "hover case"), (1, "click case")]
  );
}

#[test]
fn selectors_and_empty_results_have_explicit_failure_modes() {
  let suite = Fixture::new(SUITE).load();
  for options in [
    Options {
      includes: vec!["missing*".to_owned()],
      ..Options::default()
    },
    Options {
      excludes: vec!["missing*".to_owned()],
      ..Options::default()
    },
    Options {
      profile: Some("missing".to_owned()),
      ..Options::default()
    },
  ] {
    assert!(selection::resolve(&suite, &options).is_err());
  }
  let excludes = vec!["*".to_owned()];
  assert!(
    selection::resolve(
      &suite,
      &Options {
        excludes: excludes.clone(),
        ..Options::default()
      }
    )
    .is_err()
  );
  assert!(
    selection::resolve(
      &suite,
      &Options {
        excludes,
        allow_empty: true,
        ..Options::default()
      }
    )
    .unwrap()
    .scenarios
    .is_empty()
  );
}

#[test]
fn file_and_standard_input_fragments_inherit_member_by_member() {
  let fixture = Fixture::new(SUITE);
  let base = fixture.load();
  let fragment_file = fixture.temporary.path().join("agent-fragment.toml");
  fs::write(&fragment_file, FRAGMENT).unwrap();
  let file_suite = config::load_fragment(&base, FragmentInput::File(fragment_file), true).unwrap();
  assert_eq!(file_suite.name, "outside fragment");
  assert_eq!(file_suite.player, base.player);
  assert_eq!(file_suite.profiles, base.profiles);
  assert_eq!(file_suite.defaults.step_timeout.as_millis(), 1_000);
  assert_eq!(
    file_suite.defaults.scenario_timeout,
    base.defaults.scenario_timeout
  );
  assert!(!file_suite.defaults.comparison.anti_alias);
  assert_eq!(file_suite.defaults.comparison.threshold.as_str(), "0.1");
  assert_eq!(file_suite.aliases.len(), 2);
  assert!(file_suite.baseline.is_none());

  let stdin_suite = config::load_fragment(
    &base,
    FragmentInput::StandardInput {
      source: FRAGMENT.replace("name = \"outside fragment\"\n", ""),
      name: None,
    },
    false,
  )
  .unwrap();
  assert_eq!(stdin_suite.name, "standard-input");
  assert!(
    config::load_fragment(
      &base,
      FragmentInput::StandardInput {
        source: FRAGMENT.to_owned(),
        name: None,
      },
      true,
    )
    .unwrap_err()
    .to_string()
    .contains("do not support --watch")
  );
}

#[test]
fn fragment_alias_conflicts_and_unknown_fields_are_rejected() {
  let fixture = Fixture::new(SUITE);
  let base = fixture.load();
  for source in [
    FRAGMENT.replace(
      "4aac8ca0-af3d-409e-958e-62954e6cb3d1",
      "d1bbd0ad-fcb7-48d7-b409-d221adc9eac6",
    ),
    FRAGMENT.replace(
      "step_timeout = \"1s\"",
      "step_timeout = \"1s\"\nplayer = {}",
    ),
  ] {
    assert!(
      config::load_fragment(
        &base,
        FragmentInput::StandardInput { source, name: None },
        false,
      )
      .is_err()
    );
  }
}

#[test]
fn a_full_suite_fragment_does_not_inherit_from_the_discovered_suite() {
  let base_fixture = Fixture::new(SUITE);
  let full_fixture = Fixture::new(&SUITE.replace("name = \"matrix\"", "name = \"independent\""));
  let loaded = config::load_fragment(
    &base_fixture.load(),
    FragmentInput::File(full_fixture.config.clone()),
    false,
  )
  .unwrap();
  assert_eq!(loaded.name, "independent");
  assert_eq!(loaded.repository, full_fixture.root.canonicalize().unwrap());
}

#[test]
fn list_prints_selected_checkpoints_and_precise_skip_reasons() {
  let fixture = Fixture::new(SUITE);
  let output = Command::new(env!("CARGO_BIN_EXE_ditto"))
    .args([
      "--config",
      fixture.config.to_str().unwrap(),
      "list",
      "--profile",
      "web-ci",
      "*case",
    ])
    .output()
    .unwrap();
  assert!(output.status.success());
  let stdout = String::from_utf8(output.stdout).unwrap();
  assert!(stdout.contains("* web-ci [webgl]"), "{stdout}");
  assert!(
    stdout.contains("video case [skip: unsupported-step:video]"),
    "{stdout}"
  );
  assert!(stdout.contains("screenshot: video-screen"), "{stdout}");
}

struct Fixture {
  temporary: tempfile::TempDir,
  root: PathBuf,
  config: PathBuf,
}

impl Fixture {
  fn new(source: &str) -> Self {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("repo");
    fs::create_dir_all(root.join("Assets/Scenes")).unwrap();
    fs::create_dir_all(root.join("rules")).unwrap();
    let config = root.join("ditto.toml");
    fs::write(&config, source).unwrap();
    assert!(
      Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(&root)
        .status()
        .unwrap()
        .success()
    );
    Self {
      temporary,
      root,
      config,
    }
  }

  fn load(&self) -> battlement_ditto::config::model::Suite {
    config::load(Some(&self.config)).unwrap()
  }
}

const FRAGMENT: &str = r#"name = "outside fragment"

[defaults]
step_timeout = "1s"

[defaults.comparison]
anti_alias = false

[aliases]
item = "4aac8ca0-af3d-409e-958e-62954e6cb3d1"
extra = "d1bbd0ad-fcb7-48d7-b409-d221adc9eac6"

[[scenarios]]
name = "fragment click"

[[scenarios.steps]]
click = { target = "extra" }
"#;

const SUITE: &str = r#"name = "matrix"
default_profile = "macos-local"

[defaults]
step_timeout = "2s"
scenario_timeout = "10s"
motion = "controlled"

[defaults.comparison]
threshold = 0.1
anti_alias = true
max_changed_percent = 0.01

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[aliases]
item = "4aac8ca0-af3d-409e-958e-62954e6cb3d1"

[profiles.macos-local]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[profiles.web-ci]
target = "webgl"
display = { width = 1280, height = 720, scale = 1.0 }

[profiles.iphone-ci]
target = "ios-simulator"
device = "iPhone 16 Pro"
orientation = "portrait"

[[scenarios]]
name = "hover case"
[[scenarios.steps]]
hover = { target = "item" }
[[scenarios.steps]]
screenshot = { name = "hover-screen" }

[[scenarios]]
name = "video case"
[[scenarios.steps]]
video = { action = "start", name = "clip" }
[[scenarios.steps]]
screenshot = { name = "video-screen" }
[[scenarios.steps]]
video = { action = "stop" }

[[scenarios]]
name = "click case"
[[scenarios.steps]]
click = { target = "item" }
[[scenarios.steps]]
screenshot = { name = "click-screen" }
"#;
