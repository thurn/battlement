use anyhow::Result;

use crate::config::model::Suite;

/// Prepares owner-specific project inputs before Ditto resolves a player build.
pub trait PlayerPreparation: Send + Sync {
  fn prepare(&self, suite: &Suite) -> Result<()>;
}

/// Leaves direct Battlement projects unchanged.
pub struct NoPlayerPreparation;

impl PlayerPreparation for NoPlayerPreparation {
  fn prepare(&self, _suite: &Suite) -> Result<()> {
    Ok(())
  }
}
