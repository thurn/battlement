//! Persistence boundary tests deliberately exercise bytes; gameplay fixtures do not.
use reactant::{PersistenceBackend, PersistenceCompletion, PersistenceRequest};
use std::{
  collections::BTreeMap,
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
};
/// In-memory persistence with injectable failures at the external storage boundary.
#[derive(Default)]
pub struct MemoryPersistence {
  values: Mutex<BTreeMap<PathBuf, Vec<u8>>>,
  fail_load: AtomicBool,
  fail_store: AtomicBool,
  fail_remove: AtomicBool,
}

impl MemoryPersistence {
  /// Seeds bytes without using the filesystem.
  pub fn with_file(path: impl Into<PathBuf>, bytes: &[u8]) -> Arc<Self> {
    let storage = Arc::new(Self::default());
    storage
      .values
      .lock()
      .unwrap()
      .insert(path.into(), bytes.to_vec());
    storage
  }

  /// Creates fresh shared storage.
  pub fn empty() -> Arc<Self> {
    Arc::new(Self::default())
  }

  /// Fails subsequent loads.
  pub fn fail_load(&self) {
    self.fail_load.store(true, Ordering::SeqCst);
  }

  /// Fails subsequent writes.
  pub fn fail_store(&self) {
    self.fail_store.store(true, Ordering::SeqCst);
  }

  /// Restores successful writes after an injected failure.
  pub fn recover_store(&self) {
    self.fail_store.store(false, Ordering::SeqCst);
  }

  /// Fails subsequent deletes.
  pub fn fail_remove(&self) {
    self.fail_remove.store(true, Ordering::SeqCst);
  }
}

impl PersistenceBackend for MemoryPersistence {
  fn start(self: Arc<Self>, request: PersistenceRequest, complete: PersistenceCompletion) {
    complete(request.id, request.execute(self.as_ref()));
  }
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    if self.fail_load.load(Ordering::SeqCst) {
      return Err("injected load failure".to_owned());
    }
    Ok(self.values.lock().unwrap().get(path).cloned())
  }

  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    if self.fail_store.load(Ordering::SeqCst) {
      return Err("injected store failure".to_owned());
    }
    self
      .values
      .lock()
      .unwrap()
      .insert(path.to_owned(), bytes.to_vec());
    Ok(())
  }

  fn remove(&self, path: &Path) -> Result<(), String> {
    if self.fail_remove.load(Ordering::SeqCst) {
      return Err("injected remove failure".to_owned());
    }
    self.values.lock().unwrap().remove(path);
    Ok(())
  }
}
