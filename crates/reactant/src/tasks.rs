use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use reactant_core::hooks;
use reactant_rules::{CancellationToken, ComputationLane};

use crate::task_store::TaskStore;

/// Component-owned work whose old handles become idle after replacement or unmount.
pub struct Task<T: Send + Sync + 'static> {
  store: TaskStore<T>,
  retry: Rc<dyn Fn() -> bool>,
}

/// Immutable result of one keyed computation after worker cleanup.
#[derive(Debug)]
pub enum TaskState<T> {
  /// The key is disabled or its owner ended.
  Idle,
  /// The current computation is queued or running.
  Pending,
  /// The current computation finished successfully.
  Ready(Arc<T>),
  /// The current computation failed; retry is explicit.
  Failed(String),
}

impl<T> Clone for TaskState<T> {
  fn clone(&self) -> Self {
    match self {
      Self::Idle => Self::Idle,
      Self::Pending => Self::Pending,
      Self::Ready(value) => Self::Ready(Arc::clone(value)),
      Self::Failed(message) => Self::Failed(message.clone()),
    }
  }
}

impl<T> PartialEq for TaskState<T> {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Idle, Self::Idle) | (Self::Pending, Self::Pending) => true,
      (Self::Ready(a), Self::Ready(b)) => Arc::ptr_eq(a, b),
      (Self::Failed(a), Self::Failed(b)) => a == b,
      _ => false,
    }
  }
}

impl<T: Send + Sync + 'static> Clone for Task<T> {
  fn clone(&self) -> Self {
    Self {
      store: self.store.clone(),
      retry: self.retry.clone(),
    }
  }
}

impl<T: Send + Sync + 'static> Task<T> {
  /// Observes the current owner; retained handles never expose an ended owner's result.
  pub fn state(&self) -> TaskState<T> {
    self.store.read()
  }

  /// Retries a failed current owner once; stale, healthy and pending handles return false.
  pub fn retry(&self) -> bool {
    (self.retry)()
  }
}

/// Runs work once per enabled key and invalidates it on replacement or unmount.
/// Equal keys preserve work despite unrelated renders or new closure captures.
/// Work checks its cancellation token at bounded algorithm transitions.
pub fn use_task<K, T>(
  key: Option<K>,
  work: impl FnOnce(CancellationToken) -> T + Send + 'static,
) -> Task<T>
where
  K: hooks::Dependencies,
  T: Send + Sync + 'static,
{
  let lane = hooks::use_memo(|| Rc::new(ComputationLane::<T>::default()), ());
  let (attempt, restart) = hooks::use_state(0_u64);
  let enabled = key.is_some();
  let store = hooks::use_memo(move || TaskStore::new(enabled), (key, attempt));
  hooks::use_external_store(store.clone());
  let owned = store.clone();
  hooks::use_effect(
    move || {
      if enabled {
        owned.start(lane.replace(work));
      }
      move || {
        owned.end();
        lane.cancel();
      }
    },
    store.clone(),
  );
  let current = store.clone();
  let retry = hooks::use_memo(
    move || {
      Rc::new(move || {
        if !current.request_retry() {
          return false;
        }
        restart.update(|attempt| attempt.checked_add(1).expect("task attempt overflow"));
        true
      }) as Rc<dyn Fn() -> bool>
    },
    store.clone(),
  );
  Task { store, retry }
}

/// Diagnostic wait for worker cleanup; does not advance presentation or drive rendering.
#[doc(hidden)]
pub fn wait_for_task<T: Send + Sync + 'static>(task: &Task<T>, timeout: Duration) -> bool {
  task.store.wait(timeout)
}
