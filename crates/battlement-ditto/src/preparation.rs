use anyhow::Result;
use battlement_tooling::build_control::BuildControl;

use crate::config::model::Suite;

/// Prepares owner-specific project inputs before Ditto resolves a player build.
pub trait PlayerPreparation: Send + Sync {
  fn prepare(&self, suite: &Suite, control: BuildControl<'_>) -> Result<()>;
}

/// Leaves direct Battlement projects unchanged.
pub struct NoPlayerPreparation;

impl PlayerPreparation for NoPlayerPreparation {
  fn prepare(&self, _suite: &Suite, control: BuildControl<'_>) -> Result<()> {
    control.check()
  }
}
