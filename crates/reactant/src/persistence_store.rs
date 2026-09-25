//! Typed persistence queue independent of component lifetimes.

use serde::{Serialize, de::DeserializeOwned};
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use uuid::Uuid;

use crate::persistence_operation::{
  PersistenceBackend, PersistenceId, PersistenceOperation, PersistenceRequest,
};

/// Applied intent, acknowledged storage, and outstanding operation state.
#[derive(Clone, Debug, PartialEq)]
pub struct PersistenceSnapshot<T> {
  /// Most recent caller intent, independent of storage success.
  pub desired: Option<T>,
  /// Most recent acknowledged value.
  pub durable: Option<T>,
  /// Initial bytes or absence were read; corrupt bytes are reported as an error.
  pub hydrated: bool,
  /// The active request, including a delayed initial read.
  pub pending: Option<PersistenceId>,
  /// Last failure, retained until the newest intent succeeds.
  pub error: Option<String>,
}

/// Serializes operations for one file, coalescing queued writes to newest intent.
/// Own one store per file and share it with consumers. Submitted work drains after drop.
#[derive(Clone)]
pub struct PersistenceStore<T> {
  inner: Rc<RefCell<Storage<T>>>,
}

struct Storage<T> {
  path: Option<PathBuf>,
  backend: Rc<dyn PersistenceBackend>,
  snapshot: PersistenceSnapshot<T>,
  active: Option<Active<T>>,
  queued: Option<Option<T>>,
  notify: Option<Rc<dyn Fn()>>,
}

struct Active<T> {
  id: PersistenceId,
  value: Option<T>,
  loading: bool,
}

impl<T: Clone + PartialEq + Serialize + DeserializeOwned + 'static> PersistenceStore<T> {
  /// Begins hydration. Missing host storage remains visibly unavailable.
  pub fn new(path: Option<PathBuf>, backend: Rc<dyn PersistenceBackend>) -> Self {
    let store = Self {
      inner: Rc::new(RefCell::new(Storage {
        path,
        backend,
        snapshot: PersistenceSnapshot {
          desired: None,
          durable: None,
          hydrated: false,
          pending: None,
          error: None,
        },
        active: None,
        queued: None,
        notify: None,
      })),
    };
    store.begin(None, true);
    store
  }

  /// Reads current intent and storage acknowledgment state.
  pub fn snapshot(&self) -> PersistenceSnapshot<T> {
    self.inner.borrow().snapshot.clone()
  }

  /// Applies intent immediately, without claiming durability before acknowledgment.
  pub fn update(&self, value: T) {
    self.enqueue(Some(value));
  }

  /// Orders deletion after any operation already in flight.
  pub fn clear(&self) {
    self.enqueue(None);
  }

  /// Retries current intent; hydration failures retry the load first.
  pub fn retry(&self) {
    let state = self.snapshot();
    if self.inner.borrow().active.is_some() {
      return;
    }
    if !state.hydrated {
      self.begin(None, true);
    } else {
      self.enqueue(state.desired);
    }
  }

  pub(crate) fn listen(&self, notify: Rc<dyn Fn()>) {
    self.inner.borrow_mut().notify = Some(notify);
    self.notify();
  }

  pub(crate) fn detach(&self) {
    self.inner.borrow_mut().notify = None;
  }

  fn notify(&self) {
    let notify = self.inner.borrow().notify.clone();
    if let Some(notify) = notify {
      notify();
    }
  }

  fn enqueue(&self, value: Option<T>) {
    {
      let mut storage = self.inner.borrow_mut();
      storage.snapshot.desired = value.clone();
      storage.queued = Some(value);
    }
    self.notify();
    self.drain();
  }

  fn drain(&self) {
    let next = {
      let mut storage = self.inner.borrow_mut();
      if storage.active.is_some() || !storage.snapshot.hydrated {
        return;
      }
      storage.queued.take()
    };
    if let Some(value) = next {
      self.begin(value, false);
    }
  }

  fn begin(&self, value: Option<T>, loading: bool) {
    let id = Uuid::new_v4();
    let (path, backend) = {
      let mut storage = self.inner.borrow_mut();
      storage.active = Some(Active {
        id,
        value: value.clone(),
        loading,
      });
      storage.snapshot.pending = Some(id);
      (storage.path.clone(), storage.backend.clone())
    };
    self.notify();
    let Some(path) = path else {
      self.inner.borrow_mut().snapshot.hydrated = true;
      self.complete(id, Err("persistent storage is unavailable".to_owned()));
      return;
    };
    let operation = if loading {
      PersistenceOperation::Load
    } else if let Some(value) = value {
      match serde_json::to_vec_pretty(&value) {
        Ok(bytes) => PersistenceOperation::Store(bytes),
        Err(error) => {
          self.complete(
            id,
            Err(format!("could not serialize persistent state: {error}")),
          );
          return;
        }
      }
    } else {
      PersistenceOperation::Remove
    };
    let completion_owner = RefCell::new(Some(self.clone()));
    backend.start(
      PersistenceRequest {
        id,
        path,
        operation,
      },
      Rc::new(move |completed, result| {
        if completed != id {
          return;
        }
        let owner = completion_owner.borrow_mut().take();
        if let Some(owner) = owner {
          owner.complete(completed, result);
        }
      }),
    );
  }

  fn complete(&self, id: PersistenceId, result: Result<Option<Vec<u8>>, String>) {
    {
      let mut storage = self.inner.borrow_mut();
      if storage.snapshot.pending != Some(id) {
        return;
      }
      let active = storage
        .active
        .take()
        .expect("pending persistence request has an operation");
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
        result.map(|_| active.value)
      };
      match result {
        Ok(value) => {
          storage.snapshot.durable = value.clone();
          if active.loading {
            storage.snapshot.hydrated = true;
            if storage.queued.is_none() {
              storage.snapshot.desired = value;
            }
          }
          if storage.queued.is_none() {
            storage.snapshot.error = None;
          }
        }
        Err(error) => {
          storage.snapshot.error = Some(error);
        }
      }
    }
    self.notify();
    self.drain();
  }
}
