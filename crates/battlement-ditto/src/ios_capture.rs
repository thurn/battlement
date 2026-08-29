//! Bounded execution of one immutable iOS Simulator player.

use std::{
  path::PathBuf,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use battlement_tooling::{
  build_cache::BuildHandle,
  ios_build::{self, IosStartupIdentity},
};
use uuid::Uuid;

use crate::{
  config::model::Orientation,
  ios_simulator::IosSimulator,
  player_supervision::{PlayerExitStatus, PlayerSupervisor, SimulatorApp},
  scenario_orchestration::{
    ScenarioMaterializer, ScenarioOrchestrationSnapshot, ScenarioOrchestrator,
  },
  session_server::{PlayerSessionRequirements, PlayerSessionServer},
  wire::{
    common::DeadlineKind,
    job::{Command as JobCommand, Job, Platform},
    lifecycle::{NextAction, StartupIdentity},
    result::{
      PhaseName, PhaseResult, PhaseStatus, PlayerSessionResult, RunResult, RunStatus,
      ScenarioStatus,
    },
  },
};

/// Host phase limits for one Simulator player.
#[derive(Clone, Copy, Debug)]
pub struct IosCaptureTimeouts {
  pub startup: Duration,
  pub shutdown: Duration,
  pub interrupt_grace: Duration,
  pub poll_interval: Duration,
}

/// Immutable inputs for one Simulator player job.
pub struct IosCaptureRequest<'a> {
  pub build: &'a BuildHandle,
  pub simulator: Arc<Mutex<IosSimulator>>,
  pub orientation: Orientation,
  pub job: Job,
  pub requirements: PlayerSessionRequirements,
  pub orchestration_path: PathBuf,
  pub bail_after: Option<u32>,
  pub timeouts: IosCaptureTimeouts,
}

/// Durable Simulator facts ready to merge into a terminal run result.
#[derive(Debug)]
pub struct IosCaptureOutcome {
  pub exit_code: u8,
  pub player_exit: Option<PlayerExitStatus>,
  pub player_session: Option<PlayerSessionResult>,
  pub orchestration: ScenarioOrchestrationSnapshot,
  pub phases: Vec<PhaseResult>,
}

struct SharedSimulator(Arc<Mutex<IosSimulator>>);

impl SimulatorApp for SharedSimulator {
  fn is_running(&mut self) -> Result<bool> {
    self.0.lock().unwrap().is_running()
  }

  fn terminate(&mut self) -> Result<()> {
    self.0.lock().unwrap().terminate()
  }
}

impl IosCaptureOutcome {
  /// Applies execution facts without replacing build or artifact metadata.
  pub fn apply_to(&self, result: &mut RunResult) {
    result.status = match self.exit_code {
      0 => RunStatus::Passed,
      1 => RunStatus::Failed,
      130 => RunStatus::Interrupted,
      _ => RunStatus::InfrastructureError,
    };
    result.exit_code = self.exit_code;
    result.phases = self.phases.clone();
    result.player_sessions = self.player_session.clone().into_iter().collect();
    result.jobs = self.orchestration.jobs.clone();
  }
}

