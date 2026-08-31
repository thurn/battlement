//! Warm macOS player ownership across immutable watch jobs.

use std::{
  fs,
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
  thread,
  time::{Duration, Instant},
};

use anyhow::{Context, Result, ensure};
use battlement_tooling::macos_build;

use crate::{
  macos_capture::{MacosCaptureOutcome, MacosCaptureRequest, MacosPlayerLauncher},
  player_supervision::PlayerSupervisor,
  scenario_orchestration::{ScenarioMaterializer, ScenarioOrchestrator},
  session_server::PlayerSessionServer,
  wire::{
    common::DeadlineKind,
    lifecycle::{NextAction, StartupIdentity, StartupReport},
    result::{PhaseName, PhaseResult, PhaseStatus, PlayerSessionResult, ScenarioStatus},
  },
};

/// Result of launching a player for the first watch job.
pub(crate) struct WarmLaunch {
  pub player: Option<WarmMacosPlayer>,
  pub outcome: MacosCaptureOutcome,
}

/// One accepted macOS player and HTTP session waiting between watch jobs.
pub(crate) struct WarmMacosPlayer {
  server: PlayerSessionServer,
  supervisor: PlayerSupervisor,
  startup_report: StartupReport,
  player_log_source: std::path::PathBuf,
  timeouts: crate::macos_capture::MacosCaptureTimeouts,
}

impl WarmMacosPlayer {
  /// Launches and executes the first immutable job without closing the accepted session.
  pub fn launch(
    request: MacosCaptureRequest<'_>,
    launcher: &dyn MacosPlayerLauncher,
    materializer: Arc<dyn ScenarioMaterializer>,
    interrupted: &AtomicBool,
  ) -> Result<WarmLaunch> {
    validate(&request)?;
    let player_session_id = uuid::Uuid::new_v4().to_string();
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
      player_session_id,
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
      request.timeouts.startup,
      request.timeouts.poll_interval,
    )?;
    let mut phases = vec![phase(
      PhaseName::Launch,
      PhaseStatus::Passed,
      launch_duration,
    )];
    let Some(startup) = startup else {
      server.expire();
      phases.push(phase(
        PhaseName::Startup,
        PhaseStatus::Interrupted,
        elapsed_ms(startup_started),
      ));
      let diagnostic = retain_log(
        &request.player_log_source,
        &request.requirements.storage_directory,
        server.player_session_id(),
      )?;
      return Ok(WarmLaunch {
        player: None,
        outcome: MacosCaptureOutcome {
          exit_code: 130,
          player_exit: None,
          player_session: startup_report(&server).map(|startup_report| PlayerSessionResult {
            player_session_id: server.player_session_id().to_owned(),
            accepted: false,
            startup_report,
            diagnostic_paths: vec![diagnostic],
          }),
          orchestration: orchestrator.snapshot(),
          phases,
        },
      });
    };
    let report = report(&startup.started.identity)
      .context("fresh macOS player did not report startup facts")?;
    if startup.decision.action != NextAction::Continue {
      server.expire();
      phases.push(phase(
        PhaseName::Startup,
        PhaseStatus::Failed,
        elapsed_ms(startup_started),
      ));
      let diagnostic = retain_log(
        &request.player_log_source,
        &request.requirements.storage_directory,
        server.player_session_id(),
      )?;
      return Ok(WarmLaunch {
        player: None,
        outcome: MacosCaptureOutcome {
          exit_code: 2,
          player_exit: None,
          player_session: Some(PlayerSessionResult {
            player_session_id: server.player_session_id().to_owned(),
            accepted: false,
            startup_report: report,
            diagnostic_paths: vec![diagnostic],
          }),
          orchestration: orchestrator.snapshot(),
          phases,
        },
      });
    }
    phases.push(phase(
      PhaseName::Startup,
      PhaseStatus::Passed,
      elapsed_ms(startup_started),
    ));
    let mut player = Self {
      server,
      supervisor,
      startup_report: report,
      player_log_source: request.player_log_source,
      timeouts: request.timeouts,
    };
    let outcome = player.finish_job(
      orchestrator,
      &request.requirements.storage_directory,
      request.job.remaining_run_timeout_ms,
      interrupted,
      phases,
    )?;
    Ok(WarmLaunch {
      player: (outcome.exit_code != 130).then_some(player),
      outcome,
    })
  }

  /// Reports whether the idle target remains available without creating a run failure.
  pub fn is_alive(&mut self) -> Result<bool> {
    Ok(self.supervisor.poll()?.is_none())
  }

  /// Installs and executes another immutable job on the accepted session.
  pub fn execute(
    &mut self,
    request: MacosCaptureRequest<'_>,
    materializer: Arc<dyn ScenarioMaterializer>,
    interrupted: &AtomicBool,
  ) -> Result<MacosCaptureOutcome> {
    validate(&request)?;
    ensure!(
      request.job.profile.build_fingerprint == self.startup_report.build_fingerprint,
      "warm player build fingerprint changed"
    );
    ensure!(
      request.job.profile.source_fingerprint == self.startup_report.source_fingerprint,
      "warm player source fingerprint changed"
    );
    let origin = Instant::now();
    let now = Arc::new(move || origin.elapsed().as_millis() as u64);
    let orchestrator = Arc::new(ScenarioOrchestrator::new(
      request.job.clone(),
      self.server.player_session_id().to_owned(),
      request.orchestration_path,
      request.bail_after,
      now,
      materializer,
    )?);
    let run_timeout_ms = request.job.remaining_run_timeout_ms;
    self.server.wait_for_next_job(self.timeouts.startup)?;
    self.server.install_job(
      request.job,
      request.requirements.storage_directory.clone(),
      orchestrator.clone(),
    )?;
    let startup_started = Instant::now();
    let startup = wait_for_startup(
      &self.server,
      &mut self.supervisor,
      interrupted,
      self.timeouts.startup,
      self.timeouts.poll_interval,
    )?
    .context("warm player startup was interrupted")?;
    ensure!(
      matches!(startup.started.identity, StartupIdentity::Accepted(_)),
      "warm player repeated its startup report"
    );
    ensure!(
      startup.decision.action == NextAction::Continue,
      "warm player job was rejected"
    );
    self.finish_job(
      orchestrator,
      &request.requirements.storage_directory,
      run_timeout_ms,
      interrupted,
      vec![phase(
        PhaseName::Startup,
        PhaseStatus::Passed,
        elapsed_ms(startup_started),
      )],
    )
  }

  /// Ends the exact warm session and lets the player exit after its long poll.
  pub fn shutdown(mut self) {
    self.server.expire();
    let started = Instant::now();
    while started.elapsed() < self.timeouts.shutdown {
      if self.supervisor.poll().ok().flatten().is_some() {
        return;
      }
      thread::sleep(self.timeouts.poll_interval);
    }
  }

  fn finish_job(
    &mut self,
    orchestrator: Arc<ScenarioOrchestrator>,
    directory: &std::path::Path,
    run_timeout_ms: u64,
    interrupted: &AtomicBool,
    mut phases: Vec<PhaseResult>,
  ) -> Result<MacosCaptureOutcome> {
    let run_started = Instant::now();
    let terminal = loop {
      if interrupted.load(Ordering::Acquire) {
        break false;
      }
      if self.server.durable_state().terminal.is_some() {
        break true;
      }
      if let Some(status) = self.supervisor.poll()? {
        self.server.expire();
        anyhow::bail!("warm macOS player exited during a dispatched job: {status:?}");
      }
      ensure!(
        run_started.elapsed() < Duration::from_millis(run_timeout_ms),
        "warm macOS job exceeded the run deadline"
      );
      thread::sleep(self.timeouts.poll_interval);
    };
    let snapshot = orchestrator.snapshot();
    let exit_code = if terminal {
      execution_exit_code(&snapshot)
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
    ));
    phases.extend(boundary_phases(&snapshot));
    let diagnostic = retain_log(
      &self.player_log_source,
      directory,
      self.server.player_session_id(),
    )?;
    Ok(MacosCaptureOutcome {
      exit_code,
      player_exit: None,
      player_session: Some(PlayerSessionResult {
        player_session_id: self.server.player_session_id().to_owned(),
        accepted: true,
        startup_report: self.startup_report.clone(),
        diagnostic_paths: vec![diagnostic],
      }),
      orchestration: snapshot,
      phases,
    })
  }
}

