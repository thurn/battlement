use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};
use uuid::Uuid;

use crate::{
  crash_scenario,
  player_supervision::{PlayerExitContext, PlayerExitRecovery},
  scenario_orchestration::ScenarioMaterializer,
  wire::{
    common::{ErrorCode, ErrorSource, StepStatus},
    job::{Job, ResolvedScenario},
    lifecycle::{
      DittoContext, DittoEventRecord, ExecutionStatus, NativeVideoInput, PlayerFailureFrame,
      PlayerStepResult, ReachedArtifact, ScenarioBoundaryOutcome, ScenarioComplete,
    },
    result::{
      ErrorOccurrence, JobResult, JobStatus, LogSpan, PlayerSessionResult, Recovery, ScenarioResult,
    },
  },
};

pub(crate) fn reconstruct(
  context: PlayerExitContext,
  error_id: &str,
  remaining_run_timeout_ms: u64,
  materializer: &dyn ScenarioMaterializer,
) -> Result<PlayerExitRecovery> {
  context.job.validate()?;
  host_error_id(error_id)?;
  let player_session = PlayerSessionResult {
    player_session_id: context.player_session_id.clone(),
    accepted: true,
    startup_report: context.startup_report.clone(),
    diagnostic_paths: context.diagnostic_paths.clone(),
  };
  let retained_artifact_ids = retained_artifacts(&context.durable.records);
  if !context.active_run {
    return Ok(PlayerExitRecovery {
      stale_session: true,
      player_session,
      job: None,
      scenario: None,
      occurrence: None,
      recovery_job: None,
      retained_artifact_ids,
    });
  }

  let last_sequence = context
    .durable
    .next_log_sequence
    .and_then(|next| next.checked_sub(1));
  let committed: BTreeSet<&str> = context
    .durable
    .completed_scenario_ids
    .iter()
    .map(String::as_str)
    .collect();
  let latest = latest_scenario(&context.durable.records, &context.job)?;
  let already_committed = latest
    .as_ref()
    .is_some_and(|latest| committed.contains(latest.scenario.id.as_str()));
  let occurrence = (!already_committed).then(|| {
    process_exit_occurrence(
      &context,
      error_id,
      latest.as_ref().map(|latest| latest.scenario.id.as_str()),
      latest.as_ref().and_then(|latest| latest.active_step),
      last_sequence,
    )
  });
  let scenario = reconstruct_scenario(
    &context,
    latest.as_ref(),
    already_committed,
    error_id,
    last_sequence,
    materializer,
  )?;
  let reached_index = reached_index(&context, latest.as_ref(), &committed);
  let recovery_job = recovery_job(&context.job, reached_index, remaining_run_timeout_ms);
  let indexes = reached_indexes(&context.job, &committed, scenario.as_ref());
  Ok(PlayerExitRecovery {
    stale_session: true,
    player_session,
    job: Some(JobResult {
      job_id: context.job.job_id,
      player_session_id: context.player_session_id,
      status: if occurrence.is_some() {
        JobStatus::InfrastructureError
      } else {
        JobStatus::Interrupted
      },
      first_scenario_index: indexes.first().copied(),
      last_scenario_index: indexes.last().copied(),
    }),
    scenario,
    occurrence,
    recovery_job,
    retained_artifact_ids,
  })
}

pub(crate) struct LatestScenario<'a> {
  pub(crate) scenario: &'a ResolvedScenario,
  pub(crate) first_sequence: u64,
  ended: Option<EndedScenario>,
  pub(crate) active_step: Option<u32>,
}

struct EndedScenario {
  execution_status: ExecutionStatus,
  failure_frame: Option<PlayerFailureFrame>,
  video_inputs: Vec<NativeVideoInput>,
  execution_duration_ms: u64,
  startup_duration_ms: u64,
  boundary: ScenarioBoundaryOutcome,
  primary_error_ref: Option<String>,
}

