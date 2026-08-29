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
  observed: bool,
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
      observed: false,
    }
  }

  /// Reports an exit once, leaving a running target untouched.
  pub fn poll(&mut self) -> Result<Option<PlayerExitStatus>> {
    if self.observed {
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
    self.observed = status.is_some();
    Ok(status)
  }

  fn child(platform: SupervisedPlatform, child: Child) -> Self {
    Self {
      target: Target::Child { platform, child },
      observed: false,
    }
  }
}

impl Drop for PlayerSupervisor {
  fn drop(&mut self) {
    if self.observed {
      return;
    }
    match &mut self.target {
      Target::Child { child, .. } => {
        if child.try_wait().ok().flatten().is_none() {
          let _ = child.kill();
          let _ = child.wait();
        }
      }
      Target::Simulator(app) => {
        let _ = app.terminate();
      }
    }
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
