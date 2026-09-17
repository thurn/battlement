//! Display-local state whose notifications schedule component work.

use std::{
  collections::HashMap,
  sync::{Arc, Mutex},
};

use crate::external_store::{ExternalStore, StoreNotify, Subscription};

/// Thread-safe display-local state with coalesced notifications and stable hook reads.
pub struct DisplayStore<T> {
  state: Arc<Mutex<State<T>>>,
}

struct State<T> {
  value: T,
  listeners: HashMap<u64, StoreNotify>,
  next: u64,
}

impl<T> DisplayStore<T> {
  /// Creates a settings, selection, or other display-local store.
  pub fn new(value: T) -> Self {
    Self {
      state: Arc::new(Mutex::new(State {
        value,
        listeners: HashMap::new(),
        next: 0,
      })),
    }
  }
}

impl<T: Clone + PartialEq> DisplayStore<T> {
  /// Replaces the value and schedules consumers whose selected values change.
  pub fn set(&self, value: T) {
    self.update(|current| *current = value);
  }

  /// Updates the value atomically; notification never runs a component inline.
  pub fn update(&self, update: impl FnOnce(&mut T)) {
    let listeners = {
      let mut state = self.state.lock().expect("display store poisoned");
      let previous = state.value.clone();
      update(&mut state.value);
      if previous == state.value {
        return;
      }
      state.listeners.values().cloned().collect::<Vec<_>>()
    };
    for listener in listeners {
      listener.notify();
    }
  }
}

impl<T> Clone for DisplayStore<T> {
  fn clone(&self) -> Self {
    Self {
      state: self.state.clone(),
    }
  }
}

impl<T> PartialEq for DisplayStore<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.state, &other.state)
  }
}

impl<T: Default> Default for DisplayStore<T> {
  fn default() -> Self {
    Self::new(T::default())
  }
}

impl<T: Clone + PartialEq + Send + Sync + 'static> ExternalStore for DisplayStore<T> {
  type Snapshot = T;

  fn snapshot(&self) -> T {
    self
      .state
      .lock()
      .expect("display store poisoned")
      .value
      .clone()
  }

  fn subscribe(&self, notify: StoreNotify) -> Subscription {
    let id = {
      let mut state = self.state.lock().expect("display store poisoned");
      let id = state.next;
      state.next = id
        .checked_add(1)
        .expect("display subscription identity overflow");
      state.listeners.insert(id, notify);
      id
    };
    let state = Arc::downgrade(&self.state);
    Subscription::new(move || {
      if let Some(state) = state.upgrade() {
        state
          .lock()
          .expect("display store poisoned")
          .listeners
          .remove(&id);
      }
    })
  }
}
