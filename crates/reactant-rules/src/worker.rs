use std::{
  any::Any,
  panic::{self, AssertUnwindSafe},
  sync::{Arc, Condvar, Mutex, MutexGuard},
  thread,
};

use crate::worker_observer::{Observers, WorkerEvent, WorkerObserver};

type WorkerTask = Box<dyn FnOnce(WorkerConnection) + Send + 'static>;

struct Cancellation;

pub(crate) struct WorkerSlot {
  shared: Arc<Shared>,
}

pub(crate) struct WorkerConnection {
  shared: Arc<ConnectionShared>,
}

struct Shared {
  state: Mutex<SlotState>,
  changed: Condvar,
}

struct SlotState {
  next_id: u64,
  active: Option<ActiveWorker>,
  pending: Option<PendingWorker>,
  closed: bool,
  events: Observers,
}

struct ActiveWorker {
  id: u64,
  connection: Arc<ConnectionShared>,
}

struct PendingWorker {
  id: u64,
  task: WorkerTask,
}

struct ConnectionShared {
  cancelled: Mutex<bool>,
  changed: Condvar,
}

enum WorkerOutcome {
  Completed,
  Cancelled,
  Failed(String),
}

impl WorkerSlot {
  pub(crate) fn new() -> Self {
    Self {
      shared: Arc::new(Shared {
        state: Mutex::new(SlotState {
          next_id: 1,
          active: None,
          pending: None,
          closed: false,
          events: Observers::default(),
        }),
        changed: Condvar::new(),
      }),
    }
  }

  #[cfg(any(test, feature = "platform-proof"))]
  pub(crate) fn observer(&self) -> WorkerObserver {
    self::lock(&self.shared.state).events.subscribe(None)
  }

  pub(crate) fn observe_next(&self) -> WorkerObserver {
    let mut state = self::lock(&self.shared.state);
    let id = state.next_id;
    state.events.subscribe(Some(id))
  }

  pub(crate) fn is_idle(&self) -> bool {
    self::lock(&self.shared.state).active.is_none()
  }

  pub(crate) fn cancel_run(&self, id: u64) {
    let pending = {
      let mut state = self::lock(&self.shared.state);
      if let Some(active) = &state.active
        && active.id == id
      {
        self::cancel(&active.connection);
      }
      if state
        .pending
        .as_ref()
        .is_some_and(|pending| pending.id == id)
      {
        state.pending.take()
      } else {
        None
      }
    };
    if let Some(pending) = pending {
      drop(pending);
      let mut state = self::lock(&self.shared.state);
      state.events.push(WorkerEvent::Cancelled(id));
      state.events.push(WorkerEvent::Stopped(id));
      self.shared.changed.notify_all();
    }
  }

  pub(crate) fn replace(&self, task: impl FnOnce(WorkerConnection) + Send + 'static) -> u64 {
    let mut state = self::lock(&self.shared.state);
    assert!(!state.closed, "cannot replace a closed rules worker slot");
    let id = state.next_id;
    state.next_id += 1;
    let pending = PendingWorker {
      id,
      task: Box::new(task),
    };
    let (start, superseded) = if let Some(active) = &state.active {
      self::cancel(&active.connection);
      (None, state.pending.replace(pending))
    } else {
      state.active = Some(ActiveWorker {
        id,
        connection: Arc::new(ConnectionShared {
          cancelled: Mutex::new(false),
          changed: Condvar::new(),
        }),
      });
      (Some(pending), None)
    };
    drop(state);
    drop(superseded);
    if let Some(worker) = start {
      self::spawn(Arc::clone(&self.shared), worker);
    }
    id
  }

  pub(crate) fn close(&self) {
    let mut state = self::lock(&self.shared.state);
    state.closed = true;
    if let Some(active) = &state.active {
      self::cancel(&active.connection);
    }
    let pending = state.pending.take();
    self.shared.changed.notify_all();
    drop(state);
    drop(pending);
  }
}

impl Drop for WorkerSlot {
  fn drop(&mut self) {
    self.close();
  }
}

impl WorkerConnection {
  #[cfg(any(test, feature = "platform-proof"))]
  pub(crate) fn wait_until_cancelled(&self) -> ! {
    let mut cancelled = self::lock(&self.shared.cancelled);
    while !*cancelled {
      cancelled = self::wait(&self.shared.changed, cancelled);
    }
    drop(cancelled);
    self::unwind_cancelled()
  }

  pub(crate) fn is_cancelled(&self) -> bool {
    *self::lock(&self.shared.cancelled)
  }

  pub(crate) fn check_cancelled(&self) {
    let cancelled = self.is_cancelled();
    if cancelled {
      self::unwind_cancelled();
    }
  }
}

fn spawn(shared: Arc<Shared>, pending: PendingWorker) {
  thread::Builder::new()
    .name("reactant-rules".to_owned())
    .spawn(move || self::run(shared, pending))
    .expect("rules worker thread must start");
}

fn run(shared: Arc<Shared>, pending: PendingWorker) {
  let id = pending.id;
  let connection = {
    let mut state = self::lock(&shared.state);
    let active = state
      .active
      .as_ref()
      .expect("started worker must be active");
    assert_eq!(
      active.id, id,
      "started worker must match the active request"
    );
    let connection = WorkerConnection {
      shared: Arc::clone(&active.connection),
    };
    state.events.push(WorkerEvent::Started(id));
    shared.changed.notify_all();
    connection
  };

  // The closure owns all game state and context. If it unwinds, those private
  // values are discarded before the shared coordinator is touched again.
  let outcome = match panic::catch_unwind(AssertUnwindSafe(move || {
    connection.check_cancelled();
    (pending.task)(connection);
  })) {
    Ok(()) => WorkerOutcome::Completed,
    Err(payload) if payload.is::<Cancellation>() => WorkerOutcome::Cancelled,
    Err(payload) => WorkerOutcome::Failed(self::panic_message(payload.as_ref())),
  };
  self::finish(shared, id, outcome);
}

