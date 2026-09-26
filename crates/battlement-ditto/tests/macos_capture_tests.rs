use std::{
  fs,
  process::{Command, Stdio},
  sync::{
    Arc, Barrier, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use battlement_ditto::{
  self as ditto, macos_capture,
  wire::{
    common::{DeadlineKind, ErrorCode, StepStatus},
    result::{JobStatus, PhaseName, PhaseStatus, RunResult, RunStatus, ScenarioStatus},
  },
};
use battlement_tooling::unity_lease::CompilerCapacityLease;
use serde_json::json;

use crate::macos_fixture::{FixtureBuild, FixtureLauncher, PassMaterializer};

#[path = "support/macos_fixture.rs"]
mod macos_fixture;

#[path = "support/macos_cleanup.rs"]
mod macos_cleanup;

static CAPTURE_TEST_GATE: Mutex<()> = Mutex::new(());

#[test]
fn fixture_completes_three_scenarios_and_writes_a_valid_result() {
  let _guard = CAPTURE_TEST_GATE.lock().unwrap();
  let build = FixtureBuild::new(true);
  let run = tempfile::tempdir().unwrap();
  let launcher = FixtureLauncher::new(run.path(), json!({}), "complete");
  let outcome = macos_capture::capture_macos(
    macos_fixture::request(&build.handle, run.path(), 3),
    &launcher,
    Arc::new(PassMaterializer),
    &AtomicBool::new(false),
  )
  .unwrap();

  assert_eq!(launcher.count.load(Ordering::SeqCst), 1);
  assert_eq!(outcome.exit_code, 0);
  assert_eq!(outcome.orchestration.scenarios.len(), 3);
  assert_eq!(outcome.orchestration.jobs.len(), 1);
  assert!(outcome.player_exit.unwrap().code == Some(0));
  let diagnostic = outcome.player_session.as_ref().unwrap().diagnostic_paths[0].clone();
  assert!(run.path().join(&diagnostic).is_file());

  let mut result = macos_fixture::empty_result();
  outcome.apply_to(&mut result);
  result.artifacts = vec!["logs/events.jsonl".to_owned(), diagnostic];
  result.validate().unwrap();
  fs::write(
    run.path().join("result.json"),
    result.to_canonical_json().unwrap(),
  )
  .unwrap();
  let retained: RunResult =
    serde_json::from_slice(&fs::read(run.path().join("result.json")).unwrap()).unwrap();
  assert_eq!(retained.status, RunStatus::Passed);
  assert_eq!(retained.scenarios.len(), 3);
  assert_eq!(retained.player_sessions.len(), 1);
}

#[test]
fn every_startup_mismatch_stops_before_scenario_setup() {
  let _guard = CAPTURE_TEST_GATE.lock().unwrap();
  let build = FixtureBuild::new(true);
  let cases = [
    (
      "display",
      json!({"display": {"width": 640, "height": 720, "scale": 1.0, "orientation": null, "safe_area": [0, 0, 640, 720]}}),
    ),
    (
      "build",
      json!({"build_fingerprint": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}),
    ),
    (
      "source",
      json!({"source_fingerprint": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}),
    ),
    ("diagnostics", json!({"diagnostics": false})),
    ("adapter", json!({"capture_adapter": "wrong-adapter"})),
    ("capability", json!({"capabilities": []})),
    (
      "native ownership",
      json!({"native_execution_id": "83ef88f8-e5f8-4654-84a9-11410975266d"}),
    ),
    ("unity", json!({"unity_version": "6000.0.99f1"})),
  ];
  for (name, override_value) in cases {
    let run = tempfile::tempdir().unwrap();
    let launcher = FixtureLauncher::new(run.path(), override_value, "complete");
    let outcome = macos_capture::capture_macos(
      macos_fixture::request(&build.handle, run.path(), 1),
      &launcher,
      Arc::new(PassMaterializer),
      &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(outcome.exit_code, 2, "{name}");
    assert!(!outcome.player_session.unwrap().accepted, "{name}");
    assert!(outcome.orchestration.jobs.is_empty(), "{name}");
    assert!(!run.path().join("setup").exists(), "{name}");
  }
}

#[test]
fn diagnostics_disabled_build_never_starts_and_interrupt_is_bounded() {
  let _guard = CAPTURE_TEST_GATE.lock().unwrap();
  let disabled = FixtureBuild::new(false);
  let rejected_run = tempfile::tempdir().unwrap();
  let rejected = FixtureLauncher::new(rejected_run.path(), json!({}), "complete");
  let error = macos_capture::capture_macos(
    macos_fixture::request(&disabled.handle, rejected_run.path(), 1),
    &rejected,
    Arc::new(PassMaterializer),
    &AtomicBool::new(false),
  )
  .unwrap_err();
  assert!(error.to_string().contains("diagnostics disabled"));
  assert_eq!(rejected.count.load(Ordering::SeqCst), 0);

  let build = FixtureBuild::new(true);
  let run = tempfile::tempdir().unwrap();
  let launcher = FixtureLauncher::new(run.path(), json!({}), "idle");
  let interrupted = Arc::new(AtomicBool::new(false));
  let signal = interrupted.clone();
  thread::spawn(move || {
    thread::sleep(Duration::from_millis(100));
    signal.store(true, Ordering::Release);
  });
  let started = Instant::now();
  let outcome = macos_capture::capture_macos(
    macos_fixture::request(&build.handle, run.path(), 1),
    &launcher,
    Arc::new(PassMaterializer),
    interrupted.as_ref(),
  )
  .unwrap();
  assert_eq!(outcome.exit_code, 130);
  assert!(started.elapsed() < Duration::from_secs(2));
  assert!(run.path().join("logs").read_dir().unwrap().count() == 1);
}

#[test]
fn independent_native_captures_overlap_with_distinct_ownership() {
  let barrier = Arc::new(Barrier::new(2));
  let captures = (0..2)
    .map(|_| {
      let barrier = barrier.clone();
      thread::spawn(move || {
        let build = FixtureBuild::new(true);
        let run = tempfile::tempdir().unwrap();
        let launcher =
          FixtureLauncher::new(run.path(), json!({}), "complete").with_launch_barrier(barrier);
        macos_capture::capture_macos(
          macos_fixture::request(&build.handle, run.path(), 1),
          &launcher,
          Arc::new(PassMaterializer),
          &AtomicBool::new(false),
        )
        .unwrap()
      })
    })
    .collect::<Vec<_>>();
  let outcomes = captures
    .into_iter()
    .map(|capture| capture.join().unwrap())
    .collect::<Vec<_>>();

  assert!(outcomes.iter().all(|outcome| outcome.exit_code == 0));
  let ownership = outcomes
    .iter()
    .map(|outcome| {
      outcome
        .player_session
        .as_ref()
        .unwrap()
        .startup_report
        .as_ref()
        .unwrap()
        .native_execution_id
        .clone()
        .unwrap()
    })
    .collect::<Vec<_>>();
  assert_ne!(ownership[0], ownership[1]);
}

#[test]
fn player_startup_deadline_begins_after_machine_capacity_admission() {
  let _guard = CAPTURE_TEST_GATE.lock().unwrap();
  let run = tempfile::tempdir().unwrap();
  let slots = run.path().join("resource-slots");
  let first = CompilerCapacityLease::acquire(&slots).unwrap();
  let second = CompilerCapacityLease::acquire(&slots).unwrap();
  let path = run.path().to_owned();
  let capture = thread::spawn(move || {
    let build = FixtureBuild::new(true);
    let launcher = FixtureLauncher::new(&path, json!({}), "complete");
    macos_capture::capture_macos(
      macos_fixture::request(&build.handle, &path, 1),
      &launcher,
      Arc::new(PassMaterializer),
      &AtomicBool::new(false),
    )
    .unwrap()
  });
  thread::sleep(Duration::from_millis(150));
  assert!(!run.path().join("source-player.log").exists());
  drop(first);
  let outcome = capture.join().unwrap();
  drop(second);
  assert_eq!(outcome.exit_code, 0);
  assert!(outcome.phases[0].duration_ms < 2_000);
}

#[test]
fn startup_timeout_exit_and_cancellation_retain_truthful_public_evidence() {
  let _guard = CAPTURE_TEST_GATE.lock().unwrap();
  for mode in ["timeout", "early-exit", "cancel"] {
    let build = FixtureBuild::new(true);
    let run = tempfile::tempdir().unwrap();
    let launcher = FixtureLauncher::new(
      run.path(),
      json!({}),
      if mode == "early-exit" {
        "exit-before-startup"
      } else {
        "no-startup"
      },
    );
    let interrupted = Arc::new(AtomicBool::new(false));
    let signal = if mode == "cancel" {
      let interrupted = interrupted.clone();
      let ready = run.path().join("setup-startup");
      Some(thread::spawn(move || {
        let started = Instant::now();
        while !ready.exists() {
          assert!(started.elapsed() < Duration::from_secs(5));
          thread::sleep(Duration::from_millis(5));
        }
        interrupted.store(true, Ordering::Release);
      }))
    } else {
      None
    };
    let mut input = macos_fixture::request(&build.handle, run.path(), 1);
    input.timeouts.startup = Duration::from_millis(300);
    let outcome =
      macos_capture::capture_macos(input, &launcher, Arc::new(PassMaterializer), &interrupted)
        .unwrap();
    if let Some(signal) = signal {
      signal.join().unwrap();
    }
    assert_eq!(outcome.exit_code, if mode == "cancel" { 130 } else { 2 });
    assert_eq!(outcome.phases[0].name, PhaseName::Launch);
    assert_eq!(outcome.phases[0].status, PhaseStatus::Passed);
    let startup = &outcome.phases[1];
    assert_eq!(startup.name, PhaseName::Startup);
    assert_eq!(
      startup.status,
      if mode == "cancel" {
        PhaseStatus::Interrupted
      } else {
        PhaseStatus::Failed
      }
    );
    assert!(startup.duration_ms > 0);
    assert_eq!(
      startup.expired_deadline,
      (mode == "timeout").then_some(DeadlineKind::Startup)
    );
    if mode == "timeout" {
      assert!(startup.duration_ms >= 300);
    }
    assert_eq!(outcome.phases[2].name, PhaseName::Cleanup);
    assert_eq!(outcome.phases[2].status, PhaseStatus::Passed);
    let session = outcome.player_session.as_ref().unwrap();
    assert!(!session.accepted);
    assert!(session.startup_report.is_none());
    let diagnostic = session.diagnostic_paths[0].clone();
    assert_eq!(startup.log_path.as_deref(), Some(diagnostic.as_str()));
    assert_eq!(
      fs::read_to_string(run.path().join(&diagnostic)).unwrap(),
      "fixture player log\n"
    );
    assert!(outcome.orchestration.jobs.is_empty());
    if mode == "early-exit" {
      assert_eq!(outcome.player_exit.unwrap().code, Some(7));
    }
    if mode != "cancel" {
      assert_eq!(
        outcome.errors[0].code,
        if mode == "timeout" {
          ErrorCode::DeadlineExpired
        } else {
          ErrorCode::RuntimeProcessExit
        }
      );
      assert_eq!(
        outcome.errors[0].player_session_id.as_deref(),
        Some(session.player_session_id.as_str())
      );
    } else {
      assert!(outcome.errors.is_empty());
    }
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
    let mut result = macos_fixture::empty_result();
    outcome.apply_to(&mut result);
    result.artifacts = vec![diagnostic];
    let encoded = result.to_canonical_json().unwrap();
    let retained: RunResult = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(
      retained.phases[1].expired_deadline,
      startup.expired_deadline
    );
  }
}

#[test]
fn accepted_execution_losses_retain_durable_results_and_cleanup() {
  let _guard = CAPTURE_TEST_GATE.lock().unwrap();
  for mode in ["job-exit", "job-timeout", "job-between"] {
    let build = FixtureBuild::new(true);
    let run = tempfile::tempdir().unwrap();
    let launcher = FixtureLauncher::new(run.path(), json!({}), mode);
    let mut input = macos_fixture::request(&build.handle, run.path(), 3);
    input.job.remaining_run_timeout_ms = 500;
    let mut extra = input.job.scenarios[1].steps[0].clone();
    extra.index = 1;
    input.job.scenarios[1].steps.push(extra);
    let outcome = macos_capture::capture_macos(
      input,
      &launcher,
      Arc::new(PassMaterializer),
      &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(launcher.count.load(Ordering::SeqCst), 1);
    let session = outcome.player_session.as_ref().unwrap();
    assert!(session.accepted);
    assert!(session.startup_report.is_some());
    for name in [PhaseName::Launch, PhaseName::Startup, PhaseName::Cleanup] {
      assert_eq!(
        outcome
          .phases
          .iter()
          .find(|phase| phase.name == name)
          .unwrap()
          .status,
        PhaseStatus::Passed
      );
    }
    let phase = outcome
      .phases
      .iter()
      .find(|phase| phase.name == PhaseName::Scenarios)
      .unwrap();
    assert_eq!(phase.status, PhaseStatus::Failed);
    assert_eq!(
      phase.expired_deadline,
      (mode == "job-timeout").then_some(DeadlineKind::Run)
    );
    assert_eq!(outcome.errors.len(), 1, "{mode}: {:?}", outcome.errors);
    assert_eq!(
      outcome.errors[0].code,
      if mode == "job-timeout" {
        ErrorCode::DeadlineExpired
      } else {
        ErrorCode::RuntimeProcessExit
      }
    );
    assert_eq!(
      outcome.orchestration.jobs[0].status,
      JobStatus::InfrastructureError
    );
    let scenarios = &outcome.orchestration.scenarios;
    assert_eq!(scenarios.len(), 3);
    assert_eq!(scenarios[0].status, ScenarioStatus::Passed);
    assert_eq!(scenarios[1].status, ScenarioStatus::InfrastructureError);
    assert_eq!(scenarios[1].steps[0].status, StepStatus::Passed);
    assert_eq!(scenarios[1].steps[0].duration_ms, 3);
    assert_eq!(
      scenarios[1].steps[1].status,
      if mode == "job-between" {
        StepStatus::NotRun
      } else {
        StepStatus::InfrastructureError
      }
    );
    assert!(!scenarios[1].logs.as_ref().unwrap().complete);
    assert_eq!(scenarios[2].status, ScenarioStatus::NotRun);
    assert!(outcome.orchestration.pending_recovery.is_none());
    let diagnostic = session.diagnostic_paths[0].clone();
    assert_eq!(
      fs::read_to_string(run.path().join(&diagnostic)).unwrap(),
      "fixture player log\n"
    );
    let mut result = macos_fixture::empty_result();
    outcome.apply_to(&mut result);
    result.artifacts = vec![
      "logs/events.jsonl".to_owned(),
      diagnostic,
      "orchestration.json".to_owned(),
    ];
    let encoded = result.to_canonical_json().unwrap();
    let retained: RunResult = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(retained.status, RunStatus::InfrastructureError);
    let checkpoint: serde_json::Value =
      serde_json::from_slice(&fs::read(run.path().join("orchestration.json")).unwrap()).unwrap();
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
