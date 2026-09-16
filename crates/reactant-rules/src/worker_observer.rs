use std::{
  sync::{Arc, Condvar, Mutex, Weak},
  time::{Duration, Instant},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorkerEvent {
  Started(u64),
  Completed(u64),
  Cancelled(u64),
  Failed(u64, String),
  Stopped(u64),
}

#[derive(Clone)]
pub(crate) struct WorkerObserver {
  observed: Arc<Observed>,
}

#[derive(Default)]
pub(crate) struct Observers {
  listeners: Vec<Weak<Observed>>,
}

struct Observed {
  run: Option<u64>,
  events: Mutex<Vec<WorkerEvent>>,
  changed: Condvar,
}

impl Observers {
  pub(crate) fn subscribe(&mut self, run: Option<u64>) -> WorkerObserver {
    let events = Vec::new();
    let observed = Arc::new(Observed {
      run,
      events: Mutex::new(events),
      changed: Condvar::new(),
    });
    self.listeners.push(Arc::downgrade(&observed));
    WorkerObserver { observed }
  }

  pub(crate) fn push(&mut self, event: WorkerEvent) {
    let id = match &event {
      WorkerEvent::Started(id)
      | WorkerEvent::Completed(id)
      | WorkerEvent::Cancelled(id)
      | WorkerEvent::Failed(id, _)
      | WorkerEvent::Stopped(id) => *id,
    };
    self.listeners.retain(|listener| {
      let Some(observed) = listener.upgrade() else {
        return false;
      };
      if observed.run.is_none_or(|run| run == id) {
        observed.events.lock().unwrap().push(event.clone());
        observed.changed.notify_all();
      }
      !(observed.run == Some(id) && matches!(event, WorkerEvent::Stopped(_)))
    });
  }
}

impl WorkerObserver {
  pub(crate) fn events(&self) -> Vec<WorkerEvent> {
    self.observed.events.lock().unwrap().clone()
  }

  pub(crate) fn wait_for(
    &self,
    timeout: Duration,
    predicate: impl Fn(&[WorkerEvent]) -> bool,
  ) -> bool {
    let deadline = Instant::now() + timeout;
    let mut events = self.observed.events.lock().unwrap();
    while !predicate(&events) {
      let remaining = deadline.saturating_duration_since(Instant::now());
      if remaining.is_zero() {
        return false;
      }
      events = self
        .observed
        .changed
        .wait_timeout(events, remaining)
        .unwrap()
        .0;
    }
    true
  }
}
