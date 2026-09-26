//! Typed, ordered persistence independent of component lifetimes.

use serde::{Serialize, de::DeserializeOwned};
use std::{
  path::PathBuf,
  sync::{Arc, Condvar, Mutex},
  time::Duration,
};
use uuid::Uuid;

use reactant_core::external_store::{ExternalStore, StoreNotify, Subscription};

use crate::{
  persistence_operation::{
    PersistenceBackend, PersistenceId, PersistenceOperation, PersistenceRequest, PersistenceVersion,
  },
  persistence_slot::PersistenceSlot,
};

/// Observable storage boundary; pending intent is never reported as committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistenceStatus {
  /// The initial read is outstanding.
  Loading,
  /// Hydration found no saved value.
  Absent,
  /// Hydration read a complete saved value.
  Loaded,
  /// A captured mutation is awaiting acknowledgment.
  Pending,
  /// A mutation completed durably.
  Committed,
  /// The latest storage operation failed and can be retried.
  Error,
}

/// Applied intent, acknowledged storage, and outstanding operation state.
#[derive(Clone, Debug, PartialEq)]
pub struct PersistenceSnapshot<T> {
  /// Latest caller intent, including unsaved changes.
  pub desired: Option<T>,
  /// Last loaded or successfully acknowledged value.
  pub durable: Option<T>,
  /// Initial bytes or absence were read; corruption remains visible in error.
  pub hydrated: bool,
  /// Active request identity, distinct from newer queued intent.
  pub pending: Option<PersistenceId>,
  /// Latest load, serialization, write, or deletion failure.
  pub error: Option<String>,
  /// Latest desired intent, captured separately by every dispatched request.
  pub version: PersistenceVersion,
  /// Last successful write by this owner; loading alone is not a new commit.
  pub committed: Option<PersistenceVersion>,
}

/// Serializes a save slot across mounted owners, keeping only the latest queued intent.
#[derive(Clone)]
pub struct PersistenceStore<T> {
  inner: Arc<Mutex<Storage<T>>>,
  notifications: Arc<Mutex<Vec<Arc<StoreNotify>>>>,
  changed: Arc<Condvar>,
}

struct Storage<T> {
  backend: Arc<dyn PersistenceBackend>,
  started: bool,
  accepting: bool,
  path: Option<PathBuf>,
  slot: Option<Arc<PersistenceSlot>>,
  snapshot: PersistenceSnapshot<T>,
  active: Option<Active<T>>,
  queued: Option<Intent<T>>,
}

#[derive(Clone)]
struct Intent<T> {
  value: Option<T>,
  version: PersistenceVersion,
}

struct Prepared {
  id: PersistenceId,
  request: Result<PersistenceRequest, String>,
}

struct Active<T> {
  id: PersistenceId,
  intent: Intent<T>,
  loading: bool,
}

impl<T> PersistenceSnapshot<T> {
  /// Classifies hydration, pending intent, durability and recoverable failure.
  pub fn status(&self) -> PersistenceStatus {
    if !self.hydrated && self.error.is_none() {
      return PersistenceStatus::Loading;
    }
    if self.pending.is_some() {
      return if self.hydrated {
        PersistenceStatus::Pending
      } else {
        PersistenceStatus::Loading
      };
    }
    if self.error.is_some() {
      return PersistenceStatus::Error;
    }
    if self.committed.is_some() {
      return PersistenceStatus::Committed;
    }
    if self.durable.is_some() {
      PersistenceStatus::Loaded
    } else {
      PersistenceStatus::Absent
    }
  }
}

impl<T> PartialEq for PersistenceStore<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.inner, &other.inner)
  }
}

