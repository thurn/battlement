//! Durable host decisions between player scenarios.

use std::{
  fs,
  path::PathBuf,
  sync::{Arc, Mutex},
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
  session_server::PlayerSessionHandler,
  wire::{
    common::{ErrorCode, StepName, StepStatus},
    job::{Job, ResolvedScenario, StepKind},
    lifecycle::{
      JobComplete, JobCompleteAck, JobFailed, JobFailedAck, NextAction, ScenarioBoundaryOutcome,
      ScenarioComplete, ScenarioDecision,
    },
    result::{
      JobResult, JobStatus, Recovery, ScenarioResult, ScenarioStatus, ScenarioTimings, StepResult,
    },
    run_storage_io,
  },
};

/// A stable failure returned with a stop or relaunch decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionFailure {
  pub error_id: String,
  pub code: ErrorCode,
  pub message: String,
}

/// A completed scenario after player data and host processing are combined.
pub struct MaterializedScenario {
  pub result: ScenarioResult,
  pub primary_failure: Option<DecisionFailure>,
}

/// Produces the authoritative scenario result before the next player action.
pub trait ScenarioMaterializer: Send + Sync + 'static {
  fn materialize(
    &self,
    job: &Job,
    complete: &ScenarioComplete,
    recovery: Recovery,
  ) -> Result<MaterializedScenario>;
}

/// Durable orchestration facts suitable for a partial run checkpoint.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioOrchestrationSnapshot {
  pub run_deadline_ms: u64,
  pub completed_failures: u32,
  pub jobs: Vec<JobResult>,
  pub scenarios: Vec<ScenarioResult>,
  pub pending_recovery: Option<Job>,
  pub last_decision: Option<ScenarioDecision>,
}

/// Coordinates scenario results, bail decisions, and recovery-job suffixes.
pub struct ScenarioOrchestrator {
  state_path: PathBuf,
  bail_after: Option<u32>,
  now_ms: Arc<dyn Fn() -> u64 + Send + Sync>,
  materializer: Arc<dyn ScenarioMaterializer>,
  state: Mutex<State>,
}

struct State {
  deadline_ms: u64,
  completed_failures: u32,
  jobs: Vec<JobResult>,
  scenarios: Vec<(u32, ScenarioResult)>,
  active: Option<ActiveJob>,
  pending_recovery: Option<Job>,
  last_decision: Option<ScenarioDecision>,
}

struct ActiveJob {
  job: Job,
  player_session_id: String,
  infrastructure_failure: bool,
}

impl ScenarioOrchestrator {
  /// Starts one run using an injected monotonic clock in milliseconds.
  pub fn new(
    job: Job,
    player_session_id: String,
    state_path: PathBuf,
    bail_after: Option<u32>,
    now_ms: Arc<dyn Fn() -> u64 + Send + Sync>,
    materializer: Arc<dyn ScenarioMaterializer>,
  ) -> Result<Self> {
    job.validate()?;
    ensure!(
      bail_after.is_none_or(|value| value > 0),
      "bail count must be positive"
    );
    let deadline_ms = now_ms()
      .checked_add(job.remaining_run_timeout_ms)
      .context("run deadline overflow")?;
    let orchestrator = Self {
      state_path,
      bail_after,
      now_ms,
      materializer,
      state: Mutex::new(State {
        deadline_ms,
        completed_failures: 0,
        jobs: Vec::new(),
        scenarios: Vec::new(),
        active: Some(ActiveJob {
          job,
          player_session_id,
          infrastructure_failure: false,
        }),
        pending_recovery: None,
        last_decision: None,
      }),
    };
    persist(
      &orchestrator.state_path,
      &orchestrator.state.lock().unwrap().snapshot(),
    )?;
    Ok(orchestrator)
  }

  /// Returns the last durable orchestration checkpoint.
  pub fn snapshot(&self) -> ScenarioOrchestrationSnapshot {
    self.state.lock().unwrap().snapshot()
  }

