//! Correlated storage requests and background backend execution.

use std::path::{Path, PathBuf};
use std::{
  panic::{self, AssertUnwindSafe},
  sync::Arc,
  thread,
};

use uuid::Uuid;

/// A storage request identity, unique across mounted consumers.
pub type PersistenceId = Uuid;

/// Thread-safe completion; duplicate and superseded request identities are ignored.
pub type PersistenceCompletion =
  Arc<dyn Fn(PersistenceId, Result<Option<Vec<u8>>, String>) + Send + Sync>;

/// Identifies immutable intent within one mounted save-slot owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistenceVersion {
  /// Save-slot ownership generation.
  pub owner: Uuid,
  /// Monotonically increasing intent within the owner.
  pub revision: u64,
}

/// Raw persistent storage used by component-owned serialized state.
pub trait PersistenceBackend: Send + Sync + 'static {
  /// Loads bytes, returning `None` when no value exists.
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String>;
  /// Durably replaces the complete value.
  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;
  /// Durably removes the value. Missing values are already removed.
  fn remove(&self, path: &Path) -> Result<(), String>;

  /// Shares slot ordering between backend instances addressing the same physical storage.
  /// The default isolates independently injected storage instances.
  fn namespace(&self) -> Option<&'static str> {
    None
  }

  /// Starts blocking backend work off the application thread and acknowledges durability.
  /// Browser backends override this with their asynchronous host transaction.
  fn start(self: Arc<Self>, request: PersistenceRequest, complete: PersistenceCompletion) {
    let failed = complete.clone();
    let id = request.id;
    let result = thread::Builder::new()
      .name("reactant-storage".into())
      .spawn(move || {
        let result = panic::catch_unwind(AssertUnwindSafe(|| request.execute(self.as_ref())))
          .unwrap_or_else(|_| Err("persistence backend panicked".into()));
        complete(id, result);
      });
    if let Err(error) = result {
      failed(id, Err(error.to_string()));
    }
  }
}

/// An owned request that may outlive the initiating component callback.
#[derive(Clone, Debug)]
pub struct PersistenceRequest {
  /// Echo this identity unchanged in the completion.
  pub id: PersistenceId,
  /// Immutable intent identity, independent of later queued updates.
  pub version: PersistenceVersion,
  /// Host-resolved storage location.
  pub path: PathBuf,
  /// Requested read or mutation.
  pub operation: PersistenceOperation,
}

impl PersistenceRequest {
  /// Executes blocking I/O on a backend worker, or a deterministic in-memory test backend.
  pub fn execute(
    &self,
    backend: &(impl PersistenceBackend + ?Sized),
  ) -> Result<Option<Vec<u8>>, String> {
    match &self.operation {
      PersistenceOperation::Load => backend.load(&self.path),
      PersistenceOperation::Store(bytes) => backend.store(&self.path, bytes).map(|()| None),
      PersistenceOperation::Remove => backend.remove(&self.path).map(|()| None),
    }
  }
}

/// Reads and mutations share one ordered completion channel.
#[derive(Clone, Debug, PartialEq)]
pub enum PersistenceOperation {
  /// Read the complete stored value.
  Load,
  /// Atomically replace the stored bytes.
  Store(Vec<u8>),
  /// Remove the value, accepting absence.
  Remove,
}