fn reconstruct_scenario(
  context: &PlayerExitContext,
  latest: Option<&LatestScenario<'_>>,
  already_committed: bool,
  error_id: &str,
  last_sequence: Option<u64>,
  materializer: &dyn ScenarioMaterializer,
) -> Result<Option<ScenarioResult>> {
  if already_committed {
    return Ok(None);
  }
  let Some(latest) = latest else {
    return Ok(None);
  };
  if let Some(ended) = &latest.ended {
    let complete = completion_from_context(context, latest, ended, last_sequence)?;
    let mut result = materializer
      .materialize(&context.job, &complete, Recovery::Relaunch)?
      .result;
    result.logs = log_span(context, latest.first_sequence, last_sequence, true);
    result.recovery = Recovery::Relaunch;
    Ok(Some(result))
  } else {
    Ok(Some(crash_scenario::incomplete(
      context,
      latest,
      error_id,
      last_sequence,
    )?))
  }
}

fn latest_scenario<'a>(
  records: &[DittoEventRecord],
  job: &'a Job,
) -> Result<Option<LatestScenario<'a>>> {
  let mut latest = None;
  for record in records {
    let DittoEventRecord::Context(record) = record else {
      continue;
    };
    match &record.body {
      DittoContext::ScenarioStarted { scenario_id } => {
        latest = Some(LatestScenario {
          scenario: job
            .scenarios
            .iter()
            .find(|scenario| scenario.id == *scenario_id)
            .context("durable scenario does not belong to the job")?,
          first_sequence: record.sequence,
          ended: None,
          active_step: None,
        });
      }
      DittoContext::StepStarted {
        scenario_id,
        step_index,
      } if owns_latest(&latest, scenario_id) => {
        latest.as_mut().unwrap().active_step = Some(*step_index)
      }
      DittoContext::StepEnded {
        scenario_id,
        result,
      } if owns_latest(&latest, scenario_id) => {
        if latest.as_ref().unwrap().active_step == Some(result.index) {
          latest.as_mut().unwrap().active_step = None;
        }
      }
      DittoContext::ScenarioEnded {
        scenario_id,
        execution_status,
        failure_frame,
        video_inputs,
        execution_duration_ms,
        startup_duration_ms,
        boundary,
        primary_error_ref,
      } if owns_latest(&latest, scenario_id) => {
        latest.as_mut().unwrap().ended = Some(EndedScenario {
          execution_status: *execution_status,
          failure_frame: failure_frame.clone(),
          video_inputs: video_inputs.clone(),
          execution_duration_ms: *execution_duration_ms,
          startup_duration_ms: *startup_duration_ms,
          boundary: boundary.clone(),
          primary_error_ref: primary_error_ref.clone(),
        });
        latest.as_mut().unwrap().active_step = None;
      }
      _ => {}
    }
  }
  Ok(latest)
}

fn completion_from_context(
  context: &PlayerExitContext,
  latest: &LatestScenario<'_>,
  ended: &EndedScenario,
  last_sequence: Option<u64>,
) -> Result<ScenarioComplete> {
  let steps = latest
    .scenario
    .steps
    .iter()
    .map(|expected| {
      crash_scenario::step_ended(
        &context.durable.records,
        &latest.scenario.id,
        expected.index,
      )
      .unwrap_or_else(|| PlayerStepResult {
        index: expected.index,
        name: expected.name.clone(),
        kind: crash_scenario::step_kind(&expected.action),
        status: StepStatus::NotRun,
        duration_ms: 0,
        expired_deadline: None,
        error_refs: Vec::new(),
        assertion: None,
        screenshot_artifact_id: None,
        video_input_id: None,
      })
    })
    .collect::<Vec<_>>();
  let artifacts = context
    .durable
    .records
    .iter()
    .filter_map(|record| match record {
      DittoEventRecord::Context(record) => match &record.body {
        DittoContext::ArtifactAccepted {
          scenario_id,
          step_index,
          artifact_id,
          artifact_kind,
        } if scenario_id == &latest.scenario.id => Some(ReachedArtifact {
          artifact_id: artifact_id.clone(),
          step_index: *step_index,
          kind: artifact_kind.clone(),
        }),
        _ => None,
      },
      DittoEventRecord::Log(_) => None,
    })
    .collect();
  Ok(ScenarioComplete {
    scenario_id: latest.scenario.id.clone(),
    execution_status: ended.execution_status,
    steps,
    artifacts,
    failure_frame: ended.failure_frame.clone(),
    video_inputs: ended.video_inputs.clone(),
    last_log_sequence: last_sequence.context("ended scenario has no durable log sequence")?,
    execution_duration_ms: ended.execution_duration_ms,
    startup_duration_ms: ended.startup_duration_ms,
    boundary: ended.boundary.clone(),
    primary_error_ref: ended.primary_error_ref.clone(),
  })
}