  /// Installs the pending recovery suffix for a newly launched player.
  pub fn begin_recovery(&self, player_session_id: String) -> Result<Option<Job>> {
    let mut state = self.state.lock().unwrap();
    ensure!(state.active.is_none(), "the previous job is not terminal");
    let Some(mut job) = state.pending_recovery.take() else {
      return Ok(None);
    };
    let remaining = state.deadline_ms.saturating_sub((self.now_ms)());
    if remaining == 0 {
      mark_suffix(&mut state, &job.scenarios, "run-infrastructure-error");
      mark_relaunch_failed(&mut state);
      persist(&self.state_path, &state.snapshot())?;
      return Ok(None);
    }
    job.remaining_run_timeout_ms = remaining;
    state.active = Some(ActiveJob {
      job: job.clone(),
      player_session_id,
      infrastructure_failure: false,
    });
    persist(&self.state_path, &state.snapshot())?;
    Ok(Some(job))
  }

  /// Records a failed recovery launch and suppresses every remaining scenario.
  pub fn relaunch_failed(&self) -> Result<()> {
    let mut state = self.state.lock().unwrap();
    ensure!(state.active.is_none(), "the previous job is not terminal");
    let job = state
      .pending_recovery
      .take()
      .context("there is no pending recovery job")?;
    mark_suffix(&mut state, &job.scenarios, "run-infrastructure-error");
    mark_relaunch_failed(&mut state);
    persist(&self.state_path, &state.snapshot())
  }
}

impl PlayerSessionHandler for ScenarioOrchestrator {
  fn scenario_complete(&self, complete: &ScenarioComplete) -> Result<ScenarioDecision> {
    let mut state = self.state.lock().unwrap();
    let active_job = state
      .active
      .as_ref()
      .context("there is no active job")?
      .job
      .clone();
    let scenario = active_job
      .scenarios
      .iter()
      .find(|scenario| scenario.id == complete.scenario_id)
      .context("scenario does not belong to the active job")?;
    ensure!(
      !state
        .scenarios
        .iter()
        .any(|(_, result)| result.id == complete.scenario_id),
      "a reached scenario cannot be completed twice"
    );
    let recovery = match complete.boundary {
      ScenarioBoundaryOutcome::Passed { .. } => Recovery::Reset,
      ScenarioBoundaryOutcome::Failed { .. } => Recovery::Relaunch,
    };
    let materialized = self
      .materializer
      .materialize(&active_job, complete, recovery)?;
    validate_materialized(scenario, &materialized)?;
    let run_index = scenario.run_index;
    let remaining = active_job
      .scenarios
      .iter()
      .skip_while(|candidate| candidate.id != complete.scenario_id)
      .skip(1)
      .cloned()
      .collect::<Vec<_>>();
    let functional_failure = materialized.result.status == ScenarioStatus::Failed;
    if functional_failure {
      state.completed_failures += 1;
    }
    let bail = self
      .bail_after
      .is_some_and(|limit| state.completed_failures >= limit);
    let infrastructure = matches!(
      materialized.result.status,
      ScenarioStatus::InfrastructureError | ScenarioStatus::Interrupted
    );
    let boundary_failed = matches!(complete.boundary, ScenarioBoundaryOutcome::Failed { .. });
    state.scenarios.push((run_index, materialized.result));
    if infrastructure || boundary_failed {
      state.active.as_mut().unwrap().infrastructure_failure = true;
    }
    let action = if bail || infrastructure {
      mark_suffix(
        &mut state,
        &remaining,
        if bail {
          "bail"
        } else {
          "run-infrastructure-error"
        },
      );
      NextAction::Stop
    } else if boundary_failed && !remaining.is_empty() {
      state.pending_recovery = Some(recovery_job(&active_job, remaining));
      NextAction::Relaunch
    } else if boundary_failed {
      NextAction::Stop
    } else {
      NextAction::Continue
    };
    let decision = decision(
      action,
      state.completed_failures,
      materialized.primary_failure,
    )?;
    state.last_decision = Some(decision.clone());
    persist(&self.state_path, &state.snapshot())?;
    Ok(decision)
  }

