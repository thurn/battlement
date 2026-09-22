//! Persistence boundary tests deliberately exercise bytes; gameplay fixtures do not.
use chess_rules::{PersistenceBackend, persistence};
use std::{
  cell::{Cell, RefCell},
  collections::BTreeMap,
  path::{Path, PathBuf},
  rc::Rc,
};
#[derive(Default)]
pub struct MemoryPersistence {
  values: RefCell<BTreeMap<PathBuf, Vec<u8>>>,
  fail_load: Cell<bool>,
  fail_store: Cell<bool>,
  fail_remove: Cell<bool>,
}

impl MemoryPersistence {
  pub fn with_save(bytes: &[u8]) -> Rc<Self> {
    let storage = Rc::new(Self::default());
    storage
      .values
      .borrow_mut()
      .insert(save_path(), bytes.to_vec());
    storage
  }

  pub fn empty() -> Rc<Self> {
    Rc::new(Self::default())
  }

  pub fn fail_load(&self) {
    self.fail_load.set(true);
  }

  pub fn fail_store(&self) {
    self.fail_store.set(true);
  }

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

fn save_path() -> PathBuf {
  PathBuf::from("memory").join(persistence::SAVE_FILE_NAME)
}
