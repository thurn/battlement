use std::{
  fs,
  sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
  },
};

use battlement_ditto::{
  scenario_orchestration::{
    DecisionFailure, MaterializedScenario, ScenarioMaterializer, ScenarioOrchestrationSnapshot,
    ScenarioOrchestrator, skipped_scenario,
  },
  session_server::PlayerSessionHandler,
  wire::{
    common::{ErrorCode, StepName, StepStatus},
    job::{
      Capability, Command, Display, InputTarget, Job, Motion, Platform, ResolvedProfile,
      ResolvedScenario, ResolvedStep, StepKind,
    },
    lifecycle::{
      BoundaryStage, ExecutionStatus, JobComplete, NextAction, ScenarioBoundaryOutcome,
      ScenarioComplete, TerminalReason, UnstartedScenario,
    },
    result::{JobStatus, Recovery, ScenarioResult, ScenarioStatus, ScenarioTimings, StepResult},
  },
};
use tempfile::TempDir;

const RUN_ID: &str = "01000000-0000-4000-8000-000000000001";
const JOB_ID: &str = "02000000-0000-4000-8000-000000000001";
const SESSION_1: &str = "03000000-0000-4000-8000-000000000001";
const SESSION_2: &str = "03000000-0000-4000-8000-000000000002";

#[test]
fn two_jobs_preserve_results_deadline_relaunch_and_bail() {
  let directory = TempDir::new().unwrap();
  let state_path = directory.path().join("orchestration.json");
  let clock = Arc::new(AtomicU64::new(100));
  let orchestrator = ScenarioOrchestrator::new(
    job(5),
    SESSION_1.to_owned(),
    state_path.clone(),
    Some(2),
    clock_fn(&clock),
    Arc::new(TestMaterializer),
  )
  .unwrap();

  assert_decision(
    &orchestrator,
    completion(0, ExecutionStatus::Passed, passed_boundary()),
    NextAction::Continue,
    0,
  );
  assert_decision(
    &orchestrator,
    completion(1, ExecutionStatus::Failed, passed_boundary()),
    NextAction::Continue,
    1,
  );
  let relaunch = orchestrator
    .scenario_complete(&completion(
      2,
      ExecutionStatus::Passed,
      ScenarioBoundaryOutcome::Failed {
        duration_ms: 9,
        stage: BoundaryStage::Reset,
        error_ref: "P0002".to_owned(),
      },
    ))
    .unwrap();
  assert_eq!(relaunch.action, NextAction::Relaunch);
  assert_eq!(relaunch.completed_failures, 1);
  assert_eq!(relaunch.error_code, Some(ErrorCode::RuntimeResetFailed));
  let recovery_id = orchestrator.snapshot().pending_recovery.unwrap().job_id;

  orchestrator
    .job_complete(&job_complete(
      JOB_ID,
      &[0, 1, 2],
      &[3, 4],
      TerminalReason::InfrastructureError,
      "relaunch",
    ))
    .unwrap();
  clock.store(350, Ordering::SeqCst);
  let recovery = orchestrator
    .begin_recovery(SESSION_2.to_owned())
    .unwrap()
    .unwrap();
  assert_eq!(recovery.job_id, recovery_id);
  assert_ne!(recovery.job_id, JOB_ID);
  assert_eq!(recovery.remaining_run_timeout_ms, 750);
  assert_eq!(scenario_indexes(&recovery), vec![3, 4]);

  let stop = orchestrator
    .scenario_complete(&completion(3, ExecutionStatus::Failed, passed_boundary()))
    .unwrap();
  assert_eq!(stop.action, NextAction::Stop);
  assert_eq!(stop.completed_failures, 2);
  orchestrator
    .job_complete(&job_complete(
      &recovery.job_id,
      &[3],
      &[4],
      TerminalReason::Bail,
      "bail",
    ))
    .unwrap();

  let snapshot = orchestrator.snapshot();
  assert_eq!(snapshot.run_deadline_ms, 1_100);
  assert_eq!(snapshot.completed_failures, 2);
  assert_eq!(snapshot.jobs.len(), 2);
  assert_eq!(snapshot.jobs[0].status, JobStatus::InfrastructureError);
  assert_eq!(snapshot.jobs[1].status, JobStatus::Failed);
  assert_eq!(snapshot.scenarios.len(), 5);
  assert_eq!(snapshot.scenarios[0].status, ScenarioStatus::Passed);
  assert_eq!(snapshot.scenarios[1].status, ScenarioStatus::Failed);
  assert_eq!(snapshot.scenarios[2].recovery, Recovery::Relaunch);
  assert_eq!(snapshot.scenarios[3].status, ScenarioStatus::Failed);
  assert_eq!(snapshot.scenarios[4].status, ScenarioStatus::NotRun);
  assert_eq!(snapshot.scenarios[4].status_reason.as_deref(), Some("bail"));
  assert_eq!(
    serde_json::from_slice::<ScenarioOrchestrationSnapshot>(&fs::read(state_path).unwrap())
      .unwrap(),
    snapshot
  );
}

