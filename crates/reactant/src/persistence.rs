//! Typed component-owned persistent state.

use std::{
  fs,
  io::ErrorKind,
  path::{Path, PathBuf},
  rc::Rc,
};

use reactant_core::{app_context::HostEnvironment, hooks};
use serde::{Serialize, de::DeserializeOwned};

/// A typed persistent value and its non-fatal storage error.
#[derive(Clone)]
pub struct PersistentState<T: Clone + PartialEq + 'static> {
  value: Option<T>,
  error: Option<String>,
  path: Option<PathBuf>,
  backend: Rc<dyn PersistenceBackend>,
  value_setter: hooks::StateSetter<Option<T>>,
  error_setter: hooks::StateSetter<Option<String>>,
}

/// Raw persistent storage used by component-owned serialized state.
pub trait PersistenceBackend {
  /// Loads bytes at a host-resolved path, returning `None` when no value exists.
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String>;

  /// Replaces the bytes at a host-resolved path.
  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String>;

  /// Removes the value at a host-resolved path. Missing values are already removed.
  fn remove(&self, path: &Path) -> Result<(), String>;
}

/// Filesystem-backed persistence used by default.
#[derive(Default)]
pub struct FilePersistenceBackend;

impl PersistenceBackend for FilePersistenceBackend {
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
      Ok(bytes) => Ok(Some(bytes)),
      Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
      Err(error) => Err(error.to_string()),
    }
  }

  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(path, bytes).map_err(|error| error.to_string())
  }

  fn remove(&self, path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
      Ok(()) => Ok(()),
      Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
      Err(error) => Err(error.to_string()),
    }
  }
}

/// Loads a serde value from the host's persistent-data directory.
/// The file name must remain stable for the lifetime of the component.
pub fn use_persistent_state<T>(file_name: impl Into<String>) -> PersistentState<T>
where
  T: Clone + PartialEq + Serialize + DeserializeOwned + 'static,
{
  use_persistent_state_with(file_name, Rc::new(FilePersistenceBackend))
}

/// Loads typed state through an injected raw-byte persistence backend.
pub fn use_persistent_state_with<T>(
  file_name: impl Into<String>,
  backend: Rc<dyn PersistenceBackend>,
) -> PersistentState<T>
where
  T: Clone + PartialEq + Serialize + DeserializeOwned + 'static,
{
  let environment = hooks::use_required_context::<HostEnvironment>();
  let file_name = file_name.into();
  assert!(
    !file_name.is_empty() && std::path::Path::new(&file_name).file_name().is_some(),
    "persistent state requires a file name"
  );
  let initial_file_name = file_name.clone();
  let mounted_file_name = hooks::use_memo(move || initial_file_name, ());
  assert_eq!(
    mounted_file_name, file_name,
    "persistent state file name cannot change while mounted"
  );
  let path = environment
    .persistent_data_path
    .as_ref()
    .map(|directory| directory.join(&mounted_file_name));
  let load_path = path.clone();
  let load_backend = backend.clone();
  let (initial, initial_error) = hooks::use_memo(
    move || match load_path {
      None => (None, None),
      Some(path) => match load_backend.load(&path) {
        Ok(Some(bytes)) => match serde_json::from_slice(&bytes) {
          Ok(value) => (Some(value), None),
          Err(error) => (
            None,
            Some(format!("could not read persistent state: {error}")),
          ),
        },
        Ok(None) => (None, None),
        Err(error) => (
          None,
          Some(format!("could not read persistent state: {error}")),
        ),
      },
    },
    (),
  );
  let (value, value_setter) = hooks::use_state(initial);
  let (error, error_setter) = hooks::use_state(initial_error);
  PersistentState {
    value,
    error,
    path,
    backend,
    value_setter,
    error_setter,
  }
}

impl<T> PersistentState<T>
where
  T: Clone + PartialEq + Serialize + DeserializeOwned + 'static,
{
  /// Returns the last loaded or successfully saved value.
  pub fn value(&self) -> Option<&T> {
    self.value.as_ref()
  }

  /// Returns the latest non-fatal load, save, or clear error.
  pub fn error(&self) -> Option<&str> {
    self.error.as_deref()
  }

  /// Serializes and stores a replacement value.
  pub fn update(&self, value: T) {
    let result = self.path.as_ref().map_or(Ok(()), |path| {
      let bytes = serde_json::to_vec_pretty(&value)
        .map_err(|error| format!("could not serialize persistent state: {error}"))?;
      self
        .backend
        .store(path, &bytes)
        .map_err(|error| format!("could not persist state: {error}"))
    });
    match result {
      Ok(()) => {
        self.value_setter.set(Some(value));
        self.error_setter.set(None);
      }
      Err(error) => self.error_setter.set(Some(error)),
    }
  }

  /// Removes the persisted value. Missing files are already clear.
  pub fn clear(&self) {
    let result = self.path.as_ref().map_or(Ok(()), |path| {
      self
        .backend
        .remove(path)
        .map_err(|error| format!("could not clear persistent state: {error}"))
    });
    match result {
      Ok(()) => {
        self.value_setter.set(None);
        self.error_setter.set(None);
      }
      Err(error) => self.error_setter.set(Some(error)),
    }
  }
}

/// Reports whether the connected host selected a named capability module.
pub fn use_host_module(module: &str) -> bool {
  hooks::use_required_context::<HostEnvironment>()
    .modules
    .iter()
    .any(|selected| selected == module)
}