impl<T: Clone + PartialEq + Serialize + DeserializeOwned + Send + Sync + 'static>
  PersistenceStore<T>
{
  /// Claims a slot and begins asynchronous hydration. Older owners cannot start new writes.
  pub fn new(path: Option<PathBuf>, backend: Arc<dyn PersistenceBackend>) -> Self {
    let store = Self::pending(path, backend);
    store.activate();
    store
  }

  pub(crate) fn pending(path: Option<PathBuf>, backend: Arc<dyn PersistenceBackend>) -> Self {
    let version = PersistenceVersion {
      owner: Uuid::new_v4(),
      revision: 0,
    };
    Self {
      inner: Arc::new(Mutex::new(Storage {
        path,
        backend,
        started: false,
        accepting: false,
        slot: None,
        snapshot: PersistenceSnapshot {
          desired: None,
          durable: None,
          hydrated: false,
          pending: None,
          error: None,
          version,
          committed: None,
        },
        active: None,
        queued: None,
      })),
      notifications: Arc::default(),
      changed: Arc::default(),
    }
  }

  pub(crate) fn activate(&self) {
    let (path, backend, owner) = {
      let mut storage = self.inner.lock().unwrap();
      if storage.started {
        return;
      }
      storage.started = true;
      storage.accepting = true;
      (
        storage.path.clone(),
        storage.backend.clone(),
        storage.snapshot.version.owner,
      )
    };
    let slot = path
      .as_ref()
      .map(|path| PersistenceSlot::claim(path, backend, owner));
    let request = {
      let mut storage = self.inner.lock().unwrap();
      storage.slot = slot;
      let version = storage.snapshot.version;
      storage.prepare(
        Intent {
          value: None,
          version,
        },
        true,
      )
    };
    self.notify();
    self.launch(request);
  }

  pub(crate) fn detach(&self) {
    self.inner.lock().unwrap().accepting = false;
  }

  /// Reads a consistent snapshot without blocking on storage work.
  pub fn snapshot(&self) -> PersistenceSnapshot<T> {
    self.inner.lock().unwrap().snapshot.clone()
  }

  /// Captures immutable intent immediately, without claiming storage success.
  pub fn update(&self, value: T) {
    self.enqueue(Some(value));
  }

  /// Derives a replacement atomically from the latest queued intent.
  pub fn update_with(&self, update: impl FnOnce(Option<T>) -> T) {
    let request = {
      let mut storage = self.inner.lock().unwrap();
      if !storage.accepting {
        return;
      }
      let value = update(storage.snapshot.desired.clone());
      storage.enqueue(Some(value))
    };
    self.notify();
    if let Some(request) = request {
      self.launch(request);
    }
  }

  /// Orders deletion after the currently active operation.
  pub fn clear(&self) {
    self.enqueue(None);
  }

  /// Retries the latest desired value, or hydration when the initial read failed.
  pub fn retry(&self) {
    let request = {
      let mut storage = self.inner.lock().unwrap();
      if !storage.accepting || storage.active.is_some() {
        return;
      }
      let intent = Intent {
        value: storage.snapshot.desired.clone(),
        version: storage.snapshot.version,
      };
      let loading = !storage.snapshot.hydrated;
      storage.prepare(intent, loading)
    };
    self.notify();
    self.launch(request);
  }

  /// Bounded diagnostic/CLI wait; application rendering must never call this.
  #[doc(hidden)]
  pub fn wait_for_idle(&self, timeout: Duration) -> bool {
    let (storage, _) = self
      .changed
      .wait_timeout_while(self.inner.lock().unwrap(), timeout, |storage| {
        storage.active.is_some()
      })
      .unwrap();
    let idle = storage.active.is_none();
    drop(storage);
    if idle {
      self.notify();
    }
    idle
  }

  fn notify(&self) {
    for notify in &*self.notifications.lock().unwrap() {
      notify.notify();
    }
    self.changed.notify_all();
  }

  fn enqueue(&self, value: Option<T>) {
    let request = self.inner.lock().unwrap().enqueue(value);
    self.notify();
    if let Some(request) = request {
      self.launch(request);
    }
  }

  fn launch(&self, prepared: Prepared) {
    let request = match prepared.request {
      Ok(request) => request,
      Err(error) => {
        self.complete(prepared.id, Err(error));
        return;
      }
    };
    let (path, slot) = {
      let storage = self.inner.lock().unwrap();
      (storage.path.clone(), storage.slot.clone())
    };
    let Some(slot) = slot else {
      self.inner.lock().unwrap().snapshot.hydrated = true;
      self.complete(request.id, Err("persistent storage is unavailable".into()));
      return;
    };
    let owner = self.clone();
    slot.submit(
      PersistenceRequest {
        path: path.expect("claimed slot has a path"),
        ..request
      },
      Arc::new(move |id, result| owner.complete(id, result)),
    );
  }

  fn complete(&self, id: PersistenceId, result: Result<Option<Vec<u8>>, String>) {
    let request = {
      let mut storage = self.inner.lock().unwrap();
      if storage.snapshot.pending != Some(id) {
        return;
      }
      let active = storage.active.take().expect("pending operation");
      assert_eq!(active.id, id);
      storage.snapshot.pending = None;
      let result = if active.loading {
        if result.is_ok() {
          storage.snapshot.hydrated = true;
        }
        result.and_then(|bytes| {
          bytes
            .map(|bytes| {
              serde_json::from_slice(&bytes)
                .map_err(|error| format!("could not read persistent state: {error}"))
            })
            .transpose()
        })
      } else {
        result.map(|_| active.intent.value)
      };
      match result {
        Ok(value) => {
          storage.snapshot.durable = value.clone();
          if active.loading {
            if storage.queued.is_none() {
              storage.snapshot.desired = value;
            }
          } else {
            storage.snapshot.committed = Some(active.intent.version);
          }
          if storage.queued.is_none() {
            storage.snapshot.error = None;
          }
        }
        Err(error) => storage.snapshot.error = Some(error),
      }
      storage.next()
    };
    self.notify();
    if let Some(request) = request {
      self.launch(request);
    }
  }
}