fn process_exit_occurrence(
  context: &PlayerExitContext,
  error_id: &str,
  scenario_id: Option<&str>,
  step_index: Option<u32>,
  log_sequence: Option<u64>,
) -> ErrorOccurrence {
  ErrorOccurrence {
    id: error_id.to_owned(),
    code: ErrorCode::RuntimeProcessExit,
    source: ErrorSource::Ditto,
    message: "owned player exited before job completion".to_owned(),
    job_id: Some(context.job.job_id.clone()),
    player_session_id: Some(context.player_session_id.clone()),
    scenario_id: scenario_id.map(str::to_owned),
    step_index,
    log_sequence,
  }
}

pub(crate) fn log_span(
  context: &PlayerExitContext,
  first: u64,
  last: Option<u64>,
  complete: bool,
) -> Option<LogSpan> {
  last.filter(|last| first <= *last).map(|last| LogSpan {
    job_id: context.job.job_id.clone(),
    player_session_id: context.player_session_id.clone(),
    first_sequence: first,
    last_sequence: last,
    complete,
    path: context.log_path.clone(),
  })
}

fn recovery_job(job: &Job, reached_index: Option<u32>, remaining: u64) -> Option<Job> {
  let scenarios = job
    .scenarios
    .iter()
    .filter(|scenario| reached_index.is_none_or(|index| scenario.run_index > index))
    .cloned()
    .collect::<Vec<_>>();
  if scenarios.is_empty() || remaining == 0 {
    return None;
  }
  let mut recovery = job.clone();
  recovery.job_id = Uuid::new_v4().to_string();
  recovery.remaining_run_timeout_ms = remaining;
  recovery.scenarios = scenarios;
  Some(recovery)
}

fn reached_index(
  context: &PlayerExitContext,
  latest: Option<&LatestScenario<'_>>,
  committed: &BTreeSet<&str>,
) -> Option<u32> {
  latest.map(|latest| latest.scenario.run_index).or_else(|| {
    context
      .job
      .scenarios
      .iter()
      .filter(|scenario| committed.contains(scenario.id.as_str()))
      .map(|scenario| scenario.run_index)
      .max()
  })
}

fn reached_indexes(
  job: &Job,
  committed: &BTreeSet<&str>,
  scenario: Option<&ScenarioResult>,
) -> Vec<u32> {
  job
    .scenarios
    .iter()
    .filter(|candidate| {
      committed.contains(candidate.id.as_str())
        || scenario.is_some_and(|scenario| scenario.id == candidate.id)
    })
    .map(|scenario| scenario.run_index)
    .collect()
}

fn retained_artifacts(records: &[DittoEventRecord]) -> Vec<String> {
  records
    .iter()
    .filter_map(|record| match record {
      DittoEventRecord::Context(record) => match &record.body {
        DittoContext::ArtifactAccepted { artifact_id, .. } => Some(artifact_id.clone()),
        _ => None,
      },
      DittoEventRecord::Log(_) => None,
    })
    .collect()
}

fn owns_latest(latest: &Option<LatestScenario<'_>>, scenario_id: &str) -> bool {
  latest
    .as_ref()
    .is_some_and(|latest| latest.scenario.id == scenario_id)
}

fn host_error_id(value: &str) -> Result<()> {
  ensure!(
    value.len() == 5
      && value.starts_with('E')
      && value[1..].bytes().all(|byte| byte.is_ascii_digit())
      && value != "E0000",
    "host error ID must use positive E####"
  );
  Ok(())
}