#[test]
fn failed_relaunch_marks_only_the_unreached_suffix_not_run() {
  let directory = TempDir::new().unwrap();
  let orchestrator = ScenarioOrchestrator::new(
    job(3),
    SESSION_1.to_owned(),
    directory.path().join("orchestration.json"),
    None,
    Arc::new(|| 0),
    Arc::new(TestMaterializer),
  )
  .unwrap();
  let decision = orchestrator
    .scenario_complete(&completion(
      0,
      ExecutionStatus::Passed,
      ScenarioBoundaryOutcome::Failed {
        duration_ms: 3,
        stage: BoundaryStage::Destroy,
        error_ref: "P0002".to_owned(),
      },
    ))
    .unwrap();
  assert_eq!(decision.action, NextAction::Relaunch);
  orchestrator
    .job_complete(&job_complete(
      JOB_ID,
      &[0],
      &[1, 2],
      TerminalReason::InfrastructureError,
      "relaunch",
    ))
    .unwrap();

  orchestrator.relaunch_failed().unwrap();
  let snapshot = orchestrator.snapshot();
  assert_eq!(snapshot.scenarios[0].recovery, Recovery::RelaunchFailed);
  assert!(snapshot.scenarios[1..].iter().all(|scenario| {
    scenario.status == ScenarioStatus::NotRun
      && scenario.status_reason.as_deref() == Some("run-infrastructure-error")
  }));
  assert!(
    orchestrator
      .begin_recovery(SESSION_2.to_owned())
      .unwrap()
      .is_none()
  );
}

#[test]
fn capability_skip_retains_every_authored_step() {
  let scenario = scenario(7);
  let result = skipped_scenario(&scenario, "unsupported-input:hover").unwrap();
  assert_eq!(result.status, ScenarioStatus::Skipped);
  assert_eq!(
    result.status_reason.as_deref(),
    Some("unsupported-input:hover")
  );
  assert_eq!(result.steps.len(), scenario.steps.len());
  assert!(result.steps.iter().all(|step| {
    step.status == StepStatus::NotRun
      && step.status_reason.as_deref() == Some("unsupported-input:hover")
  }));
}

struct TestMaterializer;

impl ScenarioMaterializer for TestMaterializer {
  fn materialize(
    &self,
    job: &Job,
    complete: &ScenarioComplete,
    recovery: Recovery,
  ) -> anyhow::Result<MaterializedScenario> {
    let scenario = job
      .scenarios
      .iter()
      .find(|scenario| scenario.id == complete.scenario_id)
      .unwrap();
    let status = match complete.execution_status {
      ExecutionStatus::Passed => ScenarioStatus::Passed,
      ExecutionStatus::Failed => ScenarioStatus::Failed,
      ExecutionStatus::Interrupted => ScenarioStatus::Interrupted,
    };
    let boundary_failure = match complete.boundary {
      ScenarioBoundaryOutcome::Failed {
        stage: BoundaryStage::Destroy,
        ..
      } => Some((ErrorCode::RuntimeDestroyFailed, "player destroy failed")),
      ScenarioBoundaryOutcome::Failed {
        stage: BoundaryStage::Reset,
        ..
      } => Some((ErrorCode::RuntimeResetFailed, "player reset failed")),
      ScenarioBoundaryOutcome::Passed { .. } => None,
    };
    let functional_failure = (status == ScenarioStatus::Failed)
      .then_some((ErrorCode::AssertionFailed, "scenario assertion failed"));
    let primary_failure = boundary_failure
      .or(functional_failure)
      .map(|(code, message)| DecisionFailure {
        error_id: format!("E{:04}", scenario.run_index + 1),
        code,
        message: message.to_owned(),
      });
    Ok(MaterializedScenario {
      result: reached_result(scenario, complete, status, recovery),
      primary_failure,
    })
  }
}

