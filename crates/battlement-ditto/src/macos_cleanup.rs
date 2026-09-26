use std::{
  fs,
  io::ErrorKind,
  path::Path,
  thread,
  time::{Duration, Instant},
};

use anyhow::Result;

use crate::{
  macos_capture::MacosCaptureOutcome,
  player_supervision::PlayerSupervisor,
  run_errors::RunErrors,
  wire::{
    common::{ErrorCode, ErrorSource},
    result::{ErrorOccurrence, PhaseName, PhaseResult, PhaseStatus},
  },
};

pub(crate) struct LogEvidence<'a> {
  pub source: &'a Path,
  pub directory: &'a Path,
  pub required: bool,
  pub errors: &'a RunErrors,
}

pub(crate) fn finish(
  outcome: &mut MacosCaptureOutcome,
  supervisor: &mut PlayerSupervisor,
  grace: Duration,
  poll_interval: Duration,
  evidence: LogEvidence<'_>,
) {
  let session = outcome
    .player_session
    .as_mut()
    .expect("launched player has a session");
  let started = Instant::now();
  let observed = wait_for_exit(supervisor, grace, poll_interval);
  let stopped = supervisor.stop();
  let mut cleanup = phase(PhaseName::Cleanup, started);
  if let Err(error) = observed {
    fail(
      &mut cleanup,
      &evidence,
      &session.player_session_id,
      ErrorCode::RuntimeDestroyFailed,
      format!("observe macOS shutdown: {error:#}"),
    );
  }
  outcome.player_exit = match stopped {
    Ok(status) => Some(status),
    Err(error) => {
      fail(
        &mut cleanup,
        &evidence,
        &session.player_session_id,
        ErrorCode::RuntimeDestroyFailed,
        format!("stop and reap macOS player: {error:#}"),
      );
      None
    }
  };
  let mut failed = cleanup.status == PhaseStatus::Failed;
  outcome.phases.push(cleanup);
  let started = Instant::now();
  let relative = format!("logs/player-{}.log", session.player_session_id);
  let retained = fs::create_dir_all(evidence.directory.join("logs"))
    .and_then(|()| fs::copy(evidence.source, evidence.directory.join(&relative)));
  match retained {
    Ok(_) => session.diagnostic_paths.push(relative),
    Err(error)
      if !evidence.required && error.kind() == ErrorKind::NotFound && !evidence.source.exists() => {
    }
    Err(error) => {
      let mut durability = phase(PhaseName::Durability, started);
      fail(
        &mut durability,
        &evidence,
        &session.player_session_id,
        ErrorCode::DurabilityFailed,
        format!("retain scoped macOS player log: {error}"),
      );
      outcome.phases.push(durability);
      failed = true;
    }
  }
  if failed && outcome.exit_code != 130 {
    outcome.exit_code = 2;
  }
  outcome.errors = evidence.errors.snapshot();
}

fn wait_for_exit(
  supervisor: &mut PlayerSupervisor,
  grace: Duration,
  poll_interval: Duration,
) -> Result<()> {
  let started = Instant::now();
  while started.elapsed() < grace {
    if !supervisor.is_alive()? {
      break;
    }
    thread::sleep(poll_interval);
  }
  Ok(())
}

fn fail(
  phase: &mut PhaseResult,
  evidence: &LogEvidence<'_>,
  session: &str,
  code: ErrorCode,
  message: String,
) {
  phase.status = PhaseStatus::Failed;
  phase
    .error_ids
    .push(evidence.errors.record(ErrorOccurrence {
      id: String::new(),
      code,
      message,
      source: ErrorSource::Ditto,
      player_session_id: Some(session.to_owned()),
      job_id: None,
      scenario_id: None,
      step_index: None,
      log_sequence: None,
    }));
}

fn phase(name: PhaseName, started: Instant) -> PhaseResult {
  PhaseResult {
    name,
    status: PhaseStatus::Passed,
    duration_ms: started.elapsed().as_millis() as u64,
    expired_deadline: None,
    log_path: None,
    error_ids: Vec::new(),
  }
}
