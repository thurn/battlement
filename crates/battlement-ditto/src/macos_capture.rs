//! Bounded execution of one immutable macOS capture player.

use std::{
  fs,
  path::{Path, PathBuf},
  process::{Child, Command, Stdio},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use battlement_tooling::{
  build_cache::BuildHandle,
  macos_build::{self, MacosStartupIdentity},
};
use uuid::Uuid;

use crate::{
  player_supervision::{PlayerExitStatus, PlayerSupervisor},
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

/// Host phase limits for a launched macOS player.
#[derive(Clone, Copy, Debug)]
pub struct MacosCaptureTimeouts {
  pub launch: Duration,
  pub startup: Duration,
  pub shutdown: Duration,
  pub interrupt_grace: Duration,
  pub poll_interval: Duration,
}

/// Immutable inputs for one macOS player job.
pub struct MacosCaptureRequest<'a> {
  pub build: &'a BuildHandle,
  pub job: Job,
  pub requirements: PlayerSessionRequirements,
  pub orchestration_path: PathBuf,
  pub player_log_source: PathBuf,
  pub bail_after: Option<u32>,
  pub timeouts: MacosCaptureTimeouts,
}

/// Launches the exact executable selected by the immutable build handle.
pub trait MacosPlayerLauncher {
  fn launch(
    &self,
    executable: &Path,
    session_url: &str,
    log_path: &Path,
    width: u32,
    height: u32,
  ) -> Result<Child>;
}

/// Production launcher for immutable macOS application executables.
pub struct ImmutableMacosLauncher;

/// Durable capture facts ready to merge into a terminal run result.
#[derive(Debug)]
pub struct MacosCaptureOutcome {
  pub exit_code: u8,
  pub player_exit: Option<PlayerExitStatus>,
  pub player_session: Option<PlayerSessionResult>,
  pub orchestration: ScenarioOrchestrationSnapshot,
  pub phases: Vec<PhaseResult>,
}

impl MacosPlayerLauncher for ImmutableMacosLauncher {
  fn launch(
    &self,
    executable: &Path,
    session_url: &str,
    log_path: &Path,
    width: u32,
    height: u32,
  ) -> Result<Child> {
    Command::new(executable)
      .arg("--battlement-ditto-url")
      .arg(session_url)
      .arg("-screen-width")
      .arg(width.to_string())
      .arg("-screen-height")
      .arg(height.to_string())
      .arg("-screen-fullscreen")
      .arg("0")
      .arg("-logFile")
      .arg(log_path)
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()
      .with_context(|| format!("launch immutable macOS player {}", executable.display()))
  }
}

impl MacosCaptureOutcome {
  /// Applies execution facts without replacing caller-owned build or artifact metadata.
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
    result.scenarios = self.orchestration.scenarios.clone();
    if let Some(session) = &self.player_session {
      for scenario in &mut result.scenarios {
        if let Some(logs) = &mut scenario.logs {
          logs.player_session_id = session.player_session_id.clone();
        }
      }
    }
  }
}