fn reached_result(
  scenario: &ResolvedScenario,
  complete: &ScenarioComplete,
  status: ScenarioStatus,
  recovery: Recovery,
) -> ScenarioResult {
  ScenarioResult {
    id: scenario.id.clone(),
    name: scenario.name.clone(),
    status,
    status_reason: None,
    motion: scenario.motion,
    duration_ms: complete.startup_duration_ms + complete.execution_duration_ms,
    expired_deadline: None,
    timings: ScenarioTimings {
      startup_ms: Some(complete.startup_duration_ms),
      reset_ms: Some(match complete.boundary {
        ScenarioBoundaryOutcome::Passed { duration_ms }
        | ScenarioBoundaryOutcome::Failed { duration_ms, .. } => duration_ms,
      }),
      baseline_download_ms: Some(0),
      comparison_ms: Some(0),
      media_ms: Some(0),
      durability_ms: Some(0),
    },
    steps: complete
      .steps
      .iter()
      .map(|step| StepResult {
        index: step.index,
        name: step.name.clone(),
        kind: step.kind,
        status: step.status,
        status_reason: None,
        duration_ms: step.duration_ms,
        expired_deadline: step.expired_deadline,
        error_ids: Vec::new(),
        assertion: step.assertion.clone(),
        screenshot: None,
        video: None,
      })
      .collect(),
    logs: None,
    failure_frame: None,
    recovery,
  }
}

fn assert_decision(
  orchestrator: &ScenarioOrchestrator,
  complete: ScenarioComplete,
  action: NextAction,
  completed_failures: u32,
) {
  let decision = orchestrator.scenario_complete(&complete).unwrap();
  assert_eq!(decision.action, action);
  assert_eq!(decision.completed_failures, completed_failures);
}

fn completion(
  index: u32,
  execution_status: ExecutionStatus,
  boundary: ScenarioBoundaryOutcome,
) -> ScenarioComplete {
  ScenarioComplete {
    scenario_id: scenario_id(index),
    execution_status,
    steps: vec![battlement_ditto::wire::lifecycle::PlayerStepResult {
      index: 0,
      name: Some("click".to_owned()),
      kind: StepName::Click,
      status: if execution_status == ExecutionStatus::Failed {
        StepStatus::Failed
      } else {
        StepStatus::Passed
      },
      duration_ms: 1,
      expired_deadline: None,
      error_refs: (execution_status == ExecutionStatus::Failed)
        .then(|| "P0001".to_owned())
        .into_iter()
        .collect(),
      assertion: None,
      screenshot_artifact_id: None,
      video_input_id: None,
    }],
    artifacts: Vec::new(),
    failure_frame: None,
    video_inputs: Vec::new(),
    last_log_sequence: 1,
    execution_duration_ms: 2,
    startup_duration_ms: 1,
    boundary,
    primary_error_ref: (execution_status == ExecutionStatus::Failed).then(|| "P0001".to_owned()),
  }
}

fn passed_boundary() -> ScenarioBoundaryOutcome {
  ScenarioBoundaryOutcome::Passed { duration_ms: 2 }
}

fn job_complete(
  job_id: &str,
  executed: &[u32],
  unstarted: &[u32],
  reason: TerminalReason,
  unstarted_reason: &str,
) -> JobComplete {
  JobComplete {
    job_id: job_id.to_owned(),
    last_log_sequence: 1,
    executed_scenario_ids: executed.iter().copied().map(scenario_id).collect(),
    unstarted_scenarios: unstarted
      .iter()
      .copied()
      .map(|index| UnstartedScenario {
        scenario_id: scenario_id(index),
        reason: unstarted_reason.to_owned(),
      })
      .collect(),
    reason,
    execution_duration_ms: 10,
  }
}

fn job(count: u32) -> Job {
  Job {
    job_id: JOB_ID.to_owned(),
    run_id: RUN_ID.to_owned(),
    remaining_run_timeout_ms: 1_000,
    log_redactions: Vec::new(),
    command: Command::Capture,
    profile: ResolvedProfile {
      name: "macos".to_owned(),
      platform: Platform::Macos,
      display: Display {
        width: 100,
        height: 100,
        scale: 1.0,
        orientation: None,
        safe_area: [0, 0, 100, 100],
      },
      build_fingerprint: "a".repeat(64),
      source_fingerprint: "b".repeat(64),
      capabilities: vec![Capability::Click],
    },
    scenarios: (0..count).map(scenario).collect(),
  }
}

fn scenario(index: u32) -> ResolvedScenario {
  ResolvedScenario {
    id: scenario_id(index),
    run_index: index,
    name: format!("scenario-{index}"),
    motion: Motion::Controlled,
    timeout_ms: 100,
    steps: vec![ResolvedStep {
      index: 0,
      name: Some("click".to_owned()),
      timeout_ms: 10,
      action: StepKind::Click {
        target: InputTarget::Coordinates([0.5, 0.5]),
      },
    }],
  }
}

fn scenario_id(index: u32) -> String {
  format!("10000000-0000-4000-8000-{index:012}")
}

fn scenario_indexes(job: &Job) -> Vec<u32> {
  job
    .scenarios
    .iter()
    .map(|scenario| scenario.run_index)
    .collect()
}

fn clock_fn(clock: &Arc<AtomicU64>) -> Arc<dyn Fn() -> u64 + Send + Sync> {
  let clock = clock.clone();
  Arc::new(move || clock.load(Ordering::SeqCst))
}
