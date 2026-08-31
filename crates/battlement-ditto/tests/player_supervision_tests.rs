use std::{
  collections::BTreeMap,
  process::Command,
  sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
  },
  thread,
  time::Duration,
};

use battlement_ditto::{
  player_supervision::{
    PlayerExitContext, PlayerSupervisor, SimulatorApp, SupervisedPlatform, reconstruct_player_exit,
  },
  scenario_orchestration::{MaterializedScenario, ScenarioMaterializer},
  session_server::PlayerSessionDurableState,
  wire::{
    common::{ErrorCode, StepName, StepStatus},
    job::{
      Capability, Command as JobCommand, Display, InputTarget, Job, Motion, Platform,
      ResolvedProfile, ResolvedScenario, ResolvedStep, StepKind,
    },
    lifecycle::{
      ArtifactKind, DittoContext, DittoContextRecord, DittoEventRecord, DittoLogSeverity,
      DittoLogSource, ExecutionStatus, PlayerStepResult, ScenarioBoundaryOutcome, ScenarioComplete,
      StartupReport,
    },
    result::{JobStatus, Recovery, ScenarioResult, ScenarioStatus, ScenarioTimings, StepResult},
  },
};

const RUN_ID: &str = "21000000-0000-4000-8000-000000000001";
const JOB_ID: &str = "22000000-0000-4000-8000-000000000001";
const SESSION_ID: &str = "23000000-0000-4000-8000-000000000001";
const ARTIFACT_ID: &str = "24000000-0000-4000-8000-000000000001";

#[test]
fn owned_processes_and_simulator_apps_report_exit_once() {
  let child = Command::new("/bin/sh")
    .args(["-c", "exit 7"])
    .spawn()
    .unwrap();
  let mut macos = PlayerSupervisor::macos(child);
  let status = wait_for_exit(&mut macos);
  assert_eq!(status.platform, SupervisedPlatform::Macos);
  assert_eq!(status.code, Some(7));
  assert!(macos.poll().unwrap().is_none());

  let child = Command::new("/usr/bin/true").spawn().unwrap();
  let mut webgl = PlayerSupervisor::webgl(child);
  assert_eq!(
    wait_for_exit(&mut webgl).platform,
    SupervisedPlatform::Webgl
  );

  let running = Arc::new(AtomicBool::new(false));
  let terminated = Arc::new(AtomicUsize::new(0));
  let mut simulator = PlayerSupervisor::ios_simulator(Box::new(FakeSimulator {
    running,
    terminated: terminated.clone(),
  }));
  assert_eq!(
    simulator.poll().unwrap().unwrap().platform,
    SupervisedPlatform::IosSimulator
  );
  drop(simulator);
  assert_eq!(terminated.load(Ordering::SeqCst), 0);
}

#[test]
fn crash_during_scenario_preserves_partial_span_and_starts_after_it() {
  let records = vec![
    context(
      1,
      DittoContext::JobStarted {
        run_id: RUN_ID.to_owned(),
      },
    ),
    context(
      2,
      DittoContext::ScenarioStarted {
        scenario_id: scenario_id(0),
      },
    ),
    context(
      3,
      DittoContext::StepStarted {
        scenario_id: scenario_id(0),
        step_index: 0,
      },
    ),
    context(
      4,
      DittoContext::ArtifactAccepted {
        scenario_id: scenario_id(0),
        step_index: Some(0),
        artifact_id: ARTIFACT_ID.to_owned(),
        artifact_kind: ArtifactKind::FailureFrame,
      },
    ),
  ];
  let recovery = reconstruct_player_exit(
    exit_context(true, records, Vec::new()),
    "E0001",
    700,
    &TestMaterializer::default(),
  )
  .unwrap();

  let occurrence = recovery.occurrence.unwrap();
  assert_eq!(occurrence.code, ErrorCode::RuntimeProcessExit);
  assert_eq!(
    occurrence.scenario_id.as_deref(),
    Some(scenario_id(0).as_str())
  );
  assert_eq!(occurrence.step_index, Some(0));
  assert_eq!(occurrence.log_sequence, Some(4));
  let scenario = recovery.scenario.unwrap();
  assert_eq!(scenario.status, ScenarioStatus::Failed);
  assert_eq!(scenario.recovery, Recovery::Relaunch);
  assert_eq!(scenario.steps[0].status, StepStatus::InfrastructureError);
  assert_eq!(scenario.steps[0].error_ids, ["E0001"]);
  let logs = scenario.logs.unwrap();
  assert_eq!((logs.first_sequence, logs.last_sequence), (2, 4));
  assert!(!logs.complete);
  assert_eq!(recovery.retained_artifact_ids, [ARTIFACT_ID]);
  assert_eq!(recovery.job.unwrap().status, JobStatus::InfrastructureError);
  assert_eq!(scenario_indexes(recovery.recovery_job.unwrap()), vec![1, 2]);
  assert_eq!(
    recovery.player_session.diagnostic_paths,
    ["logs/player-session.log"]
  );
}

