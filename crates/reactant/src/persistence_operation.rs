//! Correlated storage requests and synchronous backend adaptation.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use uuid::Uuid;

/// A storage request identity, unique across mounted consumers.
pub type PersistenceId = Uuid;

/// Completion delivered on the application's thread; duplicate/stale IDs are ignored.
pub type PersistenceCompletion = Rc<dyn Fn(PersistenceId, Result<Option<Vec<u8>>, String>)>;

/// Raw persistent storage used by component-owned serialized state.
pub trait PersistenceBackend {
  /// Loads bytes, returning `None` when no value exists.
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String>;
  /// Durably replaces the complete value.
  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;
  /// Durably removes the value. Missing values are already removed.
  fn remove(&self, path: &Path) -> Result<(), String>;

  /// Starts one request. Async backends acknowledge only after durable completion.
  /// Loads complete with bytes or absence; mutations complete with `None`.
  /// The callback must run on the application thread, never a background worker.
  fn start(&self, request: PersistenceRequest, complete: PersistenceCompletion) {
    let result = match request.operation {
      PersistenceOperation::Load => self.load(&request.path),
      PersistenceOperation::Store(bytes) => self.store(&request.path, &bytes).map(|()| None),
      PersistenceOperation::Remove => self.remove(&request.path).map(|()| None),
    };
    complete(request.id, result);
  }
}

/// An owned request that may outlive the initiating component callback.
#[derive(Clone, Debug)]
pub struct PersistenceRequest {
  /// Echo this identity unchanged in the completion.
  pub id: PersistenceId,
  /// Host-resolved storage location.
  pub path: PathBuf,
  /// Requested read or mutation.
  pub operation: PersistenceOperation,
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