/// Launches, validates, supervises, and drains one macOS capture job.
pub fn capture_macos(
  request: MacosCaptureRequest<'_>,
  launcher: &dyn MacosPlayerLauncher,
  materializer: Arc<dyn ScenarioMaterializer>,
  interrupted: &AtomicBool,
) -> Result<MacosCaptureOutcome> {
  let identity = validate_build(&request)?;
  validate_timeouts(request.timeouts)?;
  let player_session_id = Uuid::new_v4().to_string();
  let origin = Instant::now();
  let now = Arc::new(move || origin.elapsed().as_millis() as u64);
  let orchestrator = Arc::new(ScenarioOrchestrator::new(
    request.job.clone(),
    player_session_id.clone(),
    request.orchestration_path,
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
  let executable = macos_build::player_executable(request.build)?;
  let child = launcher.launch(
    &executable,
    &server.base_url(),
    &request.player_log_source,
    request.job.profile.display.width,
    request.job.profile.display.height,
  )?;
  let mut supervisor = PlayerSupervisor::macos(child);
  let launch_duration = elapsed_ms(launch_started);
  let startup_started = Instant::now();
  let startup = wait_for_startup(
    &server,
    &mut supervisor,
    interrupted,
    request.timeouts,
    startup_started,
  )?;
  let mut phases = vec![phase(
    PhaseName::Launch,
    PhaseStatus::Passed,
    launch_duration,
    None,
  )];
  let Some(startup) = startup else {
    server.expire();
    let player_exit = wait_for_exit(
      &mut supervisor,
      request.timeouts.interrupt_grace,
      request.timeouts.poll_interval,
    )?;
    let diagnostic = retain_player_log(
      &request.player_log_source,
      &request.requirements.storage_directory,
      &player_session_id,
    )?;
    phases.push(phase(
      PhaseName::Startup,
      PhaseStatus::Interrupted,
      elapsed_ms(startup_started),
      None,
    ));
    phases.push(phase_with_log(
      PhaseName::Cleanup,
      PhaseStatus::Passed,
      diagnostic.clone(),
    ));
    let report = server
      .snapshot()
      .startup
      .and_then(|startup| match startup.started.identity {
        StartupIdentity::Report(identity) => Some(identity.startup_report),
        StartupIdentity::Accepted(_) => None,
      });
    return Ok(MacosCaptureOutcome {
      exit_code: 130,
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
  };
  let report = match &startup.started.identity {
    StartupIdentity::Report(identity) => Some(identity.startup_report.clone()),
    StartupIdentity::Accepted(_) => None,
  };
  if startup.decision.action != NextAction::Continue {
    phases.push(phase(
      PhaseName::Startup,
      PhaseStatus::Failed,
      elapsed_ms(startup_started),
      None,
    ));
    server.expire();
    let player_exit = wait_for_exit(
      &mut supervisor,
      request.timeouts.shutdown,
      request.timeouts.poll_interval,
    )?;
    let diagnostic = retain_player_log(
      &request.player_log_source,
      &request.requirements.storage_directory,
      &player_session_id,
    )?;
    return Ok(MacosCaptureOutcome {
      exit_code: 2,
      player_exit,
      player_session: report.map(|startup_report| PlayerSessionResult {
        player_session_id,
        accepted: false,
        startup_report,
        diagnostic_paths: vec![diagnostic],
      }),
      orchestration: orchestrator.snapshot(),
      phases: with_cleanup(phases),
    });
  }
  let report = report.context("fresh macOS player did not report startup facts")?;
  ensure!(
    report.diagnostics && identity.diagnostics,
    "macOS diagnostics are disabled"
  );
  phases.push(phase(
    PhaseName::Startup,
    PhaseStatus::Passed,
    elapsed_ms(startup_started),
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
      anyhow::bail!("macOS player exited before durable job completion: {status:?}");
    }
    if run_started.elapsed() >= Duration::from_millis(request.job.remaining_run_timeout_ms) {
      anyhow::bail!("macOS capture exceeded the run deadline");
    }
    thread::sleep(request.timeouts.poll_interval);
  };
  server.expire();
  let shutdown = if terminal {
    request.timeouts.shutdown
  } else {
    request.timeouts.interrupt_grace
  };
  let player_exit = wait_for_exit(&mut supervisor, shutdown, request.timeouts.poll_interval)?;
  let diagnostic = retain_player_log(
    &request.player_log_source,
    &request.requirements.storage_directory,
    &player_session_id,
  )?;
  let orchestration = orchestrator.snapshot();
  let exit_code = if terminal {
    execution_exit_code(&orchestration)
  } else {
    130
  };
  phases.push(phase(
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
    elapsed_ms(run_started),
    None,
  ));
  phases.extend(boundary_phases(&orchestration));
  phases.push(phase(PhaseName::Cleanup, PhaseStatus::Passed, 0, None));
  Ok(MacosCaptureOutcome {
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

fn validate_build(request: &MacosCaptureRequest<'_>) -> Result<MacosStartupIdentity> {
  request.job.validate()?;
  ensure!(
    matches!(request.job.command, JobCommand::Run | JobCommand::Capture),
    "macOS launcher requires an execution job"
  );
  ensure!(
    request.job.profile.platform == Platform::Macos,
    "capture profile is not macOS"
  );
  let identity = macos_build::macos_startup_identity(request.build)?;
  ensure!(
    identity.diagnostics,
    "immutable macOS build has diagnostics disabled"
  );
  ensure!(
    request.requirements.diagnostics,
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

fn validate_timeouts(timeouts: MacosCaptureTimeouts) -> Result<()> {
  ensure!(
    !timeouts.launch.is_zero(),
    "launch timeout must be positive"
  );
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
  timeouts: MacosCaptureTimeouts,
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
      anyhow::bail!("macOS player exited before startup: {status:?}");
    }
    ensure!(
      started.elapsed() < timeouts.startup,
      "macOS startup deadline expired"
    );
    thread::sleep(timeouts.poll_interval);
  }
}

fn wait_for_exit(
  supervisor: &mut PlayerSupervisor,
  timeout: Duration,
  poll_interval: Duration,
) -> Result<Option<PlayerExitStatus>> {
  let started = Instant::now();
  while started.elapsed() < timeout {
    if let Some(status) = supervisor.poll()? {
      return Ok(Some(status));
    }
    thread::sleep(poll_interval);
  }
  Ok(None)
}

fn retain_player_log(source: &Path, directory: &Path, session_id: &str) -> Result<String> {
  ensure!(source.is_file(), "macOS player log was not created");
  let relative = format!("logs/player-{session_id}.log");
  let destination = directory.join(&relative);
  fs::copy(source, &destination).context("retain scoped macOS player log")?;
  Ok(relative)
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
    phase(
      PhaseName::Reset,
      PhaseStatus::Passed,
      duration(|timings| timings.reset_ms),
      None,
    ),
    phase(
      PhaseName::Durability,
      PhaseStatus::Passed,
      duration(|timings| timings.durability_ms),
      None,
    ),
  ]
}

fn with_cleanup(mut phases: Vec<PhaseResult>) -> Vec<PhaseResult> {
  phases.push(phase(PhaseName::Cleanup, PhaseStatus::Passed, 0, None));
  phases
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

fn phase_with_log(name: PhaseName, status: PhaseStatus, log_path: String) -> PhaseResult {
  PhaseResult {
    name,
    status,
    duration_ms: 0,
    expired_deadline: None,
    log_path: Some(log_path),
    error_ids: Vec::new(),
  }
}

fn elapsed_ms(started: Instant) -> u64 {
  started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}