impl<T: Clone + Serialize> Storage<T> {
  fn enqueue(&mut self, value: Option<T>) -> Option<Prepared> {
    if !self.accepting {
      return None;
    }
    self.snapshot.version.revision = self
      .snapshot
      .version
      .revision
      .checked_add(1)
      .expect("persistence revision overflow");
    self.snapshot.desired = value.clone();
    self.queued = Some(Intent {
      value,
      version: self.snapshot.version,
    });
    self.next()
  }

  fn next(&mut self) -> Option<Prepared> {
    if self.active.is_some() || !self.snapshot.hydrated {
      return None;
    }
    self.queued.take().map(|intent| self.prepare(intent, false))
  }

  fn prepare(&mut self, intent: Intent<T>, loading: bool) -> Prepared {
    let id = Uuid::new_v4();
    let operation = if loading {
      Ok(PersistenceOperation::Load)
    } else if let Some(value) = &intent.value {
      serde_json::to_vec_pretty(value)
        .map(PersistenceOperation::Store)
        .map_err(|error| format!("could not serialize persistent state: {error}"))
    } else {
      Ok(PersistenceOperation::Remove)
    };
    let version = intent.version;
    self.active = Some(Active {
      id,
      intent,
      loading,
    });
    self.snapshot.pending = Some(id);
    Prepared {
      id,
      request: operation.map(|operation| PersistenceRequest {
        id,
        version,
        path: self.path.clone().unwrap_or_default(),
        operation,
      }),
    }
  }
}

impl<T: Clone + PartialEq + Serialize + DeserializeOwned + Send + Sync + 'static> ExternalStore
  for PersistenceStore<T>
{
  type Snapshot = PersistenceSnapshot<T>;
  fn snapshot(&self) -> Self::Snapshot {
    self.snapshot()
  }
  fn subscribe(&self, notify: StoreNotify) -> Subscription {
    let notify = Arc::new(notify);
    self.notifications.lock().unwrap().push(notify.clone());
    let notifications = Arc::downgrade(&self.notifications);
    Subscription::new(move || {
      if let Some(notifications) = notifications.upgrade() {
        notifications
          .lock()
          .unwrap()
          .retain(|current| !Arc::ptr_eq(current, &notify));
      }
    })
  }
}
