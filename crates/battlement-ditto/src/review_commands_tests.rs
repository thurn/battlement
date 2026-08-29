use std::fs;

use super::select_run;
use crate::wire::{
  common::{StepName, StepStatus},
  job::Motion,
  result::{
    BaselineOutcome, ImageFile, Recovery, ResultCommand, RunResult, RunStatus, ScenarioResult,
    ScenarioStatus, ScenarioTimings, ScreenshotResult, StepResult,
  },
  run_storage::RunStore,
};

const MISMATCH: &str = "35ad217e-1ab0-4aa6-b08b-7f8700872874";
const CAPTURE: &str = "1f8e27d5-fc27-4f42-85cc-c80e442ba8c4";
const OTHER: &str = "24bf2160-306f-44c1-b1bb-fd69cc4bc329";

#[test]
fn implicit_selection_prefers_a_suite_mismatch_over_a_newer_capture() {
  let temporary = tempfile::tempdir().unwrap();
  let repository = temporary.path().join("repository");
  fs::create_dir(&repository).unwrap();
  let mut store = RunStore::open(temporary.path().join("runs")).unwrap();
  store_run(
    &mut store,
    &repository,
    "review suite",
    MISMATCH,
    ResultCommand::Run,
    true,
    1,
  );
  store_run(
    &mut store,
    &repository,
    "review suite",
    CAPTURE,
    ResultCommand::Capture,
    false,
    2,
  );
  store_run(
    &mut store,
    &repository,
    "other suite",
    OTHER,
    ResultCommand::Run,
    true,
    3,
  );

  assert_eq!(
    select_run_for(&store, &repository, "review suite", None),
    MISMATCH
  );
  assert_eq!(
    select_run_for(&store, &repository, "review suite", Some(CAPTURE)),
    CAPTURE
  );
}

fn select_run_for(
  store: &RunStore,
  repository: &std::path::Path,
  suite_name: &str,
  requested: Option<&str>,
) -> String {
  let suite = test_suite(repository, suite_name);
  select_run(store, &suite, requested).unwrap()
}

fn store_run(
  store: &mut RunStore,
  repository: &std::path::Path,
  suite: &str,
  run_id: &str,
  command: ResultCommand,
  missing: bool,
  now: u64,
) {
  let mut stderr = Vec::new();
  let mut active = store
    .begin(empty_result(run_id, command), &mut stderr, now)
    .unwrap();
  store
    .index_identity(&active, repository, suite, now)
    .unwrap();
  fs::create_dir_all(active.path().join("images")).unwrap();
  fs::write(active.path().join("images/actual.png"), b"actual").unwrap();
  store
    .finalize(
      &mut active,
      image_result(run_id, command, missing, suite),
      now,
    )
    .unwrap();
}

fn empty_result(run_id: &str, command: ResultCommand) -> RunResult {
  RunResult {
    run_id: run_id.to_owned(),
    source_run_id: None,
    lock_sha256: None,
    command,
    source_command: None,
    cycle: 1,
    suite: None,
    profile: None,
    started_at: "2026-08-29T10:00:00Z".to_owned(),
    duration_ms: 0,
    status: RunStatus::Passed,
    exit_code: 0,
    build: None,
    phases: vec![],
    player_sessions: vec![],
    jobs: vec![],
    scenarios: vec![],
    warnings: vec![],
    errors: vec![],
    baseline_writes: vec![],
    artifacts: vec![],
  }
}

fn image_result(run_id: &str, command: ResultCommand, missing: bool, suite: &str) -> RunResult {
  let failed = missing && command == ResultCommand::Run;
  RunResult {
    suite: Some(suite.to_owned()),
    profile: Some("macos-local".to_owned()),
    duration_ms: 10,
    status: if failed {
      RunStatus::Failed
    } else {
      RunStatus::Passed
    },
    exit_code: u8::from(failed),
    scenarios: vec![ScenarioResult {
      id: "a790583b-e0a4-4f70-85d5-827cc91ff442".to_owned(),
      name: "menu".to_owned(),
      status: if failed {
        ScenarioStatus::Failed
      } else {
        ScenarioStatus::Passed
      },
      status_reason: None,
      motion: Motion::Instant,
      duration_ms: 10,
      expired_deadline: None,
      timings: ScenarioTimings::default(),
      steps: vec![StepResult {
        index: 0,
        name: None,
        kind: StepName::Screenshot,
        status: if failed {
          StepStatus::Failed
        } else {
          StepStatus::Passed
        },
        status_reason: None,
        duration_ms: 5,
        expired_deadline: None,
        error_ids: vec![],
        assertion: None,
        screenshot: Some(ScreenshotResult::Captured {
          checkpoint: "ready".to_owned(),
          actual: ImageFile {
            path: "images/actual.png".to_owned(),
            sha256: "a".repeat(64),
            width: 1,
            height: 1,
          },
          baseline: if command == ResultCommand::Capture {
            BaselineOutcome::NotLoaded
          } else {
            BaselineOutcome::Missing
          },
          comparison: None,
          matched_before_update: None,
          updated: None,
        }),
        video: None,
      }],
      logs: None,
      failure_frame: None,
      recovery: Recovery::None,
    }],
    ..empty_result(run_id, command)
  }
}

fn test_suite(repository: &std::path::Path, name: &str) -> crate::config::model::Suite {
  use crate::config::{
    model::{Defaults, Player, Suite, Timeouts},
    value::{DurationValue, ExactDecimal},
  };

  Suite {
    source: repository.join("ditto.toml"),
    repository: repository.to_owned(),
    name: name.to_owned(),
    default_profile: "macos-local".to_owned(),
    player: Player {
      unity_project: repository.to_owned(),
      scene: repository.join("Game.unity"),
      rust_manifest: repository.join("Cargo.toml"),
    },
    timeouts: Timeouts {
      run: DurationValue::parse("1s").unwrap(),
      build: DurationValue::parse("1s").unwrap(),
      launch: DurationValue::parse("1s").unwrap(),
      baseline_download: DurationValue::parse("1s").unwrap(),
      simulator_boot: DurationValue::parse("1s").unwrap(),
    },
    defaults: Defaults {
      step_timeout: DurationValue::parse("1s").unwrap(),
      scenario_timeout: DurationValue::parse("1s").unwrap(),
      motion: crate::config::model::Motion::Instant,
      comparison: crate::config::model::Comparison {
        threshold: ExactDecimal::parse("0.1", "0"..="1").unwrap(),
        anti_alias: true,
        max_changed_percent: ExactDecimal::parse("0", "0"..="100").unwrap(),
      },
    },
    aliases: Default::default(),
    baseline: None,
    profiles: Default::default(),
    scenarios: vec![],
  }
}
