use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use crate::wire::{
  common::{ErrorCode, StepName},
  job::{Job, ResolvedScenario, StepKind, VideoStep},
  lifecycle::{
    ArtifactAck, ArtifactKind, HttpError, JobComplete, JobCompleteAck, JobFailed, JobFailedAck,
    LogBatchAck, NativeVideoInput, NextAction, PlayerFailureFrame, PlayerInfrastructureFailure,
    ScenarioDecision, Started, StartupIdentity, StartupReport, TerminalReason, UnstartedScenario,
  },
  validation,
};

pub(super) fn started(
  started: &Started,
  job: &Job,
  expected_player_session_id: &str,
  accepted_player_session_id: Option<&str>,
) -> Result<()> {
  job.validate()?;
  validation::identifier("player_session_id", expected_player_session_id)?;
  ensure!(
    started.job_id == job.job_id,
    "started job_id does not own this job"
  );
  ensure!(
    started.run_id == job.run_id,
    "started run_id does not own this job"
  );
  ensure!(
    started.player_session_id == expected_player_session_id,
    "started player_session_id does not own this route"
  );
  ensure!(
    started.startup_failure.is_none() || started.startup_log_failure.is_none(),
    "startup_failure and startup_log_failure are mutually exclusive"
  );
  ensure!(
    started.first_log_sequence.is_some() || started.startup_log_failure.is_some(),
    "first_log_sequence may be null only for a startup log failure"
  );
  if let Some(failure) = &started.startup_failure {
    failure_value(failure)?;
  }
  if let Some(failure) = &started.startup_log_failure {
    failure_value(failure)?;
  }
  match &started.identity {
    StartupIdentity::Report(identity) => {
      ensure!(
        accepted_player_session_id.is_none(),
        "a new player must report startup identity"
      );
      startup_report(&identity.startup_report)
    }
    StartupIdentity::Accepted(identity) => {
      validation::identifier(
        "accepted_player_session_id",
        &identity.accepted_player_session_id,
      )?;
      ensure!(
        accepted_player_session_id == Some(identity.accepted_player_session_id.as_str()),
        "accepted player session does not match the warm route"
      );
      ensure!(
        identity.accepted_player_session_id == started.player_session_id,
        "accepted identity must equal player_session_id"
      );
      Ok(())
    }
  }
}

pub(super) fn scenario_decision(decision: &ScenarioDecision) -> Result<()> {
  let fields = (
    decision.error_id.as_deref(),
    decision.error_code,
    decision.message.as_deref(),
  );
  match decision.action {
    NextAction::Continue => {
      ensure!(
        fields == (None, None, None),
        "continue decision must not contain error fields"
      );
    }
    NextAction::Stop | NextAction::Relaunch => {
      let (Some(error_id), Some(_), Some(message)) = fields else {
        anyhow::bail!("stop and relaunch decisions require every error field");
      };
      host_error_id(error_id)?;
      reason("decision.message", message)?;
    }
  }
  Ok(())
}

pub(super) fn job_complete(complete: &JobComplete, job: &Job) -> Result<()> {
  ensure!(
    complete.job_id == job.job_id,
    "completion job_id does not own this job"
  );
  ensure!(
    complete.execution_duration_ms <= job.remaining_run_timeout_ms,
    "job execution duration exceeds its remaining deadline"
  );
  terminal_accounting(
    job,
    &complete.executed_scenario_ids,
    &complete.unstarted_scenarios,
  )?;
  if complete.reason == TerminalReason::Completed {
    ensure!(
      complete.unstarted_scenarios.is_empty(),
      "completed job may not contain unstarted scenarios"
    );
  }
  Ok(())
}

pub(super) fn job_failed(failed: &JobFailed, job: &Job) -> Result<()> {
  ensure!(
    failed.job_id == job.job_id,
    "failure job_id does not own this job"
  );
  failure_value(&failed.failure)?;
  terminal_accounting(
    job,
    &failed.executed_scenario_ids,
    &failed.unstarted_scenarios,
  )
}

