use std::{
  process::{Command, Stdio},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use battlement_ditto::{
  macos_capture,
  wire::{
    common::ErrorCode,
    result::{PhaseName, PhaseStatus, RunResult, RunStatus, ScenarioStatus},
  },
};
use serde_json::json;

use crate::{
  CAPTURE_TEST_GATE,
  macos_fixture::{self, FixtureBuild, FixtureLauncher, PassMaterializer},
};

#[test]
fn shutdown_and_log_failures_keep_execution_facts_and_reap_the_player() {
  let _guard = CAPTURE_TEST_GATE.lock().unwrap();
  for mode in [
    "complete-ignore-shutdown",
    "complete-no-log",
    "complete-log-conflict",
    "reject-ignore-shutdown",
  ] {
    let build = FixtureBuild::new(true);
    let run = tempfile::tempdir().unwrap();
    let rejected = mode == "reject-ignore-shutdown";
    let log_failure = matches!(mode, "complete-no-log" | "complete-log-conflict");
    let launcher = FixtureLauncher::new(
      run.path(),
      if rejected {
        json!({"capture_adapter": "rejected-adapter"})
      } else {
        json!({})
      },
      mode,
    );
    let mut input = macos_fixture::request(&build.handle, run.path(), 1);
    input.timeouts.shutdown = Duration::from_millis(150);
    let outcome = macos_capture::capture_macos(
      input,
      &launcher,
      Arc::new(PassMaterializer),
      &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
      outcome.exit_code,
      if rejected || log_failure { 2 } else { 0 }
    );
    let session = outcome.player_session.as_ref().unwrap();
    assert_eq!(session.accepted, !rejected);
    assert!(session.startup_report.is_some());
    assert!(outcome.player_exit.is_some());
    let cleanup = outcome
      .phases
      .iter()
      .find(|p| p.name == PhaseName::Cleanup)
      .unwrap();
    assert_eq!(cleanup.status, PhaseStatus::Passed);
    if mode.ends_with("ignore-shutdown") {
      assert!(cleanup.duration_ms >= 150);
      assert_eq!(outcome.player_exit.unwrap().code, None);
    }
    if rejected {
      assert!(outcome.orchestration.scenarios.is_empty());
      assert_eq!(
        outcome
          .phases
          .iter()
          .find(|p| p.name == PhaseName::Startup)
          .unwrap()
          .status,
        PhaseStatus::Failed
      );
    } else {
      assert_eq!(outcome.orchestration.scenarios.len(), 1);
      assert_eq!(
        outcome.orchestration.scenarios[0].status,
        ScenarioStatus::Passed
      );
      assert_eq!(
        outcome
          .phases
          .iter()
          .find(|p| p.name == PhaseName::Scenarios)
          .unwrap()
          .status,
        PhaseStatus::Passed
      );
    }
    if log_failure {
      assert!(session.diagnostic_paths.is_empty());
      assert_eq!(outcome.errors.len(), 1);
      assert_eq!(outcome.errors[0].code, ErrorCode::DurabilityFailed);
      assert!(
        outcome
          .phases
          .iter()
          .any(|p| p.name == PhaseName::Durability && p.status == PhaseStatus::Failed)
      );
    } else {
      assert_eq!(session.diagnostic_paths.len(), 1);
      assert!(outcome.errors.is_empty());
    }
    let mut result = macos_fixture::empty_result();
    outcome.apply_to(&mut result);
    result.artifacts = session.diagnostic_paths.clone();
    if !rejected {
      result.artifacts.push("logs/events.jsonl".to_owned());
    }
    result.artifacts.sort();
    let encoded = result.to_canonical_json().unwrap();
    let retained: RunResult = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
      retained.status,
      if rejected || log_failure {
        RunStatus::InfrastructureError
      } else {
        RunStatus::Passed
      }
    );
    #[cfg(unix)]
    assert!(
      !Command::new("kill")
        .args(["-0", &launcher.pid.load(Ordering::SeqCst).to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
    );
  }
}
