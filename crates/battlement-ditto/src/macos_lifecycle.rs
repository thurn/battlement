use std::{
  path::Path,
  sync::atomic::{AtomicBool, Ordering},
  thread,
  time::{Duration, Instant},
};

use crate::{
  macos_capture::MacosCaptureOutcome,
  macos_cleanup::{self, LogEvidence},
  player_supervision::{PlayerExitStatus, PlayerSupervisor},
  run_errors::RunErrors,
  scenario_orchestration::ScenarioOrchestrationSnapshot,
  session_server::{PlayerSessionServer, StartupFact},
  wire::{
    common::{DeadlineKind, ErrorCode, ErrorSource},
    lifecycle::{StartupIdentity, StartupReport},
    result::{ErrorOccurrence, PhaseName, PhaseResult, PhaseStatus, PlayerSessionResult},
  },
};

pub(crate) enum Failure {
  Interrupted,
  Expired,
  Exited(PlayerExitStatus),
  ObservationFailed(String),
}

pub(crate) enum Session<'a> {
  Launched { duration_ms: u64 },
  Accepted(&'a StartupReport),
}

#[derive(Clone, Copy)]
pub(crate) enum Stage {
  Startup,
  Execution,
}

pub(crate) struct Evidence<'a> {
  pub player_log: &'a Path,
  pub directory: &'a Path,
  pub session: Session<'a>,
  pub duration_ms: u64,
  pub stage: Stage,
  pub errors: &'a RunErrors,
}

pub(crate) fn wait(
  server: &PlayerSessionServer,
  supervisor: &mut PlayerSupervisor,
  interrupted: &AtomicBool,
  timeout: Duration,
  poll_interval: Duration,
) -> Result<StartupFact, Failure> {
  wait_until(supervisor, interrupted, timeout, poll_interval, || {
    server.snapshot().startup
  })
}

pub(crate) fn wait_for_next_job(
  server: &PlayerSessionServer,
  supervisor: &mut PlayerSupervisor,
  interrupted: &AtomicBool,
  timeout: Duration,
  poll_interval: Duration,
) -> Result<(), Failure> {
  wait_until(supervisor, interrupted, timeout, poll_interval, || {
    server.waiting_for_next_job().then_some(())
  })
}

pub(crate) fn wait_for_execution(
  server: &PlayerSessionServer,
  supervisor: &mut PlayerSupervisor,
  interrupted: &AtomicBool,
  timeout: Duration,
  poll_interval: Duration,
) -> Result<bool, Failure> {
  match wait_until(supervisor, interrupted, timeout, poll_interval, || {
    if interrupted.load(Ordering::Acquire) {
      Some(false)
    } else {
      server.durable_state().terminal.as_ref().map(|_| true)
    }
  }) {
    Ok(terminal) => Ok(terminal),
    Err(Failure::Interrupted) => Ok(false),
    Err(failure) => {
      server.expire();
      if server.durable_state().terminal.is_some() {
        Ok(true)
      } else {
        Err(failure)
      }
    }
  }
}

fn wait_until<T>(
  supervisor: &mut PlayerSupervisor,
  interrupted: &AtomicBool,
  timeout: Duration,
  poll_interval: Duration,
  observe: impl Fn() -> Option<T>,
) -> Result<T, Failure> {
  let started = Instant::now();
  loop {
    if let Some(value) = observe() {
      return Ok(value);
    }
    if interrupted.load(Ordering::Acquire) {
      return Err(Failure::Interrupted);
    }
    match supervisor.poll() {
      Ok(Some(status)) => return Err(Failure::Exited(status)),
      Err(error) => return Err(Failure::ObservationFailed(error.to_string())),
      Ok(None) => {}
    }
    if started.elapsed() >= timeout {
      return Err(Failure::Expired);
    }
    thread::sleep(poll_interval);
  }
}

pub(crate) fn finish(
  startup: Failure,
  server: &PlayerSessionServer,
  supervisor: &mut PlayerSupervisor,
  orchestration: ScenarioOrchestrationSnapshot,
  evidence: Evidence<'_>,
) -> MacosCaptureOutcome {
  let interrupted = matches!(startup, Failure::Interrupted);
  let (name, deadline_kind, label) = match evidence.stage {
    Stage::Startup => (PhaseName::Startup, DeadlineKind::Startup, "startup"),
    Stage::Execution => (PhaseName::Scenarios, DeadlineKind::Run, "run"),
  };
  let deadline = matches!(startup, Failure::Expired).then_some(deadline_kind);
  let errors = evidence.errors;
  let mut phases = Vec::new();
  if let Session::Launched { duration_ms } = evidence.session {
    phases.push(phase(PhaseName::Launch, PhaseStatus::Passed, duration_ms));
  }
  let mut startup_phase = phase(
    name,
    if interrupted {
      PhaseStatus::Interrupted
    } else {
      PhaseStatus::Failed
    },
    evidence.duration_ms,
  );
  startup_phase.expired_deadline = deadline;
  let failure = match startup {
    Failure::Interrupted => None,
    Failure::Expired => Some((
      ErrorCode::DeadlineExpired,
      format!("macOS {label} deadline expired"),
    )),
    Failure::Exited(status) => Some((
      ErrorCode::RuntimeProcessExit,
      format!("macOS player exited during {label}: {status:?}"),
    )),
    Failure::ObservationFailed(message) => Some((
      match evidence.stage {
        Stage::Startup => ErrorCode::StartupProbeFailed,
        Stage::Execution => ErrorCode::RuntimeFatal,
      },
      message,
    )),
  };
  if let Some((code, message)) = failure {
    startup_phase
      .error_ids
      .push(record(errors, server.player_session_id(), code, message));
  }
  let startup_index = phases.len();
  phases.push(startup_phase);
  server.expire();
  let mut outcome = MacosCaptureOutcome {
    exit_code: if interrupted { 130 } else { 2 },
    player_exit: None,
    player_session: Some(PlayerSessionResult {
      player_session_id: server.player_session_id().to_owned(),
      accepted: matches!(evidence.session, Session::Accepted(_)),
      startup_report: match evidence.session {
        Session::Accepted(report) => Some(report.clone()),
        Session::Launched { .. } => {
          server
            .snapshot()
            .startup
            .and_then(|startup| match startup.started.identity {
              StartupIdentity::Report(identity) => Some(identity.startup_report),
              StartupIdentity::Accepted(_) => None,
            })
        }
      },
      diagnostic_paths: Vec::new(),
    }),
    orchestration,
    phases,
    errors: Vec::new(),
  };
  macos_cleanup::finish(
    &mut outcome,
    supervisor,
    Duration::ZERO,
    Duration::ZERO,
    LogEvidence {
      source: evidence.player_log,
      directory: evidence.directory,
      required: matches!(evidence.session, Session::Accepted(_)),
      errors,
    },
  );
  outcome.phases[startup_index].log_path = outcome
    .player_session
    .as_ref()
    .unwrap()
    .diagnostic_paths
    .first()
    .cloned();
  outcome
}

fn record(errors: &RunErrors, session: &str, code: ErrorCode, message: String) -> String {
  errors.record(ErrorOccurrence {
    id: String::new(),
    code,
    message,
    source: ErrorSource::Ditto,
    player_session_id: Some(session.to_owned()),
    job_id: None,
    scenario_id: None,
    step_index: None,
    log_sequence: None,
  })
}

fn phase(name: PhaseName, status: PhaseStatus, duration_ms: u64) -> PhaseResult {
  PhaseResult {
    name,
    status,
    duration_ms,
    expired_deadline: None,
    log_path: None,
    error_ids: Vec::new(),
  }
}
