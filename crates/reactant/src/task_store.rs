use std::{
  future::Future,
  pin::Pin,
  sync::{Arc, Mutex, Weak},
  task::{Context, Poll, Wake, Waker},
  time::Duration,
};

use reactant_core::external_store::{ExternalStore, StoreNotify, Subscription};
use reactant_rules::{Computation, ComputationError};

use crate::tasks::TaskState;

pub(crate) struct TaskStore<T> {
  data: Arc<Mutex<Data<T>>>,
  wake: Arc<Notifications>,
}

struct Data<T> {
  future: Option<Computation<T>>,
  state: TaskState<T>,
  active: bool,
  retry_requested: bool,
}

#[derive(Default)]
struct Notifications(Mutex<Vec<Arc<StoreNotify>>>);

impl<T> Clone for TaskStore<T> {
  fn clone(&self) -> Self {
    Self {
      data: Arc::clone(&self.data),
      wake: Arc::clone(&self.wake),
    }
  }
}

impl<T> PartialEq for TaskStore<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.data, &other.data)
  }
}

impl<T> TaskStore<T> {
  pub(crate) fn new(enabled: bool) -> Self {
    Self {
      data: Arc::new(Mutex::new(Data {
        future: None,
        state: if enabled {
          TaskState::Pending
        } else {
          TaskState::Idle
        },
        active: false,
        retry_requested: false,
      })),
      wake: Arc::new(Notifications::default()),
    }
  }

  pub(crate) fn start(&self, future: Computation<T>) {
    let mut data = self.data.lock().unwrap();
    data.active = true;
    data.future = Some(future);
    drop(data);
    self.wake.wake_by_ref();
  }

  pub(crate) fn end(&self) {
    let mut data = self.data.lock().unwrap();
    data.active = false;
    data.state = TaskState::Idle;
    data.future = None;
    drop(data);
    self.wake.wake_by_ref();
  }

  pub(crate) fn request_retry(&self) -> bool {
    let mut data = self.data.lock().unwrap();
    if !data.active || data.retry_requested {
      return false;
    }
    if !matches!(data.state, TaskState::Failed(_)) {
      return false;
    }
    data.retry_requested = true;
    true
  }

  pub(crate) fn wait(&self, timeout: Duration) -> bool {
    let data = self.data.lock().unwrap();
    data
      .future
      .as_ref()
      .map_or(!matches!(data.state, TaskState::Pending), |future| {
        future.wait_for_stopped(timeout)
      })
  }

  pub(crate) fn read(&self) -> TaskState<T> {
    let mut data = self.data.lock().unwrap();
    if let Some(future) = &mut data.future {
      let waker = Waker::from(Arc::clone(&self.wake));
      if let Poll::Ready(result) = Pin::new(future).poll(&mut Context::from_waker(&waker)) {
        data.future = None;
        data.state = match result {
          Ok(value) => TaskState::Ready(Arc::new(value)),
          Err(ComputationError::Cancelled) => TaskState::Idle,
          Err(ComputationError::Failed(message)) => TaskState::Failed(message),
        };
      }
    }
    data.state.clone()
  }
}

impl<T: Send + Sync + 'static> ExternalStore for TaskStore<T> {
  type Snapshot = TaskState<T>;
  fn snapshot(&self) -> Self::Snapshot {
    self.read()
  }
  fn subscribe(&self, notify: StoreNotify) -> Subscription {
    let notify = Arc::new(notify);
    self.wake.0.lock().unwrap().push(Arc::clone(&notify));
    let wake: Weak<Notifications> = Arc::downgrade(&self.wake);
    Subscription::new(move || {
      if let Some(wake) = wake.upgrade() {
        wake
          .0
          .lock()
          .unwrap()
          .retain(|current| !Arc::ptr_eq(current, &notify));
      }
    })
  }
}

impl Wake for Notifications {
  fn wake(self: Arc<Self>) {
    self.wake_by_ref();
  }
  fn wake_by_ref(self: &Arc<Self>) {
    for notify in &*self.0.lock().unwrap() {
      notify.notify();
    }
  }
}