#[test]
fn ended_committed_between_and_idle_losses_have_distinct_records() {
  let materializer = TestMaterializer::default();
  let records = ended_records();
  let recovered = reconstruct_player_exit(
    exit_context(true, records.clone(), Vec::new()),
    "E0001",
    600,
    &materializer,
  )
  .unwrap();
  assert_eq!(materializer.calls.load(Ordering::SeqCst), 1);
  assert!(recovered.occurrence.is_some());
  assert!(
    recovered
      .scenario
      .as_ref()
      .unwrap()
      .logs
      .as_ref()
      .unwrap()
      .complete
  );
  assert_eq!(recovered.scenario.unwrap().recovery, Recovery::Relaunch);
  assert_eq!(
    scenario_indexes(recovered.recovery_job.unwrap()),
    vec![1, 2]
  );

  let committed = reconstruct_player_exit(
    exit_context(true, records, vec![scenario_id(0)]),
    "E0002",
    500,
    &materializer,
  )
  .unwrap();
  assert_eq!(materializer.calls.load(Ordering::SeqCst), 1);
  assert!(committed.occurrence.is_none());
  assert!(committed.scenario.is_none());
  assert_eq!(committed.job.unwrap().status, JobStatus::Interrupted);
  assert_eq!(
    scenario_indexes(committed.recovery_job.unwrap()),
    vec![1, 2]
  );

  let between = reconstruct_player_exit(
    exit_context(
      true,
      vec![context(
        1,
        DittoContext::JobStarted {
          run_id: RUN_ID.to_owned(),
        },
      )],
      Vec::new(),
    ),
    "E0003",
    400,
    &materializer,
  )
  .unwrap();
  assert!(between.scenario.is_none());
  assert!(between.occurrence.unwrap().scenario_id.is_none());
  assert_eq!(
    scenario_indexes(between.recovery_job.unwrap()),
    vec![0, 1, 2]
  );

  let idle = reconstruct_player_exit(
    exit_context(false, Vec::new(), Vec::new()),
    "E0004",
    300,
    &materializer,
  )
  .unwrap();
  assert!(idle.stale_session);
  assert!(idle.job.is_none());
  assert!(idle.scenario.is_none());
  assert!(idle.occurrence.is_none());
  assert!(idle.recovery_job.is_none());
}

#[derive(Default)]
struct TestMaterializer {
  calls: AtomicUsize,
}

impl ScenarioMaterializer for TestMaterializer {
  fn materialize(
    &self,
    job: &Job,
    complete: &ScenarioComplete,
    recovery: Recovery,
  ) -> anyhow::Result<MaterializedScenario> {
    self.calls.fetch_add(1, Ordering::SeqCst);
    let scenario = job
      .scenarios
      .iter()
      .find(|scenario| scenario.id == complete.scenario_id)
      .unwrap();
    Ok(MaterializedScenario {
      result: ScenarioResult {
        id: scenario.id.clone(),
        name: scenario.name.clone(),
        status: ScenarioStatus::Passed,
        status_reason: None,
        motion: scenario.motion,
        duration_ms: complete.execution_duration_ms,
        expired_deadline: None,
        timings: ScenarioTimings::default(),
        steps: complete.steps.iter().map(result_step).collect(),
        logs: None,
        failure_frame: None,
        recovery,
      },
      primary_failure: None,
    })
  }
}

struct FakeSimulator {
  running: Arc<AtomicBool>,
  terminated: Arc<AtomicUsize>,
}

impl SimulatorApp for FakeSimulator {
  fn is_running(&mut self) -> anyhow::Result<bool> {
    Ok(self.running.load(Ordering::SeqCst))
  }

  fn terminate(&mut self) -> anyhow::Result<()> {
    self.terminated.fetch_add(1, Ordering::SeqCst);
    Ok(())
  }
}

fn wait_for_exit(
  supervisor: &mut PlayerSupervisor,
) -> battlement_ditto::player_supervision::PlayerExitStatus {
  for _ in 0..100 {
    if let Some(status) = supervisor.poll().unwrap() {
      return status;
    }
    thread::sleep(Duration::from_millis(2));
  }
  panic!("owned process did not exit")
}

