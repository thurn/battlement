//! Bounded execution of one immutable WebGL capture player.

use std::{
  fs::{self, File},
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
  webgl_build::{self, WebglStartupIdentity},
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

/// Host phase limits for a launched WebGL player.
#[derive(Clone, Copy, Debug)]
pub struct WebglCaptureTimeouts {
  pub launch: Duration,
  pub shutdown: Duration,
  pub interrupt_grace: Duration,
  pub poll_interval: Duration,
}

/// Immutable inputs for one WebGL player job.
pub struct WebglCaptureRequest<'a> {
  pub build: &'a BuildHandle,
  pub job: Job,
  pub requirements: PlayerSessionRequirements,
  pub orchestration_path: PathBuf,
  pub browser_log_source: PathBuf,
  pub bail_after: Option<u32>,
  pub headless_command: Option<&'a [String]>,
  pub timeouts: WebglCaptureTimeouts,
}

/// Result of opening a WebGL launcher locally.
pub struct WebglLaunch {
  child: Option<Child>,
}

/// Opens the exact same-origin launcher URL.
pub trait WebglPlayerLauncher {
  fn launch(
    &self,
    launcher_url: &str,
    headless_command: Option<&[String]>,
    log_path: &Path,
  ) -> Result<WebglLaunch>;
}

/// Production operating-system and configured-command launcher.
pub struct LocalWebglLauncher;

/// Durable WebGL capture facts ready for a terminal run result.
#[derive(Debug)]
pub struct WebglCaptureOutcome {
  pub exit_code: u8,
  pub player_exit: Option<PlayerExitStatus>,
  pub player_session: Option<PlayerSessionResult>,
  pub orchestration: ScenarioOrchestrationSnapshot,
  pub phases: Vec<PhaseResult>,
}

impl WebglLaunch {
  /// Creates a launch whose browser process is not observable by the host.
  pub fn operating_system() -> Self {
    Self { child: None }
  }

  /// Creates a launch backed by one directly supervised command.
  pub fn supervised(child: Child) -> Self {
    Self { child: Some(child) }
  }
}