  fn job_complete(&self, complete: &JobComplete) -> Result<JobCompleteAck> {
    let mut state = self.state.lock().unwrap();
    complete.validate(&state.active.as_ref().context("there is no active job")?.job)?;
    let active = state.active.take().unwrap();
    let indexes = complete
      .executed_scenario_ids
      .iter()
      .filter_map(|id| {
        active
          .job
          .scenarios
          .iter()
          .find(|scenario| scenario.id == *id)
      })
      .map(|scenario| scenario.run_index)
      .collect::<Vec<_>>();
    let failed = complete.executed_scenario_ids.iter().any(|id| {
      state
        .scenarios
        .iter()
        .any(|(_, scenario)| scenario.id == *id && scenario.status == ScenarioStatus::Failed)
    });
    state.jobs.push(JobResult {
      job_id: active.job.job_id.clone(),
      player_session_id: active.player_session_id,
      status: if active.infrastructure_failure {
        JobStatus::InfrastructureError
      } else if failed {
        JobStatus::Failed
      } else {
        JobStatus::Passed
      },
      first_scenario_index: indexes.first().copied(),
      last_scenario_index: indexes.last().copied(),
    });
    persist(&self.state_path, &state.snapshot())?;
    Ok(JobCompleteAck {
      job_id: complete.job_id.clone(),
    })
  }

  fn job_failed(&self, failed: &JobFailed, error_id: &str) -> Result<JobFailedAck> {
    let mut state = self.state.lock().unwrap();
    failed.validate(&state.active.as_ref().context("there is no active job")?.job)?;
    let active = state.active.take().unwrap();
    let remaining = active
      .job
      .scenarios
      .iter()
      .filter(|scenario| !failed.executed_scenario_ids.contains(&scenario.id))
      .cloned()
      .collect::<Vec<_>>();
    mark_suffix(&mut state, &remaining, "run-infrastructure-error");
    state.pending_recovery = None;
    let indexes = failed
      .executed_scenario_ids
      .iter()
      .filter_map(|id| {
        active
          .job
          .scenarios
          .iter()
          .find(|scenario| scenario.id == *id)
      })
      .map(|scenario| scenario.run_index)
      .collect::<Vec<_>>();
    state.jobs.push(JobResult {
      job_id: active.job.job_id.clone(),
      player_session_id: active.player_session_id,
      status: JobStatus::InfrastructureError,
      first_scenario_index: indexes.first().copied(),
      last_scenario_index: indexes.last().copied(),
    });
    persist(&self.state_path, &state.snapshot())?;
    Ok(JobFailedAck {
      job_id: failed.job_id.clone(),
      error_id: error_id.to_owned(),
    })
  }
}

impl State {
  fn snapshot(&self) -> ScenarioOrchestrationSnapshot {
    let mut scenarios = self.scenarios.clone();
    scenarios.sort_by_key(|(index, _)| *index);
    ScenarioOrchestrationSnapshot {
      run_deadline_ms: self.deadline_ms,
      completed_failures: self.completed_failures,
      jobs: self.jobs.clone(),
      scenarios: scenarios.into_iter().map(|(_, result)| result).collect(),
      pending_recovery: self.pending_recovery.clone(),
      last_decision: self.last_decision.clone(),
    }
  }
}

/// Creates a canonical capability-skip result for a selected scenario.
pub fn skipped_scenario(scenario: &ResolvedScenario, reason: &str) -> Result<ScenarioResult> {
  ensure!(
    reason.starts_with("unsupported-input:") || reason.starts_with("unsupported-step:"),
    "skip reason must identify an unsupported capability"
  );
  Ok(unstarted(scenario, ScenarioStatus::Skipped, reason))
}