pub(super) fn http_error(error: &HttpError) -> Result<()> {
  host_error_id(&error.error_id)?;
  reason("http error message", &error.message)?;
  ensure!(
    error.expected_sequence.is_none() || error.code == ErrorCode::TransportLogGap,
    "expected_sequence is valid only for transport.log-gap"
  );
  if let Some(run_id) = &error.related_run_id {
    validation::identifier("related_run_id", run_id)?;
  }
  Ok(())
}

pub(super) fn log_ack(
  ack: &LogBatchAck,
  player_session_id: &str,
  expected_next_sequence: u64,
) -> Result<()> {
  validation::identifier("player_session_id", &ack.player_session_id)?;
  ensure!(
    ack.player_session_id == player_session_id,
    "log acknowledgement belongs to another player session"
  );
  ensure!(
    ack.next_sequence == expected_next_sequence,
    "log acknowledgement has the wrong next sequence"
  );
  Ok(())
}

pub(super) fn artifact_ack(
  ack: &ArtifactAck,
  expected_artifact_id: &str,
  expected_sha256: &str,
) -> Result<()> {
  validation::identifier("artifact_id", &ack.artifact_id)?;
  validation::sha256("artifact sha256", &ack.sha256)?;
  ensure!(
    ack.artifact_id == expected_artifact_id,
    "artifact acknowledgement has the wrong artifact_id"
  );
  ensure!(
    ack.sha256 == expected_sha256,
    "artifact acknowledgement has the wrong hash"
  );
  Ok(())
}

pub(super) fn complete_ack(ack: &JobCompleteAck, expected_job_id: &str) -> Result<()> {
  validation::identifier("job_id", &ack.job_id)?;
  ensure!(
    ack.job_id == expected_job_id,
    "completion acknowledgement has the wrong job_id"
  );
  Ok(())
}

pub(super) fn failed_ack(ack: &JobFailedAck, expected_job_id: &str) -> Result<()> {
  complete_ack(
    &JobCompleteAck {
      job_id: ack.job_id.clone(),
    },
    expected_job_id,
  )?;
  host_error_id(&ack.error_id)
}

pub(super) fn failure_value(failure: &PlayerInfrastructureFailure) -> Result<()> {
  reason("infrastructure failure message", &failure.message)
}

pub(super) fn startup_report(report: &StartupReport) -> Result<()> {
  validation::name("capture_adapter", &report.capture_adapter)?;
  validation::name("unity_version", &report.unity_version)?;
  validation::sha256("build_fingerprint", &report.build_fingerprint)?;
  validation::sha256("source_fingerprint", &report.source_fingerprint)?;
  validation::display(report.platform, &report.display)?;
  validation::profile_capabilities(report.platform, &report.capabilities)
}

pub(super) fn native_video(input: &NativeVideoInput, scenario: &ResolvedScenario) -> Result<()> {
  validation::identifier("video input_id", &input.input_id)?;
  validation::sha256("video sha256", &input.sha256)?;
  ensure!(
    !input.path.is_empty() && input.path.len() <= 1024,
    "native video path must contain 1 through 1024 UTF-8 bytes"
  );
  ensure!(
    input.width > 0 && input.height > 0,
    "native video dimensions must be positive"
  );
  ensure!(
    input.frame_count > 0,
    "native video frame_count must be positive"
  );
  let Some(step) = scenario.steps.get(input.start_step_index as usize) else {
    anyhow::bail!("native video start_step_index is outside the scenario");
  };
  ensure!(
    matches!(step.action, StepKind::Video(VideoStep::Start { .. })),
    "native video must reference a video start step"
  );
  Ok(())
}

