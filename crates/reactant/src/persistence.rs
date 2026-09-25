//! Typed component-owned persistent state.

use reactant_core::{app_context::HostEnvironment, hooks};
use serde::{Serialize, de::DeserializeOwned};
use std::{
  path::{Component, Path},
  rc::Rc,
};

use crate::{
  persistence_file::FilePersistenceBackend,
  persistence_operation::{PersistenceBackend, PersistenceId},
  persistence_store::{PersistenceSnapshot, PersistenceStore},
};

/// A rendered persistence snapshot and a stable ordered setter boundary.
#[derive(Clone)]
pub struct PersistentState<T: Clone + PartialEq + 'static> {
  snapshot: PersistenceSnapshot<T>,
  store: PersistenceStore<T>,
}

/// Loads a serde value from the host's persistent-data directory.
/// The file name must remain stable for the lifetime of the component.
pub fn use_persistent_state<T>(file_name: impl Into<String>) -> PersistentState<T>
where
  T: Clone + PartialEq + Serialize + DeserializeOwned + 'static,
{
  self::use_persistent_state_with(file_name, Rc::new(FilePersistenceBackend))
}

/// Uses an injected backend; asynchronous hydration is exposed by `hydrated`.
/// Keep consumers that initialize from storage unmounted until hydration completes.
pub fn use_persistent_state_with<T>(
  file_name: impl Into<String>,
  backend: Rc<dyn PersistenceBackend>,
) -> PersistentState<T>
where
  T: Clone + PartialEq + Serialize + DeserializeOwned + 'static,
{
  let environment = hooks::use_required_context::<HostEnvironment>();
  let file_name = file_name.into();
  let mut components = Path::new(&file_name).components();
  assert!(
    matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none(),
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
  let store = hooks::use_memo(move || PersistenceStore::new(path, backend), ());
  let (_, set_revision) = hooks::use_state(0_u64);
  let subscribed = store.clone();
  hooks::use_effect(
    move || {
      subscribed.listen(Rc::new(move || {
        set_revision.update(|revision| revision.wrapping_add(1))
      }));
      move || subscribed.detach()
    },
    (),
  );
  PersistentState {
    snapshot: store.snapshot(),
    store,
  }
}

impl<T: Clone + PartialEq + Serialize + DeserializeOwned + 'static> PersistentState<T> {
  /// Returns only the last loaded or successfully acknowledged value.
  pub fn value(&self) -> Option<&T> {
    self.snapshot.durable.as_ref()
  }
  /// Returns immediately applied intent, including an unsaved value or deletion.
  pub fn desired(&self) -> Option<&T> {
    self.snapshot.desired.as_ref()
  }
  /// Whether the initial read completed; defaults must not be saved before this.
  pub fn hydrated(&self) -> bool {
    self.snapshot.hydrated
  }
  /// Identity of the operation awaiting acknowledgment.
  pub fn pending(&self) -> Option<PersistenceId> {
    self.snapshot.pending
  }
  /// Returns the latest load, serialization, save, or clear failure.
  pub fn error(&self) -> Option<&str> {
    self.snapshot.error.as_deref()
  }
  /// Applies and queues a replacement, coalescing intermediate queued intent.
  pub fn update(&self, value: T) {
    self.store.update(value);
  }
  /// Derives a replacement from current intent rather than a captured render snapshot.
  pub fn update_with(&self, update: impl FnOnce(Option<T>) -> T) {
    self.store.update_with(update);
  }
  /// Queues deletion after the active operation.
  pub fn clear(&self) {
    self.store.clear();
  }
  /// Retries the latest intent, or the initial read when hydration failed.
  pub fn retry(&self) {
    self.store.retry();
  }
}

/// Reports whether the connected host selected a named capability module.
pub fn use_host_module(module: &str) -> bool {
  hooks::use_required_context::<HostEnvironment>()
    .modules
    .iter()
    .any(|selected| selected == module)
}