fn validate(request: &MacosCaptureRequest<'_>) -> Result<()> {
  request.job.validate()?;
  let identity = macos_build::macos_startup_identity(request.build)?;
  ensure!(identity.diagnostics, "macOS diagnostics are disabled");
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
  Ok(())
}

fn wait_for_startup(
  server: &PlayerSessionServer,
  supervisor: &mut PlayerSupervisor,
  interrupted: &AtomicBool,
  timeout: Duration,
  poll_interval: Duration,
) -> Result<Option<crate::session_server::StartupFact>> {
  let started = Instant::now();
  loop {
    if let Some(startup) = server.snapshot().startup {
      return Ok(Some(startup));
    }
    if interrupted.load(Ordering::Acquire) {
      return Ok(None);
    }
    if let Some(status) = supervisor.poll()? {
      anyhow::bail!("macOS player exited before watch startup: {status:?}");
    }
    ensure!(
      started.elapsed() < timeout,
      "macOS watch startup deadline expired"
    );
    thread::sleep(poll_interval);
  }
}

fn startup_report(server: &PlayerSessionServer) -> Option<StartupReport> {
  server
    .snapshot()
    .startup
    .and_then(|startup| report(&startup.started.identity))
}

fn report(identity: &StartupIdentity) -> Option<StartupReport> {
  match identity {
    StartupIdentity::Report(identity) => Some(identity.startup_report.clone()),
    StartupIdentity::Accepted(_) => None,
  }
}

fn retain_log(
  source: &std::path::Path,
  directory: &std::path::Path,
  session_id: &str,
) -> Result<String> {
  ensure!(source.is_file(), "macOS player log was not created");
  let relative = format!("logs/player-{session_id}.log");
  let destination = directory.join(&relative);
  if let Some(parent) = destination.parent() {
    fs::create_dir_all(parent)?;
  }
  fs::copy(source, destination).context("retain scoped macOS player log")?;
  Ok(relative)
}

fn execution_exit_code(
  snapshot: &crate::scenario_orchestration::ScenarioOrchestrationSnapshot,
) -> u8 {
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

fn boundary_phases(
  snapshot: &crate::scenario_orchestration::ScenarioOrchestrationSnapshot,
) -> [PhaseResult; 2] {
  let total = |read: fn(&crate::wire::result::ScenarioTimings) -> Option<u64>| {
    snapshot
      .scenarios
      .iter()
      .filter_map(|scenario| read(&scenario.timings))
      .sum()
  };
  [
    phase(
      PhaseName::Reset,
      PhaseStatus::Passed,
      total(|timings| timings.reset_ms),
    ),
    phase(
      PhaseName::Durability,
      PhaseStatus::Passed,
      total(|timings| timings.durability_ms),
    ),
  ]
}

fn phase(name: PhaseName, status: PhaseStatus, duration_ms: u64) -> PhaseResult {
  PhaseResult {
    name,
    status,
    duration_ms,
    expired_deadline: None::<DeadlineKind>,
    log_path: None,
    error_ids: Vec::new(),
  }
}

fn elapsed_ms(started: Instant) -> u64 {
  started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}