fn validate_materialized(
  expected: &ResolvedScenario,
  materialized: &MaterializedScenario,
) -> Result<()> {
  let result = &materialized.result;
  ensure!(
    result.id == expected.id,
    "materialized scenario has the wrong ID"
  );
  ensure!(
    result.name == expected.name,
    "materialized scenario has the wrong name"
  );
  ensure!(
    result.motion == expected.motion,
    "materialized scenario has the wrong motion"
  );
  ensure!(
    !matches!(
      result.status,
      ScenarioStatus::Skipped | ScenarioStatus::NotRun
    ),
    "a completion must materialize a reached status"
  );
  if result.status == ScenarioStatus::Failed || result.recovery == Recovery::Relaunch {
    ensure!(
      materialized.primary_failure.is_some(),
      "failed decision requires a primary error"
    );
  }
  Ok(())
}

fn decision(
  action: NextAction,
  completed_failures: u32,
  failure: Option<DecisionFailure>,
) -> Result<ScenarioDecision> {
  let decision = match action {
    NextAction::Continue => ScenarioDecision {
      action,
      completed_failures,
      error_id: None,
      error_code: None,
      message: None,
    },
    NextAction::Stop | NextAction::Relaunch => {
      let failure = failure.context("stop and relaunch require a primary failure")?;
      ScenarioDecision {
        action,
        completed_failures,
        error_id: Some(failure.error_id),
        error_code: Some(failure.code),
        message: Some(failure.message),
      }
    }
  };
  decision.validate()?;
  Ok(decision)
}

fn recovery_job(job: &Job, scenarios: Vec<ResolvedScenario>) -> Job {
  let mut recovery = job.clone();
  recovery.job_id = Uuid::new_v4().to_string();
  recovery.scenarios = scenarios;
  recovery
}

fn mark_suffix(state: &mut State, scenarios: &[ResolvedScenario], reason: &str) {
  for scenario in scenarios {
    if !state
      .scenarios
      .iter()
      .any(|(_, result)| result.id == scenario.id)
    {
      state.scenarios.push((
        scenario.run_index,
        unstarted(scenario, ScenarioStatus::NotRun, reason),
      ));
    }
  }
}

fn mark_relaunch_failed(state: &mut State) {
  if let Some((_, result)) = state
    .scenarios
    .iter_mut()
    .rev()
    .find(|(_, result)| result.recovery == Recovery::Relaunch)
  {
    result.recovery = Recovery::RelaunchFailed;
  }
}

fn unstarted(scenario: &ResolvedScenario, status: ScenarioStatus, reason: &str) -> ScenarioResult {
  ScenarioResult {
    id: scenario.id.clone(),
    name: scenario.name.clone(),
    status,
    status_reason: Some(reason.to_owned()),
    motion: scenario.motion,
    duration_ms: 0,
    expired_deadline: None,
    timings: ScenarioTimings::default(),
    steps: scenario
      .steps
      .iter()
      .map(|step| StepResult {
        index: step.index,
        name: step.name.clone(),
        kind: step_kind(&step.action),
        status: StepStatus::NotRun,
        status_reason: Some(reason.to_owned()),
        duration_ms: 0,
        expired_deadline: None,
        error_ids: Vec::new(),
        assertion: None,
        screenshot: None,
        video: None,
      })
      .collect(),
    logs: None,
    failure_frame: None,
    recovery: Recovery::None,
  }
}

fn step_kind(kind: &StepKind) -> StepName {
  match kind {
    StepKind::Click { .. } => StepName::Click,
    StepKind::Hover { .. } => StepName::Hover,
    StepKind::Drag { .. } => StepName::Drag,
    StepKind::Key { .. } => StepName::Key,
    StepKind::Wait(_) => StepName::Wait,
    StepKind::Assert(_) => StepName::Assert,
    StepKind::Screenshot(_) => StepName::Screenshot,
    StepKind::Video(_) => StepName::Video,
  }
}

fn persist(path: &std::path::Path, snapshot: &ScenarioOrchestrationSnapshot) -> Result<()> {
  let parent = path
    .parent()
    .context("orchestration state path has no parent")?;
  fs::create_dir_all(parent)?;
  let mut bytes = serde_json::to_vec_pretty(snapshot)?;
  bytes.push(b'\n');
  run_storage_io::write_atomic(path, &bytes)
}
