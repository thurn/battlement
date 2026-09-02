//! A bounded, wake-driven executor serviced by the application host.

use std::{
  collections::VecDeque,
  sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicBool, Ordering},
  },
  task::{Context, Poll, Wake, Waker},
};

use crate::executor::{BoxFuture, SpawnedTask, Spawner};

const MAX_POLLS: usize = 256;

/// A cooperative executor; each turn polls a bounded set of ready tasks.
#[derive(Clone, Default)]
pub struct CooperativeExecutor {
  ready: Arc<Mutex<VecDeque<Arc<Task>>>>,
  servicing: Arc<AtomicBool>,
}

impl CooperativeExecutor {
  pub(crate) fn has_ready(&self) -> bool {
    !self.ready.lock().expect("executor queue lock").is_empty()
  }

  /// Polls ready tasks without waiting for pending work or repeatedly polling self-wakers.
  pub fn tick(&self) {
    if self.servicing.swap(true, Ordering::AcqRel) {
      return;
    }
    let _servicing = Servicing(&self.servicing);
    let count = self
      .ready
      .lock()
      .expect("executor queue lock")
      .len()
      .min(MAX_POLLS);
    for _ in 0..count {
      let Some(task) = self.ready.lock().expect("executor queue lock").pop_front() else {
        break;
      };
      task.queued.store(false, Ordering::Release);
      let Some(mut future) = task.future.lock().expect("executor task lock").take() else {
        continue;
      };
      let waker = Waker::from(Arc::clone(&task));
      if future.as_mut().poll(&mut Context::from_waker(&waker)) == Poll::Pending {
        let mut slot = task.future.lock().expect("executor task lock");
        if !task.canceled.load(Ordering::Acquire) {
          *slot = Some(future);
        }
      }
    }
  }
}

struct Servicing<'a>(&'a AtomicBool);

impl Drop for Servicing<'_> {
  fn drop(&mut self) {
    self.0.store(false, Ordering::Release);
  }
}

impl Spawner for CooperativeExecutor {
  fn spawn(&self, future: BoxFuture<'static, ()>) -> SpawnedTask {
    let task = Arc::new(Task {
      future: Mutex::new(Some(future)),
      ready: Arc::downgrade(&self.ready),
      queued: AtomicBool::new(false),
      canceled: AtomicBool::new(false),
    });
    task.wake_by_ref();
    SpawnedTask::new(move || {
      task.canceled.store(true, Ordering::Release);
      let future = task.future.lock().expect("executor task lock").take();
      drop(future);
    })
  }
}

struct Task {
  future: Mutex<Option<BoxFuture<'static, ()>>>,
  ready: Weak<Mutex<VecDeque<Arc<Task>>>>,
  queued: AtomicBool,
  canceled: AtomicBool,
}

impl Wake for Task {
  fn wake(self: Arc<Self>) {
    self.wake_by_ref();
  }

  fn wake_by_ref(self: &Arc<Self>) {
    if self.canceled.load(Ordering::Acquire) || self.queued.swap(true, Ordering::AcqRel) {
      return;
    }
    if let Some(ready) = self.ready.upgrade() {
      ready
        .lock()
        .expect("executor queue lock")
        .push_back(Arc::clone(self));
    }
  }
}