fn finish(shared: Arc<Shared>, id: u64, outcome: WorkerOutcome) {
  let next = {
    let mut state = self::lock(&shared.state);
    let active = state.active.take().expect("finished worker must be active");
    assert_eq!(
      active.id, id,
      "finished worker must match the active request"
    );
    drop(active);
    state.events.push(match outcome {
      WorkerOutcome::Completed => WorkerEvent::Completed(id),
      WorkerOutcome::Cancelled => WorkerEvent::Cancelled(id),
      WorkerOutcome::Failed(message) => WorkerEvent::Failed(id, message),
    });
    state.events.push(WorkerEvent::Stopped(id));
    let next = (!state.closed).then(|| state.pending.take()).flatten();
    if let Some(next) = &next {
      state.active = Some(ActiveWorker {
        id: next.id,
        connection: Arc::new(ConnectionShared {
          cancelled: Mutex::new(false),
          changed: Condvar::new(),
        }),
      });
    }
    shared.changed.notify_all();
    next
  };
  if let Some(next) = next {
    self::spawn(shared, next);
  }
}

fn cancel(connection: &ConnectionShared) {
  *self::lock(&connection.cancelled) = true;
  connection.changed.notify_all();
}

pub(crate) fn unwind_cancelled() -> ! {
  panic::resume_unwind(Box::new(Cancellation));
}

pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> String {
  payload
    .downcast_ref::<&str>()
    .map(|message| (*message).to_owned())
    .or_else(|| payload.downcast_ref::<String>().cloned())
    .unwrap_or_else(|| "non-string panic payload".to_owned())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
  mutex
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(any(test, feature = "platform-proof"))]
fn wait<'a, T>(condition: &Condvar, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
  condition
    .wait(guard)
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
  use std::sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
  };
  use std::time::Duration;

  use crate::{worker::WorkerSlot, worker_observer::WorkerEvent};

  struct DropProbe(Arc<AtomicUsize>);

  impl Drop for DropProbe {
    fn drop(&mut self) {
      self.0.fetch_add(1, Ordering::SeqCst);
    }
  }

  #[test]
  fn cancellation_before_wait_is_observed_and_cleanup_precedes_stopped() {
    let slot = WorkerSlot::new();
    let observer = slot.observer();
    let entered = Arc::new(Barrier::new(2));
    let proceed = Arc::new(Barrier::new(2));
    let drops = Arc::new(AtomicUsize::new(0));
    let entered_worker = Arc::clone(&entered);
    let proceed_worker = Arc::clone(&proceed);
    let drops_worker = Arc::clone(&drops);
    let first = slot.replace(move |connection| {
      let _probe = DropProbe(drops_worker);
      entered_worker.wait();
      proceed_worker.wait();
      connection.wait_until_cancelled();
    });
    entered.wait();
    let second = slot.replace(|_| {});
    proceed.wait();
    assert!(observer.wait_for(Duration::from_secs(5), |events| {
      events.contains(&WorkerEvent::Stopped(second))
    }));

    let events = observer.events();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert!(events.contains(&WorkerEvent::Cancelled(first)));
    assert!(events.contains(&WorkerEvent::Stopped(first)));
    assert!(events.contains(&WorkerEvent::Completed(second)));
  }

  #[test]
  fn a_real_panic_does_not_prevent_the_latest_replacement() {
    let slot = WorkerSlot::new();
    let observer = slot.observer();
    let failed = slot.replace(|_| panic!("rules failed"));
    assert!(observer.wait_for(Duration::from_secs(5), |events| {
      events.contains(&WorkerEvent::Stopped(failed))
    }));
    let replacement = slot.replace(|_| {});
    assert!(observer.wait_for(Duration::from_secs(5), |events| {
      events.contains(&WorkerEvent::Stopped(replacement))
    }));

    let events = observer.events();
    assert!(events.contains(&WorkerEvent::Failed(failed, "rules failed".to_owned())));
    assert!(events.contains(&WorkerEvent::Completed(replacement)));
  }

  #[test]
  fn only_the_latest_replacement_starts_after_abandoned_computation() {
    let slot = WorkerSlot::new();
    let observer = slot.observer();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let entered_worker = Arc::clone(&entered);
    let release_worker = Arc::clone(&release);
    let active = slot.replace(move |connection| {
      entered_worker.wait();
      release_worker.wait();
      connection.wait_until_cancelled();
    });
    entered.wait();
    let superseded = slot.replace(|_| {});
    let superseded_again = slot.replace(|_| {});
    let latest = slot.replace(|_| {});
    release.wait();
    assert!(observer.wait_for(Duration::from_secs(5), |events| {
      events.contains(&WorkerEvent::Stopped(latest))
    }));

    let events = observer.events();
    assert!(events.contains(&WorkerEvent::Cancelled(active)));
    assert!(!events.contains(&WorkerEvent::Started(superseded)));
    assert!(!events.contains(&WorkerEvent::Started(superseded_again)));
    assert!(events.contains(&WorkerEvent::Completed(latest)));
  }
}
