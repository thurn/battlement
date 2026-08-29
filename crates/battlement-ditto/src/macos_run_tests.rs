use std::{fs, process::Command};

use crate::{
  config,
  selection::{self, Options},
  wire::result::{ResultCommand, RunResult, RunStatus},
};

use super::{baseline_inputs, reduce_status, selection_has_screenshots};

#[test]
fn startup_infrastructure_failure_survives_empty_scenario_reduction() {
  let mut result = empty_result();
  result.status = RunStatus::InfrastructureError;
  result.exit_code = 2;

  reduce_status(&mut result);

  assert_eq!(result.status, RunStatus::InfrastructureError);
  assert_eq!(result.exit_code, 2);
}

#[test]
fn assertion_only_selection_never_reads_the_baseline_lock() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  fs::create_dir_all(repository.join("Assets/Scenes")).unwrap();
  fs::create_dir_all(repository.join("ProjectSettings")).unwrap();
  fs::create_dir_all(repository.join("rules/src")).unwrap();
  fs::write(repository.join("Assets/Scenes/Game.unity"), "").unwrap();
  fs::write(
    repository.join("ProjectSettings/ProjectVersion.txt"),
    "m_EditorVersion: 6000.0.56f1\n",
  )
  .unwrap();
  fs::write(
    repository.join("rules/Cargo.toml"),
    "[package]\nname='fixture'\nversion='0.1.0'\n",
  )
  .unwrap();
  fs::write(repository.join("ditto.toml"), SUITE).unwrap();
  fs::write(repository.join("ditto.lock"), "not valid json\n").unwrap();
  assert!(
    Command::new("git")
      .args(["init", "--quiet"])
      .current_dir(&repository)
      .status()
      .unwrap()
      .success()
  );

  let suite = config::load(Some(&repository.join("ditto.toml"))).unwrap();
  let selection = selection::resolve(&suite, &Options::default()).unwrap();
  assert!(!selection_has_screenshots(&selection));
  let baseline = baseline_inputs(&suite, ResultCommand::Run, false).unwrap();
  assert!(baseline.manifest.is_none());
  assert!(baseline.store.is_none());
  assert!(baseline.lock_sha256.is_none());
  assert!(baseline_inputs(&suite, ResultCommand::Run, true).is_err());
}

fn empty_result() -> RunResult {
  RunResult {
    run_id: "0197b35f-6c59-7b98-b1f0-a39f5ee54db8".to_owned(),
    source_run_id: None,
    lock_sha256: None,
    command: ResultCommand::Run,
    source_command: None,
    cycle: 1,
    suite: Some("fixture".to_owned()),
    profile: Some("macos-local".to_owned()),
    started_at: "2026-08-29T10:00:00Z".to_owned(),
    duration_ms: 0,
    status: RunStatus::Passed,
    exit_code: 0,
    build: None,
    phases: Vec::new(),
    player_sessions: Vec::new(),
    jobs: Vec::new(),
    scenarios: Vec::new(),
    warnings: Vec::new(),
    errors: Vec::new(),
    baseline_writes: Vec::new(),
    artifacts: Vec::new(),
  }
}

const SUITE: &str = r#"name = "fixture"
default_profile = "macos-local"

[player]
unity_project = "."
scene = "Assets/Scenes/Game.unity"
rust_manifest = "rules/Cargo.toml"

[baseline]
kind = "filesystem"
namespace = "fixture"
root = "baselines"

[profiles.macos-local]
target = "macos"
display = { width = 1280, height = 720, scale = 1.0 }

[[scenarios]]
name = "assertion only"

[[scenarios.steps]]
assert = { object = "00000000-0000-0000-0000-000000000001", state = "exists" }
"#;