/// Launches, validates, supervises, and drains one Simulator capture job.
pub fn capture_ios(
  request: IosCaptureRequest<'_>,
  materializer: Arc<dyn ScenarioMaterializer>,
  interrupted: &AtomicBool,
) -> Result<IosCaptureOutcome> {
  let identity = self::validate_build(&request)?;
  self::validate_timeouts(request.timeouts)?;
  let player_session_id = Uuid::new_v4().to_string();
  let origin = Instant::now();
  let now = Arc::new(move || origin.elapsed().as_millis() as u64);
  let orchestrator = Arc::new(ScenarioOrchestrator::new(
    request.job.clone(),
    player_session_id.clone(),
    request.orchestration_path.clone(),
    request.bail_after,
    now,
    materializer,
  )?);
  let server = PlayerSessionServer::bind_with_identity(
    request.job.clone(),
    request.requirements.clone(),
    player_session_id.clone(),
    orchestrator.clone(),
  )?;
  let launch_started = Instant::now();
  request.simulator.lock().unwrap().install_and_launch(
    &ios_build::player_app(request.build)?,
    &server.base_url(),
    request.orientation,
  )?;
  let mut supervisor =
    PlayerSupervisor::ios_simulator(Box::new(SharedSimulator(request.simulator.clone())));
  let mut phases = vec![self::phase(
    PhaseName::Launch,
    PhaseStatus::Passed,
    self::elapsed_ms(launch_started),
    None,
  )];
  let startup_started = Instant::now();
  let startup = self::wait_for_startup(
    &server,
    &mut supervisor,
    interrupted,
    request.timeouts,
    startup_started,
  )?;
  let Some(startup) = startup else {
    server.expire();
    let player_exit = self::wait_for_exit(
      &mut supervisor,
      request.timeouts.interrupt_grace,
      request.timeouts.poll_interval,
    )?;
    let diagnostic = self::retain_log(&request, &player_session_id)?;
    request.simulator.lock().unwrap().delete()?;
    phases.push(self::phase(
      PhaseName::Startup,
      PhaseStatus::Interrupted,
      self::elapsed_ms(startup_started),
      None,
    ));
    phases.push(self::phase_with_log(PhaseName::Cleanup, diagnostic.clone()));
    return Ok(IosCaptureOutcome {
      exit_code: 130,
      player_exit,
      player_session: self::startup_report(&server).map(|startup_report| PlayerSessionResult {
        player_session_id,
        accepted: false,
        startup_report,
        diagnostic_paths: vec![diagnostic],
      }),
      orchestration: orchestrator.snapshot(),
      phases,
    });
  };
  let report = match &startup.started.identity {
    StartupIdentity::Report(identity) => Some(identity.startup_report.clone()),
    StartupIdentity::Accepted(_) => None,
  };
  if startup.decision.action != NextAction::Continue {
    phases.push(self::phase(
      PhaseName::Startup,
      PhaseStatus::Failed,
      self::elapsed_ms(startup_started),
      None,
    ));
    server.expire();
    let player_exit = self::wait_for_exit(
      &mut supervisor,
      request.timeouts.shutdown,
      request.timeouts.poll_interval,
    )?;
    let diagnostic = self::retain_log(&request, &player_session_id)?;
    request.simulator.lock().unwrap().delete()?;
    phases.push(self::phase_with_log(PhaseName::Cleanup, diagnostic.clone()));
    return Ok(IosCaptureOutcome {
      exit_code: 2,
      player_exit,
      player_session: report.map(|startup_report| PlayerSessionResult {
        player_session_id,
        accepted: false,
        startup_report,
        diagnostic_paths: vec![diagnostic],
      }),
      orchestration: orchestrator.snapshot(),
      phases,
    });
  }
  let report = report.context("fresh Simulator player did not report startup facts")?;
  ensure!(
    report.diagnostics && identity.diagnostics,
    "Simulator diagnostics are disabled"
  );
  phases.push(self::phase(
    PhaseName::Startup,
    PhaseStatus::Passed,
    self::elapsed_ms(startup_started),
    None,
  ));
  let run_started = Instant::now();
  let terminal = loop {
    if interrupted.load(Ordering::Acquire) {
      break false;
    }
    if server.durable_state().terminal.is_some() {
      break true;
    }
    if let Some(status) = supervisor.poll()? {
      anyhow::bail!("Simulator player exited before durable job completion: {status:?}");
    }
    ensure!(
      run_started.elapsed() < Duration::from_millis(request.job.remaining_run_timeout_ms),
      "Simulator capture exceeded the run deadline"
    );
    thread::sleep(request.timeouts.poll_interval);
  };
  server.expire();
  let shutdown = if terminal {
    request.timeouts.shutdown
  } else {
    request.timeouts.interrupt_grace
  };
  let player_exit = self::wait_for_exit(&mut supervisor, shutdown, request.timeouts.poll_interval)?;
  let diagnostic = self::retain_log(&request, &player_session_id)?;
  request.simulator.lock().unwrap().delete()?;
  let orchestration = orchestrator.snapshot();
  let exit_code = if terminal {
    self::execution_exit_code(&orchestration)
  } else {
    130
  };
  phases.push(self::phase(
    PhaseName::Scenarios,
    if terminal {
      if exit_code == 0 {
        PhaseStatus::Passed
      } else {
        PhaseStatus::Failed
      }
    } else {
      PhaseStatus::Interrupted
    },
    self::elapsed_ms(run_started),
    None,
  ));
  phases.extend(self::boundary_phases(&orchestration));
  phases.push(self::phase_with_log(PhaseName::Cleanup, diagnostic.clone()));
  Ok(IosCaptureOutcome {
    exit_code,
    player_exit,
    player_session: Some(PlayerSessionResult {
      player_session_id,
      accepted: true,
      startup_report: report,
      diagnostic_paths: vec![diagnostic],
    }),
    orchestration,
    phases,
  })
}

