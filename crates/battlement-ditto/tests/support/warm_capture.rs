use std::{
  fs,
  process::{Command, Stdio},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use anyhow::Result;
use serde_json::json;

use crate::{
  macos_fixture::{self, FixtureBuild, FixtureLauncher, PassMaterializer},
  macos_run::{self, WatchRuntime},
  macos_watch_capture::WarmMacosPlayer,
  run_errors::RunErrors,
  scenario_orchestration::{DecisionFailure, MaterializedScenario, ScenarioMaterializer},
  wire::{
    common::{DeadlineKind, ErrorCode, ErrorSource, StepStatus},
    job::Job,
    lifecycle::ScenarioComplete,
    result::{
      ErrorOccurrence, JobStatus, PhaseName, PhaseStatus, Recovery, RunResult, RunStatus,
      ScenarioStatus, ScenarioTimings,
    },
  },
};

#[test]
fn retained_player_handshakes_keep_identity_logs_and_independent_cleanup() {
  for mode in [
    "warm-timeout",
    "warm-exit",
    "warm-cancel",
    "warm-before-timeout",
    "warm-before-cancel",
    "warm-complete",
  ] {
    let build = FixtureBuild::new(true);
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let launcher = FixtureLauncher::new(first.path(), json!({}), mode);
    let interrupted = Arc::new(AtomicBool::new(false));
    let mut initial = macos_fixture::request(&build.handle, first.path(), 1);
    initial.timeouts.startup = Duration::from_millis(500);
    let native_execution = initial.native_execution.clone();
    let launched =
      WarmMacosPlayer::launch(initial, &launcher, Arc::new(PassMaterializer), &interrupted)
        .unwrap();
    assert_eq!(launched.outcome.exit_code, 0, "{mode}");
    let original = launched.outcome.player_session.unwrap();
    assert!(original.accepted);
    assert!(original.startup_report.is_some());
    let mut runtime = WatchRuntime {
      player: launched.player,
      player_fingerprint: Some(build.handle.metadata().identity.fingerprint.clone()),
      odiff: Default::default(),
    };
    let mut next = macos_fixture::request(&build.handle, second.path(), 1);
    next.job.profile.native_execution_id = Some(native_execution.id().to_owned());
    next.requirements.native_execution_id = Some(native_execution.id().to_owned());
    next.native_execution = native_execution;
    let mut selected = launched.outcome.orchestration.scenarios[0].clone();
    selected.id = next.job.scenarios[0].id.clone();
    selected.status = ScenarioStatus::NotRun;
    selected.status_reason = Some("run-infrastructure-error".to_owned());
    selected.duration_ms = 0;
    selected.timings = ScenarioTimings::default();
    selected.logs = None;
    selected.recovery = Recovery::None;
    for step in &mut selected.steps {
      step.status = StepStatus::NotRun;
      step.status_reason = selected.status_reason.clone();
      step.duration_ms = 0;
    }
    let signal = mode.ends_with("cancel").then(|| {
      let signal = interrupted.clone();
      let marker = first.path().join("setup-warm");
      thread::spawn(move || {
        let started = Instant::now();
        while !marker.exists() {
          assert!(started.elapsed() < Duration::from_secs(5));
          thread::sleep(Duration::from_millis(5));
        }
        signal.store(true, Ordering::Release);
      })
    });
    let outcome = runtime
      .capture(next, Arc::new(PassMaterializer), &interrupted)
      .unwrap();
    if let Some(signal) = signal {
      signal.join().unwrap();
    }
    assert_eq!(launcher.count.load(Ordering::SeqCst), 1);
    let session = outcome.player_session.as_ref().unwrap();
    assert_eq!(session.player_session_id, original.player_session_id);
    assert_eq!(session.startup_report, original.startup_report);
    assert!(session.accepted);
    let diagnostic = session.diagnostic_paths[0].clone();
    assert_eq!(
      fs::read_to_string(second.path().join(&diagnostic)).unwrap(),
      "fixture player log\n"
    );
    assert!(
      !outcome
        .phases
        .iter()
        .any(|phase| phase.name == PhaseName::Launch)
    );
    let startup = &outcome.phases[0];
    assert_eq!(startup.name, PhaseName::Startup);
    if mode == "warm-complete" {
      assert_eq!(outcome.exit_code, 0);
      assert_eq!(startup.status, PhaseStatus::Passed);
      assert!(runtime.player.as_mut().unwrap().is_alive().unwrap());
      assert_eq!(outcome.orchestration.scenarios.len(), 1);
    } else {
      assert!(
        runtime.player.is_none(),
        "failed session retained for {mode}"
      );
      assert!(runtime.player_fingerprint.is_none());
      assert!(outcome.orchestration.jobs.is_empty());
      assert_eq!(startup.log_path.as_deref(), Some(diagnostic.as_str()));
      assert_eq!(outcome.phases[1].name, PhaseName::Cleanup);
      assert_eq!(outcome.phases[1].status, PhaseStatus::Passed);
      assert_eq!(
        startup.expired_deadline,
        mode.ends_with("timeout").then_some(DeadlineKind::Startup)
      );
      if mode.ends_with("cancel") {
        assert_eq!(outcome.exit_code, 130);
        assert_eq!(startup.status, PhaseStatus::Interrupted);
        assert!(outcome.errors.is_empty());
      } else {
        assert_eq!(outcome.exit_code, 2);
        assert_eq!(startup.status, PhaseStatus::Failed);
        assert_eq!(
          outcome.errors[0].code,
          if mode.ends_with("exit") {
            ErrorCode::RuntimeProcessExit
          } else {
            ErrorCode::DeadlineExpired
          }
        );
      }
      if mode.ends_with("exit") {
        assert_eq!(outcome.player_exit.unwrap().code, Some(7));
      }
    }
    let mut result = macos_fixture::empty_result();
    result.scenarios = vec![selected];
    macos_run::apply_capture(&mut result, outcome);
    result.artifacts = vec![diagnostic];
    if mode == "warm-complete" {
      result.artifacts.push("logs/events.jsonl".to_owned());
    }
    result.artifacts.sort();
    let bytes = result.to_canonical_json().unwrap();
    fs::write(second.path().join("result.json"), &bytes).unwrap();
    let retained: RunResult = serde_json::from_slice(&bytes).unwrap();
    if mode != "warm-complete" {
      assert_eq!(retained.scenarios[0].status, ScenarioStatus::NotRun);
      assert_eq!(
        retained.status,
        if mode.ends_with("cancel") {
          RunStatus::Interrupted
        } else {
          RunStatus::InfrastructureError
        }
      );
    }
    drop(runtime);
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

struct FailedComparison {
  errors: Arc<RunErrors>,
}

impl ScenarioMaterializer for FailedComparison {
  fn materialize(
    &self,
    job: &Job,
    complete: &ScenarioComplete,
    recovery: Recovery,
  ) -> Result<MaterializedScenario> {
    let mut result = PassMaterializer.materialize(job, complete, recovery)?;
    if complete.scenario_id == job.scenarios[0].id {
      let id = self.errors.record(ErrorOccurrence {
        id: String::new(),
        code: ErrorCode::ImageComparisonFailed,
        source: ErrorSource::Ditto,
        message: "fixture comparison failed".to_owned(),
        job_id: Some(job.job_id.clone()),
        player_session_id: None,
        scenario_id: Some(complete.scenario_id.clone()),
        step_index: Some(0),
        log_sequence: None,
      });
      result.result.status = ScenarioStatus::Failed;
      result.result.steps[0].status = StepStatus::Failed;
      result.result.steps[0].error_ids.push(id.clone());
      result.primary_failure = Some(DecisionFailure {
        error_id: id,
        code: ErrorCode::ImageComparisonFailed,
        message: "fixture comparison failed".to_owned(),
      });
    }
    Ok(result)
  }
}

#[test]
fn dispatched_warm_losses_preserve_committed_partial_and_unstarted_facts() {
  for mode in [
    "warm-job-exit",
    "warm-job-timeout",
    "warm-job-between",
    "warm-job-cancel",
  ] {
    let build = FixtureBuild::new(true);
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let launcher = FixtureLauncher::new(first.path(), json!({}), mode);
    let interrupted = Arc::new(AtomicBool::new(false));
    let initial = macos_fixture::request(&build.handle, first.path(), 1);
    let native_execution = initial.native_execution.clone();
    let launched =
      WarmMacosPlayer::launch(initial, &launcher, Arc::new(PassMaterializer), &interrupted)
        .unwrap();
    assert_eq!(launched.outcome.exit_code, 0);
    let original = launched.outcome.player_session.unwrap();
    let mut runtime = WatchRuntime {
      player: launched.player,
      player_fingerprint: Some(build.handle.metadata().identity.fingerprint.clone()),
      odiff: Default::default(),
    };
    let mut next = macos_fixture::request(&build.handle, second.path(), 3);
    next.job.profile.native_execution_id = Some(native_execution.id().to_owned());
    next.requirements.native_execution_id = Some(native_execution.id().to_owned());
    next.native_execution = native_execution;
    next.job.remaining_run_timeout_ms = 500;
    let mut extra = next.job.scenarios[1].steps[0].clone();
    extra.index = 1;
    next.job.scenarios[1].steps.push(extra);
    let errors = next.errors.clone();
    let signal = (mode == "warm-job-cancel").then(|| {
      let signal = interrupted.clone();
      let marker = first.path().join("setup-dispatched");
      thread::spawn(move || {
        let started = Instant::now();
        while !marker.exists() {
          assert!(started.elapsed() < Duration::from_secs(5));
          thread::sleep(Duration::from_millis(5));
        }
        signal.store(true, Ordering::Release);
      })
    });
    let outcome = runtime
      .capture(
        next,
        Arc::new(FailedComparison {
          errors: errors.clone(),
        }),
        &interrupted,
      )
      .unwrap();
    if let Some(signal) = signal {
      signal.join().unwrap();
    }
    assert!(runtime.player.is_none());
    assert!(runtime.player_fingerprint.is_none());
    assert_eq!(launcher.count.load(Ordering::SeqCst), 1);
    let session = outcome.player_session.as_ref().unwrap();
    assert_eq!(session.player_session_id, original.player_session_id);
    assert_eq!(session.startup_report, original.startup_report);
    assert!(session.accepted);
    let scenario_phase = outcome
      .phases
      .iter()
      .find(|phase| phase.name == PhaseName::Scenarios)
      .unwrap();
    assert_eq!(
      outcome.orchestration.scenarios[0].status,
      ScenarioStatus::Failed
    );
    assert_eq!(
      outcome.orchestration.scenarios[0].steps[0].error_ids,
      ["E0001"]
    );
    if mode == "warm-job-cancel" {
      assert_eq!(outcome.exit_code, 130);
      assert_eq!(scenario_phase.status, PhaseStatus::Interrupted);
      continue;
    }
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(scenario_phase.status, PhaseStatus::Failed);
    assert_eq!(scenario_phase.error_ids, ["E0002"]);
    assert_eq!(
      scenario_phase.expired_deadline,
      (mode == "warm-job-timeout").then_some(DeadlineKind::Run)
    );
    assert_eq!(outcome.errors.len(), 2, "{mode}: {:?}", outcome.errors);
    assert_eq!(
      outcome.errors[1].code,
      if mode == "warm-job-timeout" {
        ErrorCode::DeadlineExpired
      } else {
        ErrorCode::RuntimeProcessExit
      }
    );
    assert_eq!(
      outcome.orchestration.jobs[0].status,
      JobStatus::InfrastructureError
    );
    assert_eq!(outcome.orchestration.scenarios.len(), 3);
    let partial = &outcome.orchestration.scenarios[1];
    assert_eq!(partial.status, ScenarioStatus::InfrastructureError);
    assert_eq!(partial.steps[0].status, StepStatus::Passed);
    assert_eq!(partial.steps[0].duration_ms, 3);
    assert_eq!(
      partial.steps[1].status,
      if mode == "warm-job-between" {
        StepStatus::NotRun
      } else {
        StepStatus::InfrastructureError
      }
    );
    assert!(!partial.logs.as_ref().unwrap().complete);
    assert_eq!(
      outcome.orchestration.scenarios[2].status,
      ScenarioStatus::NotRun
    );
    assert!(outcome.orchestration.pending_recovery.is_none());
    assert!(
      outcome
        .phases
        .iter()
        .any(|phase| phase.name == PhaseName::Cleanup && phase.status == PhaseStatus::Passed)
    );
    assert!(
      !outcome
        .phases
        .iter()
        .any(|phase| phase.name == PhaseName::Durability && phase.status == PhaseStatus::Failed)
    );
    let diagnostic = session.diagnostic_paths[0].clone();
    assert_eq!(
      fs::read_to_string(second.path().join(&diagnostic)).unwrap(),
      "fixture player log\n"
    );
    let mut result = macos_fixture::empty_result();
    outcome.apply_to(&mut result);
    result.artifacts = vec![
      "logs/events.jsonl".to_owned(),
      diagnostic,
      "orchestration.json".to_owned(),
    ];
    let bytes = result.to_canonical_json().unwrap();
    let retained: RunResult = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(retained.status, RunStatus::InfrastructureError);
    let checkpoint: serde_json::Value =
      serde_json::from_slice(&fs::read(second.path().join("orchestration.json")).unwrap()).unwrap();
    assert_eq!(checkpoint["scenarios"].as_array().unwrap().len(), 3);
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
