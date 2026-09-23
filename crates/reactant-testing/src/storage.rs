//! Persistence boundary tests deliberately exercise bytes; gameplay fixtures do not.
use reactant::PersistenceBackend;
use std::{
  cell::{Cell, RefCell},
  collections::BTreeMap,
  path::{Path, PathBuf},
  rc::Rc,
};
/// In-memory persistence with injectable failures at the external storage boundary.
#[derive(Default)]
pub struct MemoryPersistence {
  values: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
  fail_load: Cell<bool>,
  fail_store: Cell<bool>,
  fail_remove: Cell<bool>,
}

impl MemoryPersistence {
  /// Seeds bytes without using the filesystem.
  pub fn with_file(path: impl Into<PathBuf>, bytes: &[u8]) -> Rc<Self> {
    let storage = Rc::new(Self::default());
    storage
      .values
      .borrow_mut()
      .insert(path.into(), bytes.to_vec());
    storage
  }

  /// Creates fresh shared storage.
  pub fn empty() -> Rc<Self> {
    Rc::new(Self::default())
  }

  /// Fails subsequent loads.
  pub fn fail_load(&self) {
    self.fail_load.set(true);
  }

  /// Fails subsequent writes.
  pub fn fail_store(&self) {
    self.fail_store.set(true);
  }

  /// Fails subsequent deletes.
  pub fn fail_remove(&self) {
    self.fail_remove.set(true);
  }
}

impl PersistenceBackend for MemoryPersistence {
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    if self.fail_load.get() {
      return Err("injected load failure".to_owned());
    }
    Ok(self.values.borrow().get(path).cloned())
  }

  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    if self.fail_store.get() {
      return Err("injected store failure".to_owned());
    }
    self
      .values
      .borrow_mut()
      .insert(path.to_owned(), bytes.to_vec());
    Ok(())
  }

  fn remove(&self, path: &Path) -> Result<(), String> {
    if self.fail_remove.get() {
      return Err("injected remove failure".to_owned());
    }
    self.values.borrow_mut().remove(path);
    Ok(())
  }
}