fn validate_build(request: &IosCaptureRequest<'_>) -> Result<IosStartupIdentity> {
  request.job.validate()?;
  ensure!(
    matches!(request.job.command, JobCommand::Run | JobCommand::Capture),
    "Simulator launcher requires an execution job"
  );
  ensure!(
    request.job.profile.platform == Platform::IosSimulator,
    "capture profile is not iOS Simulator"
  );
  let identity = ios_build::ios_startup_identity(request.build)?;
  ensure!(
    identity.diagnostics && request.requirements.diagnostics,
    "runtime diagnostics are disabled"
  );
  ensure!(
    request.job.profile.build_fingerprint == identity.build_fingerprint,
    "job selected another build"
  );
  ensure!(
    request.job.profile.source_fingerprint == identity.source_fingerprint,
    "job selected another source"
  );
  ensure!(
    request.requirements.capture_adapter == identity.capture_adapter,
    "runtime selected another capture adapter"
  );
  ensure!(
    request.requirements.unity_version == identity.unity_version,
    "runtime selected another Unity version"
  );
  Ok(identity)
}

fn validate_timeouts(timeouts: IosCaptureTimeouts) -> Result<()> {
  ensure!(
    !timeouts.startup.is_zero(),
    "startup timeout must be positive"
  );
  ensure!(
    !timeouts.shutdown.is_zero(),
    "shutdown timeout must be positive"
  );
  ensure!(
    timeouts.interrupt_grace <= Duration::from_secs(2),
    "interrupt grace exceeds two seconds"
  );
  ensure!(
    !timeouts.poll_interval.is_zero(),
    "poll interval must be positive"
  );
  Ok(())
}

fn wait_for_startup(
  server: &PlayerSessionServer,
  supervisor: &mut PlayerSupervisor,
  interrupted: &AtomicBool,
  timeouts: IosCaptureTimeouts,
  started: Instant,
) -> Result<Option<crate::session_server::StartupFact>> {
  loop {
    if let Some(startup) = server.snapshot().startup {
      return Ok(Some(startup));
    }
    if interrupted.load(Ordering::Acquire) {
      return Ok(None);
    }
    if let Some(status) = supervisor.poll()? {
      anyhow::bail!("Simulator player exited before startup: {status:?}");
    }
    ensure!(
      started.elapsed() < timeouts.startup,
      "Simulator startup deadline expired"
    );
    thread::sleep(timeouts.poll_interval);
  }
}

fn wait_for_exit(
  supervisor: &mut PlayerSupervisor,
  timeout: Duration,
  poll: Duration,
) -> Result<Option<PlayerExitStatus>> {
  let started = Instant::now();
  while started.elapsed() < timeout {
    if let Some(status) = supervisor.poll()? {
      return Ok(Some(status));
    }
    thread::sleep(poll);
  }
  Ok(None)
}

fn retain_log(request: &IosCaptureRequest<'_>, session_id: &str) -> Result<String> {
  let relative = format!("logs/simulator-{session_id}.log");
  request
    .simulator
    .lock()
    .unwrap()
    .retain_logs(&request.requirements.storage_directory.join(&relative))?;
  Ok(relative)
}

fn startup_report(server: &PlayerSessionServer) -> Option<crate::wire::lifecycle::StartupReport> {
  server
    .snapshot()
    .startup
    .and_then(|startup| match startup.started.identity {
      StartupIdentity::Report(identity) => Some(identity.startup_report),
      StartupIdentity::Accepted(_) => None,
    })
}

fn execution_exit_code(snapshot: &ScenarioOrchestrationSnapshot) -> u8 {
  if snapshot.scenarios.iter().any(|scenario| {
    matches!(
      scenario.status,
      ScenarioStatus::InfrastructureError | ScenarioStatus::Interrupted
    )
  }) {
    2
  } else if snapshot
    .scenarios
    .iter()
    .any(|scenario| scenario.status == ScenarioStatus::Failed)
  {
    1
  } else {
    0
  }
}

fn boundary_phases(snapshot: &ScenarioOrchestrationSnapshot) -> [PhaseResult; 2] {
  let duration = |value: fn(&crate::wire::result::ScenarioTimings) -> Option<u64>| {
    snapshot
      .scenarios
      .iter()
      .filter_map(|scenario| value(&scenario.timings))
      .sum()
  };
  [
    self::phase(
      PhaseName::Reset,
      PhaseStatus::Passed,
      duration(|value| value.reset_ms),
      None,
    ),
    self::phase(
      PhaseName::Durability,
      PhaseStatus::Passed,
      duration(|value| value.durability_ms),
      None,
    ),
  ]
}

fn phase(
  name: PhaseName,
  status: PhaseStatus,
  duration_ms: u64,
  expired_deadline: Option<DeadlineKind>,
) -> PhaseResult {
  PhaseResult {
    name,
    status,
    duration_ms,
    expired_deadline,
    log_path: None,
    error_ids: Vec::new(),
  }
}

fn phase_with_log(name: PhaseName, log_path: String) -> PhaseResult {
  PhaseResult {
    name,
    status: PhaseStatus::Passed,
    duration_ms: 0,
    expired_deadline: None,
    log_path: Some(log_path),
    error_ids: Vec::new(),
  }
}

fn elapsed_ms(started: Instant) -> u64 {
  started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}
