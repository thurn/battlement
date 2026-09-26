use anyhow::Result;

use crate::{
  execution_materializer,
  macos_capture::{MacosCaptureOutcome, MacosCaptureRequest},
  player_supervision::{self, PlayerExitContext},
  scenario_orchestration::{ScenarioMaterializer, ScenarioOrchestrator},
  session_server::PlayerSessionServer,
  wire::{
    common::{DeadlineKind, ErrorCode, ErrorSource, StepStatus},
    result::{
      ErrorOccurrence, JobResult, JobStatus, PhaseName, PhaseResult, PhaseStatus, Recovery,
      ScenarioStatus,
    },
  },
};

pub(crate) fn reconstruct(
  outcome: &mut MacosCaptureOutcome,
  request: &MacosCaptureRequest<'_>,
  server: &PlayerSessionServer,
  orchestrator: &ScenarioOrchestrator,
  materializer: &dyn ScenarioMaterializer,
) {
  if let Err(error) = recover(outcome, request, server, orchestrator, materializer) {
    let checkpoint = orchestrator.snapshot();
    if !checkpoint.jobs.is_empty() {
      outcome.orchestration = checkpoint;
    }
    let id = request.errors.record(ErrorOccurrence {
      id: String::new(),
      code: ErrorCode::DurabilityFailed,
      source: ErrorSource::Ditto,
      message: format!("retain dispatched job evidence: {error:#}"),
      player_session_id: Some(server.player_session_id().to_owned()),
      job_id: None,
      scenario_id: None,
      step_index: None,
      log_sequence: None,
    });
    outcome.phases.push(PhaseResult {
      name: PhaseName::Durability,
      status: PhaseStatus::Failed,
      duration_ms: 0,
      expired_deadline: None,
      log_path: None,
      error_ids: vec![id],
    });
  }
  outcome.errors = request.errors.snapshot();
}

fn recover(
  outcome: &mut MacosCaptureOutcome,
  request: &MacosCaptureRequest<'_>,
  server: &PlayerSessionServer,
  orchestrator: &ScenarioOrchestrator,
  materializer: &dyn ScenarioMaterializer,
) -> Result<()> {
  let phase = outcome
    .phases
    .iter()
    .find(|phase| phase.name == PhaseName::Scenarios)
    .unwrap();
  let id = &phase.error_ids[0];
  let deadline = phase.expired_deadline;
  let mut failure = outcome
    .errors
    .iter()
    .find(|error| &error.id == id)
    .unwrap()
    .clone();
  failure.job_id = Some(request.job.job_id.clone());
  let durable = server.durable_state();
  let player_error_ids =
    execution_materializer::retain_observed_errors(&request.errors, &durable.records);
  outcome.orchestration = orchestrator.snapshot();
  if !outcome
    .orchestration
    .jobs
    .iter()
    .any(|job| job.job_id == request.job.job_id)
  {
    outcome.orchestration.jobs.push(JobResult {
      job_id: request.job.job_id.clone(),
      player_session_id: server.player_session_id().to_owned(),
      status: JobStatus::InfrastructureError,
      first_scenario_index: None,
      last_scenario_index: None,
    });
  }
  request.errors.update(failure.clone());
  let session = outcome.player_session.as_ref().unwrap();
  let mut recovery = player_supervision::reconstruct_player_exit(
    PlayerExitContext {
      active_run: true,
      job: request.job.clone(),
      player_session_id: session.player_session_id.clone(),
      startup_report: session.startup_report.clone().unwrap(),
      durable,
      player_error_ids,
      log_path: "logs/events.jsonl".to_owned(),
      diagnostic_paths: session.diagnostic_paths.clone(),
    },
    id,
    0,
    materializer,
  )?;
  if let Some(mut occurrence) = recovery.occurrence {
    occurrence.code = failure.code;
    occurrence.message = failure.message;
    request.errors.update(occurrence);
  }
  if let Some(scenario) = recovery.scenario.as_mut()
    && scenario.logs.as_ref().is_some_and(|logs| !logs.complete)
  {
    scenario.status = ScenarioStatus::InfrastructureError;
    scenario.recovery = Recovery::None;
    scenario.expired_deadline = deadline;
    if deadline == Some(DeadlineKind::Run) {
      for step in &mut scenario.steps {
        if step.status == StepStatus::InfrastructureError {
          step.expired_deadline = deadline;
        }
      }
    }
  }
  let mut job = recovery.job.unwrap();
  job.status = JobStatus::InfrastructureError;
  outcome.orchestration = orchestrator.player_lost(job, recovery.scenario)?;
  Ok(())
}