impl WebglPlayerLauncher for LocalWebglLauncher {
  fn launch(
    &self,
    launcher_url: &str,
    headless_command: Option<&[String]>,
    log_path: &Path,
  ) -> Result<WebglLaunch> {
    if let Some(arguments) = headless_command {
      ensure!(!arguments.is_empty(), "headless command is empty");
      let replaced = arguments
        .iter()
        .map(|value| {
          if value == "{url}" {
            launcher_url.to_owned()
          } else {
            value.clone()
          }
        })
        .collect::<Vec<_>>();
      ensure!(
        replaced.iter().all(|value| value != "{url}"),
        "headless command did not resolve its launcher URL"
      );
      let output = File::create(log_path)?;
      let child = Command::new(&replaced[0])
        .args(&replaced[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::from(output.try_clone()?))
        .stderr(Stdio::from(output))
        .spawn()
        .with_context(|| format!("launch configured WebGL command {}", replaced[0]))?;
      return Ok(WebglLaunch::supervised(child));
    }
    let status = self
      .open_command(launcher_url)
      .status()
      .context("open WebGL launcher with the operating system")?;
    ensure!(
      status.success(),
      "operating-system browser opener exited with {status}"
    );
    Ok(WebglLaunch::operating_system())
  }
}

impl WebglCaptureOutcome {
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

/// Launches, validates, supervises, and drains one WebGL capture job.
pub fn capture_webgl(
  mut request: WebglCaptureRequest<'_>,
  launcher: &dyn WebglPlayerLauncher,
  materializer: Arc<dyn ScenarioMaterializer>,
  interrupted: &AtomicBool,
) -> Result<WebglCaptureOutcome> {
  let identity = self::validate_build(&request)?;
  self::validate_timeouts(request.timeouts)?;
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
  request.requirements.origin = None;
  let server = PlayerSessionServer::bind_webgl_with_identity(
    request.job.clone(),
    request.requirements.clone(),
    player_session_id.clone(),
    orchestrator.clone(),
    &request.build.player_path(),
  )?;
  let launch_started = Instant::now();
  let launch = launcher.launch(
    &server.launcher_url(),
    request.headless_command,
    &request.browser_log_source,
  )?;
  let mut supervisor = launch.child.map(PlayerSupervisor::webgl);
  let mut phases = vec![self::phase(
    PhaseName::Launch,
    PhaseStatus::Passed,
    self::elapsed_ms(launch_started),
    None,
  )];
  let startup_started = Instant::now();
  let startup = self::wait_for_startup(
    &server,
    supervisor.as_mut(),
    interrupted,
    request.timeouts,
    startup_started,
  )?;
  let Some(startup) = startup else {
    server.expire();
    let player_exit = self::wait_for_exit(
      supervisor.as_mut(),
      request.timeouts.interrupt_grace,
      request.timeouts.poll_interval,
    )?;
    phases.push(self::phase(
      PhaseName::Startup,
      PhaseStatus::Interrupted,
      self::elapsed_ms(startup_started),
      None,
    ));
    return Ok(WebglCaptureOutcome {
      exit_code: 130,
      player_exit,
      player_session: None,
      orchestration: orchestrator.snapshot(),
      phases: self::with_cleanup(phases),
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
      supervisor.as_mut(),
      request.timeouts.shutdown,
      request.timeouts.poll_interval,
    )?;
    let diagnostics = self::retain_browser_log(
      &request.browser_log_source,
      &request.requirements.storage_directory,
      &player_session_id,
    )?;
    return Ok(WebglCaptureOutcome {
      exit_code: 2,
      player_exit,
      player_session: report.map(|startup_report| PlayerSessionResult {
        player_session_id,
        accepted: false,
        startup_report,
        diagnostic_paths: diagnostics.clone(),
      }),
      orchestration: orchestrator.snapshot(),
      phases: self::with_cleanup(phases),
    });
  }
  let report = report.context("fresh WebGL player did not report startup facts")?;
  ensure!(
    report.diagnostics && identity.diagnostics,
    "WebGL diagnostics are disabled"
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
    if let Some(status) = self::poll(supervisor.as_mut())? {
      anyhow::bail!("WebGL browser exited before durable job completion: {status:?}");
    }
    ensure!(
      run_started.elapsed() < Duration::from_millis(request.job.remaining_run_timeout_ms),
      "WebGL capture exceeded the run deadline"
    );
    thread::sleep(request.timeouts.poll_interval);
  };
  server.expire();
  let player_exit = self::wait_for_exit(
    supervisor.as_mut(),
    if terminal {
      request.timeouts.shutdown
    } else {
      request.timeouts.interrupt_grace
    },
    request.timeouts.poll_interval,
  )?;
  let orchestration = orchestrator.snapshot();
  let exit_code = if terminal {
    self::execution_exit_code(&orchestration)
  } else {
    130
  };
  phases.push(self::phase(
    PhaseName::Scenarios,
    if !terminal {
      PhaseStatus::Interrupted
    } else if exit_code == 0 {
      PhaseStatus::Passed
    } else {
      PhaseStatus::Failed
    },
    self::elapsed_ms(run_started),
    None,
  ));
  phases.extend(self::boundary_phases(&orchestration));
  phases.push(self::phase(
    PhaseName::Cleanup,
    PhaseStatus::Passed,
    0,
    None,
  ));
  Ok(WebglCaptureOutcome {
    exit_code,
    player_exit,
    player_session: Some(PlayerSessionResult {
      player_session_id: player_session_id.clone(),
      accepted: true,
      startup_report: report,
      diagnostic_paths: self::retain_browser_log(
        &request.browser_log_source,
        &request.requirements.storage_directory,
        &player_session_id,
      )?,
    }),
    orchestration,
    phases,
  })
}

impl LocalWebglLauncher {
  #[cfg(target_os = "macos")]
  fn open_command(&self, url: &str) -> Command {
    let mut command = Command::new("/usr/bin/open");
    command.arg(url);
    command
  }

  #[cfg(target_os = "linux")]
  fn open_command(&self, url: &str) -> Command {
    let mut command = Command::new("xdg-open");
    command.arg(url);
    command
  }

  #[cfg(windows)]
  fn open_command(&self, url: &str) -> Command {
    let mut command = Command::new("cmd");
    command.args(["/C", "start", "", url]);
    command
  }
}

fn validate_build(request: &WebglCaptureRequest<'_>) -> Result<WebglStartupIdentity> {
  request.job.validate()?;
  ensure!(
    matches!(request.job.command, JobCommand::Run | JobCommand::Capture),
    "WebGL launcher requires an execution job"
  );
  ensure!(
    request.job.profile.platform == Platform::Webgl,
    "capture profile is not WebGL"
  );
  let identity = webgl_build::webgl_startup_identity(request.build)?;
  ensure!(
    identity.diagnostics,
    "immutable WebGL build has diagnostics disabled"
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

fn validate_timeouts(timeouts: WebglCaptureTimeouts) -> Result<()> {
  ensure!(
    !timeouts.launch.is_zero(),
    "launch timeout must be positive"
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
  mut supervisor: Option<&mut PlayerSupervisor>,
  interrupted: &AtomicBool,
  timeouts: WebglCaptureTimeouts,
  started: Instant,
) -> Result<Option<crate::session_server::StartupFact>> {
  loop {
    if let Some(startup) = server.snapshot().startup {
      return Ok(Some(startup));
    }
    if interrupted.load(Ordering::Acquire) {
      return Ok(None);
    }
    if let Some(status) = self::poll(supervisor.as_deref_mut())? {
      anyhow::bail!("WebGL browser exited before startup: {status:?}");
    }
    ensure!(
      started.elapsed() < timeouts.launch,
      "WebGL launch deadline expired"
    );
    thread::sleep(timeouts.poll_interval);
  }
}

fn poll(supervisor: Option<&mut PlayerSupervisor>) -> Result<Option<PlayerExitStatus>> {
  supervisor
    .map(PlayerSupervisor::poll)
    .transpose()
    .map(Option::flatten)
}

fn wait_for_exit(
  mut supervisor: Option<&mut PlayerSupervisor>,
  timeout: Duration,
  poll_interval: Duration,
) -> Result<Option<PlayerExitStatus>> {
  let started = Instant::now();
  while started.elapsed() < timeout {
    if let Some(status) = self::poll(supervisor.as_deref_mut())? {
      return Ok(Some(status));
    }
    if supervisor.is_none() {
      return Ok(None);
    }
    thread::sleep(poll_interval);
  }
  Ok(None)
}

fn retain_browser_log(source: &Path, directory: &Path, session_id: &str) -> Result<Vec<String>> {
  if !source.is_file() {
    return Ok(Vec::new());
  }
  let relative = format!("logs/browser-{session_id}.log");
  fs::copy(source, directory.join(&relative)).context("retain supervised browser output")?;
  Ok(vec![relative])
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
      duration(|timings| timings.reset_ms),
      None,
    ),
    self::phase(
      PhaseName::Durability,
      PhaseStatus::Passed,
      duration(|timings| timings.durability_ms),
      None,
    ),
  ]
}

fn with_cleanup(mut phases: Vec<PhaseResult>) -> Vec<PhaseResult> {
  phases.push(self::phase(
    PhaseName::Cleanup,
    PhaseStatus::Passed,
    0,
    None,
  ));
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

fn elapsed_ms(started: Instant) -> u64 {
  started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}
