//! Owned-player liveness and crash reconstruction.

use std::{collections::BTreeMap, process::Child};

use anyhow::Result;

use crate::{
  crash_reconstruction,
  scenario_orchestration::ScenarioMaterializer,
  session_server::PlayerSessionDurableState,
  wire::{
    job::Job,
    lifecycle::StartupReport,
    result::{ErrorOccurrence, JobResult, PlayerSessionResult, ScenarioResult},
  },
};

/// The owned target whose liveness is being observed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupervisedPlatform {
  Macos,
  Webgl,
  IosSimulator,
}

/// One terminal liveness observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlayerExitStatus {
  pub platform: SupervisedPlatform,
  pub code: Option<i32>,
}

/// An app-scoped Simulator liveness and cleanup adapter.
pub trait SimulatorApp: Send {
  fn is_running(&mut self) -> Result<bool>;
  fn terminate(&mut self) -> Result<()>;
}

/// Polls one owned process or Simulator application without affecting unrelated targets.
pub struct PlayerSupervisor {
  target: Target,
  exit_status: Option<PlayerExitStatus>,
}

enum Target {
  Child {
    platform: SupervisedPlatform,
    child: Child,
  },
  Simulator(Box<dyn SimulatorApp>),
}

impl PlayerSupervisor {
  /// Takes ownership of a launched macOS player process.
  pub fn macos(child: Child) -> Self {
    Self::child(SupervisedPlatform::Macos, child)
  }

  /// Takes ownership of a configured WebGL launcher command.
  pub fn webgl(child: Child) -> Self {
    Self::child(SupervisedPlatform::Webgl, child)
  }

  /// Takes ownership of one app-scoped Simulator adapter.
  pub fn ios_simulator(app: Box<dyn SimulatorApp>) -> Self {
    Self {
      target: Target::Simulator(app),
      exit_status: None,
    }
  }

  /// Reports an exit once, leaving a running target untouched.
  pub fn poll(&mut self) -> Result<Option<PlayerExitStatus>> {
    if self.exit_status.is_some() {
      return Ok(None);
    }
    let status = match &mut self.target {
      Target::Child { platform, child } => child.try_wait()?.map(|status| PlayerExitStatus {
        platform: *platform,
        code: status.code(),
      }),
      Target::Simulator(app) => (!app.is_running()?).then_some(PlayerExitStatus {
        platform: SupervisedPlatform::IosSimulator,
        code: None,
      }),
    };
    self.exit_status = status;
    Ok(status)
  }

  pub(crate) fn is_alive(&mut self) -> Result<bool> {
    self.poll()?;
    Ok(self.exit_status.is_none())
  }

  /// Stops and reaps the owned target, preserving an already observed exit.
  pub fn stop(&mut self) -> Result<PlayerExitStatus> {
    if let Some(status) = self.exit_status {
      return Ok(status);
    }
    let status = match &mut self.target {
      Target::Child { platform, child } => {
        let exit = match child.try_wait()? {
          Some(exit) => exit,
          None => {
            child.kill()?;
            child.wait()?
          }
        };
        PlayerExitStatus {
          platform: *platform,
          code: exit.code(),
        }
      }
      Target::Simulator(app) => {
        app.terminate()?;
        PlayerExitStatus {
          platform: SupervisedPlatform::IosSimulator,
          code: None,
        }
      }
    };
    self.exit_status = Some(status);
    Ok(status)
  }

  fn child(platform: SupervisedPlatform, child: Child) -> Self {
    Self {
      target: Target::Child { platform, child },
      exit_status: None,
    }
  }
}

impl Drop for PlayerSupervisor {
  fn drop(&mut self) {
    let _ = self.stop();
  }
}

/// All durable facts used to classify one observed player loss.
pub struct PlayerExitContext {
  pub active_run: bool,
  pub job: Job,
  pub player_session_id: String,
  pub startup_report: StartupReport,
  pub durable: PlayerSessionDurableState,
  pub player_error_ids: BTreeMap<String, String>,
  pub log_path: String,
  pub diagnostic_paths: Vec<String>,
}

/// Exact durable records synthesized or retained after a player loss.
pub struct PlayerExitRecovery {
  pub stale_session: bool,
  pub player_session: PlayerSessionResult,
  pub job: Option<JobResult>,
  pub scenario: Option<ScenarioResult>,
  pub occurrence: Option<ErrorOccurrence>,
  pub recovery_job: Option<Job>,
  pub retained_artifact_ids: Vec<String>,
}

/// Reconstructs one player loss without retrying a reached scenario.
pub fn reconstruct_player_exit(
  context: PlayerExitContext,
  error_id: &str,
  remaining_run_timeout_ms: u64,
  materializer: &dyn ScenarioMaterializer,
) -> Result<PlayerExitRecovery> {
  crash_reconstruction::reconstruct(context, error_id, remaining_run_timeout_ms, materializer)
}