pub(super) fn failure_frame(frame: &PlayerFailureFrame) -> Result<()> {
  match frame {
    PlayerFailureFrame::Captured { artifact_id } => {
      validation::identifier("failure frame artifact_id", artifact_id)
    }
    PlayerFailureFrame::Unavailable {
      reason: value,
      error_ref,
    } => {
      reason("failure frame reason", value)?;
      if let Some(error_ref) = error_ref {
        player_error_ref(error_ref)?;
      }
      Ok(())
    }
  }
}

pub(super) fn artifact_kind(kind: &ArtifactKind) -> Result<()> {
  if let ArtifactKind::Screenshot { checkpoint } = kind {
    validation::name("artifact checkpoint", checkpoint)?;
  }
  Ok(())
}

pub(super) fn scenario<'a>(job: &'a Job, scenario_id: &str) -> Result<&'a ResolvedScenario> {
  validation::identifier("scenario_id", scenario_id)?;
  job
    .scenarios
    .iter()
    .find(|scenario| scenario.id == scenario_id)
    .ok_or_else(|| anyhow::anyhow!("scenario_id does not belong to this job"))
}

pub(super) fn step_name(kind: &StepKind) -> StepName {
  match kind {
    StepKind::Click { .. } => StepName::Click,
    StepKind::Hover { .. } => StepName::Hover,
    StepKind::Drag { .. } => StepName::Drag,
    StepKind::Key { .. } => StepName::Key,
    StepKind::Wait(_) => StepName::Wait,
    StepKind::Assert(_) => StepName::Assert,
    StepKind::AccessibilityAssert(_) => StepName::AccessibilityAssert,
    StepKind::AccessibilityAction { .. } => StepName::AccessibilityAction,
    StepKind::Screenshot(_) => StepName::Screenshot,
    StepKind::Video(_) => StepName::Video,
  }
}

pub(super) fn player_error_ref(value: &str) -> Result<()> {
  numbered_reference("player error reference", value, 'P')
}

pub(super) fn host_error_id(value: &str) -> Result<()> {
  numbered_reference("host error ID", value, 'E')
}

pub(super) fn reason(field: &str, value: &str) -> Result<()> {
  ensure!(!value.is_empty(), "{field} must not be empty");
  ensure!(
    value.len() <= 4096,
    "{field} may contain at most 4096 UTF-8 bytes"
  );
  Ok(())
}

fn terminal_accounting(
  job: &Job,
  executed: &[String],
  unstarted: &[UnstartedScenario],
) -> Result<()> {
  ensure!(
    executed.len() + unstarted.len() == job.scenarios.len(),
    "terminal scenario accounting must cover every job scenario"
  );
  let mut ids = BTreeSet::new();
  for (index, id) in executed.iter().enumerate() {
    validation::identifier("executed scenario_id", id)?;
    ensure!(ids.insert(id), "terminal scenario IDs must be unique");
    ensure!(
      job.scenarios[index].id == *id,
      "executed scenarios must be an ordered job prefix"
    );
  }
  for (offset, entry) in unstarted.iter().enumerate() {
    validation::identifier("unstarted scenario_id", &entry.scenario_id)?;
    reason("unstarted scenario reason", &entry.reason)?;
    ensure!(
      ids.insert(&entry.scenario_id),
      "terminal scenario IDs must be unique"
    );
    ensure!(
      job.scenarios[executed.len() + offset].id == entry.scenario_id,
      "unstarted scenarios must be the ordered job suffix"
    );
  }
  Ok(())
}

fn numbered_reference(field: &str, value: &str, prefix: char) -> Result<()> {
  let bytes = value.as_bytes();
  ensure!(
    bytes.len() == 5 && bytes[0] == prefix as u8,
    "{field} must use {prefix}#### syntax"
  );
  ensure!(
    bytes[1..].iter().all(u8::is_ascii_digit) && &bytes[1..] != b"0000",
    "{field} must use a positive four-digit sequence"
  );
  Ok(())
}