fn ended_records() -> Vec<DittoEventRecord> {
  vec![
    context(
      1,
      DittoContext::ScenarioStarted {
        scenario_id: scenario_id(0),
      },
    ),
    context(
      2,
      DittoContext::StepStarted {
        scenario_id: scenario_id(0),
        step_index: 0,
      },
    ),
    context(
      3,
      DittoContext::StepEnded {
        scenario_id: scenario_id(0),
        result: player_step(StepStatus::Passed),
      },
    ),
    context(
      4,
      DittoContext::ScenarioEnded {
        scenario_id: scenario_id(0),
        execution_status: ExecutionStatus::Passed,
        failure_frame: None,
        video_inputs: Vec::new(),
        execution_duration_ms: 10,
        startup_duration_ms: 2,
        settle_duration_ms: 3,
        capture_duration_ms: 4,
        boundary: ScenarioBoundaryOutcome::Passed { duration_ms: 1 },
        primary_error_ref: None,
      },
    ),
  ]
}

fn exit_context(
  active_run: bool,
  records: Vec<DittoEventRecord>,
  completed_scenario_ids: Vec<String>,
) -> PlayerExitContext {
  let first = records.first().map(sequence);
  let next = records.last().map(|record| sequence(record) + 1);
  PlayerExitContext {
    active_run,
    job: job(),
    player_session_id: SESSION_ID.to_owned(),
    startup_report: startup_report(),
    durable: PlayerSessionDurableState {
      first_log_sequence: first,
      next_log_sequence: next,
      records,
      completed_scenario_ids,
      terminal: None,
    },
    player_error_ids: BTreeMap::new(),
    log_path: "logs/events.jsonl".to_owned(),
    diagnostic_paths: vec!["logs/player-session.log".to_owned()],
  }
}

fn context(sequence: u64, body: DittoContext) -> DittoEventRecord {
  DittoEventRecord::Context(DittoContextRecord {
    schema: 1,
    job_id: JOB_ID.to_owned(),
    player_session_id: SESSION_ID.to_owned(),
    sequence,
    timestamp_unix_us: sequence as i64,
    source: DittoLogSource::DittoPlayer,
    severity: DittoLogSeverity::Information,
    event_name: "ditto.context".to_owned(),
    message: "context".to_owned(),
    body,
  })
}

fn sequence(record: &DittoEventRecord) -> u64 {
  match record {
    DittoEventRecord::Context(record) => record.sequence,
    DittoEventRecord::Log(record) => record.sequence,
  }
}

fn result_step(step: &PlayerStepResult) -> StepResult {
  StepResult {
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
  }
}

fn player_step(status: StepStatus) -> PlayerStepResult {
  PlayerStepResult {
    index: 0,
    name: Some("click".to_owned()),
    kind: StepName::Click,
    status,
    duration_ms: 10,
    expired_deadline: None,
    error_refs: Vec::new(),
    assertion: None,
    screenshot_artifact_id: None,
    video_input_id: None,
  }
}

fn job() -> Job {
  Job {
    job_id: JOB_ID.to_owned(),
    run_id: RUN_ID.to_owned(),
    remaining_run_timeout_ms: 1_000,
    log_redactions: Vec::new(),
    command: JobCommand::Capture,
    profile: ResolvedProfile {
      name: "macos".to_owned(),
      platform: Platform::Macos,
      display: display(),
      build_fingerprint: "a".repeat(64),
      source_fingerprint: "b".repeat(64),
      capabilities: vec![Capability::Click],
    },
    scenarios: (0..3).map(scenario).collect(),
  }
}

fn scenario(index: u32) -> ResolvedScenario {
  ResolvedScenario {
    id: scenario_id(index),
    run_index: index,
    name: format!("scenario-{index}"),
    fixture: None,
    motion: Motion::Controlled,
    timeout_ms: 100,
    steps: vec![ResolvedStep {
      index: 0,
      name: Some("click".to_owned()),
      timeout_ms: 50,
      action: StepKind::Click {
        target: InputTarget::Coordinates([0.5, 0.5]),
        settle: true,
      },
    }],
  }
}

fn startup_report() -> StartupReport {
  StartupReport {
    platform: Platform::Macos,
    capture_adapter: "native".to_owned(),
    build_fingerprint: "a".repeat(64),
    source_fingerprint: "b".repeat(64),
    unity_version: "6000.0.50f1".to_owned(),
    diagnostics: true,
    display: display(),
    capabilities: vec![Capability::Click],
  }
}

fn display() -> Display {
  Display {
    width: 100,
    height: 100,
    scale: 1.0,
    orientation: None,
    safe_area: [0, 0, 100, 100],
  }
}

fn scenario_id(index: u32) -> String {
  format!("25000000-0000-4000-8000-{index:012}")
}

fn scenario_indexes(job: Job) -> Vec<u32> {
  job
    .scenarios
    .iter()
    .map(|scenario| scenario.run_index)
    .collect()
}
