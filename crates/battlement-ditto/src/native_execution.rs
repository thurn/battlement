//! Process-local identity and capture ownership for a native Ditto player.

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use anyhow::{Result, bail};
use uuid::Uuid;

/// Identity and single-capture guard owned by one Ditto execution.
#[derive(Debug)]
pub struct NativeExecution {
  id: String,
  capture_active: AtomicBool,
}

impl NativeExecution {
  /// Creates an isolated native execution identity without acquiring a machine-wide resource.
  #[must_use]
  pub fn create() -> Self {
    Self {
      id: Uuid::new_v4().to_string(),
      capture_active: AtomicBool::new(false),
    }
  }

  /// Returns the opaque identity passed directly to this execution's player process.
  pub fn id(&self) -> &str {
    &self.id
  }

  pub(crate) fn claim_capture(self: &Arc<Self>) -> Result<NativeExecutionClaim> {
    if self
      .capture_active
      .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
      .is_err()
    {
      bail!("native execution is already launching or supervising a player");
    }
    Ok(NativeExecutionClaim(self.clone()))
  }
}

pub(crate) struct NativeExecutionClaim(Arc<NativeExecution>);

impl Drop for NativeExecutionClaim {
  fn drop(&mut self) {
    self.0.capture_active.store(false, Ordering::Release);
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::NativeExecution;

  #[test]
  fn independent_native_executions_have_isolated_identities() {
    let first = NativeExecution::create();
    let second = NativeExecution::create();

    assert_ne!(first.id(), second.id());
  }

  #[test]
  fn one_execution_cannot_launch_two_captures() {
    let execution = Arc::new(NativeExecution::create());
    let first = execution.claim_capture().unwrap();

    assert!(execution.claim_capture().is_err());
    drop(first);
    assert!(execution.claim_capture().is_ok());
  }

  #[test]
  fn independent_executions_can_capture_concurrently() {
    let first = Arc::new(NativeExecution::create());
    let second = Arc::new(NativeExecution::create());

    let _first_claim = first.claim_capture().unwrap();
    let _second_claim = second.claim_capture().unwrap();
  }
}
