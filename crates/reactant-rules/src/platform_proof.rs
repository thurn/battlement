//! Test-only controls for proving the worker boundary on release platforms.

use std::{
  sync::{Arc, Condvar, Mutex, MutexGuard},
  thread::{self, ThreadId},
  time::{Duration, Instant},
};

use crate::{
  Game,
  worker::{WorkerConnection, WorkerSlot},
  worker_observer::{WorkerEvent, WorkerObserver},
};

/// A deterministic task accepted by the native platform proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProofTask {
  /// Blocks at a cancellation-aware nested rules boundary.
  Wait,
  /// Holds ordinary computation until the test releases it.
  Computation,
  /// Raises a genuine rules panic.
  Panic,
  /// Returns normally.
  Complete,
}

/// Observable worker and cleanup values for the platform proof.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProofSnapshot {
  pub started: usize,
  pub stopped: usize,
  pub cancelled: usize,
  pub failed: usize,
  pub completed: usize,
  pub drops: usize,
  pub waiting: bool,
  pub computing: bool,
  pub last_started: u64,
  pub off_creator_thread: bool,
}

/// Owns one non-joining worker slot used by native release validation.
pub struct WorkerProof {
  slot: WorkerSlot,
  worker: WorkerObserver,
  state: Arc<ProofState>,
}

/// Observes a proof after its owning engine has been destroyed.
#[derive(Clone)]
pub struct ProofObserver {
  worker: WorkerObserver,
  state: Arc<ProofState>,
}

struct ProofState {
  values: Mutex<ProofValues>,
  changed: Condvar,
  creator_thread: ThreadId,
}

#[derive(Default)]
struct ProofValues {
  drops: usize,
  waiting: bool,
  computing: bool,
  release_computation: bool,
  off_creator_thread: bool,
}

struct DropProbe {
  state: Arc<ProofState>,
}

struct WaitingProbe {
  state: Arc<ProofState>,
}

struct ProofGame;

struct ProofGameState {
  generation: u64,
}

struct ProofContext {
  connection: WorkerConnection,
  state: Arc<ProofState>,
}

impl WorkerProof {
  /// Creates an idle proof worker.
  #[must_use]
  pub fn new() -> Self {
    let slot = WorkerSlot::new();
    let worker = slot.observer();
    Self {
      slot,
      worker,
      state: Arc::new(ProofState {
        values: Mutex::new(ProofValues::default()),
        changed: Condvar::new(),
        creator_thread: thread::current().id(),
      }),
    }
  }

  /// Returns an observer that remains valid after this owner is dropped.
  #[must_use]
  pub fn observer(&self) -> ProofObserver {
    ProofObserver {
      worker: self.worker.clone(),
      state: Arc::clone(&self.state),
    }
  }

  /// Replaces the active or pending proof task and returns its identity.
  pub fn replace(&self, task: ProofTask) -> u64 {
    if task == ProofTask::Computation {
      let mut values = self::lock(&self.state.values);
      if !values.computing {
        values.release_computation = false;
      }
    }
    let state = Arc::clone(&self.state);
    let accepted = ProofGameState { generation: 0 };
    let mut worker_state = ProofGame::logical_clone(&accepted);
    self.slot.replace(move |connection| {
      let mut context = ProofContext { connection, state };
      ProofGame::execute(&mut context, &mut worker_state, task);
    })
  }
}

impl Default for WorkerProof {
  fn default() -> Self {
    Self::new()
  }
}

impl ProofObserver {
  /// Returns the current proof observation without blocking.
  #[must_use]
  pub fn snapshot(&self) -> ProofSnapshot {
    let events = self.worker.events();
    let values = self::lock(&self.state.values);
    ProofSnapshot {
      started: self::count(&events, |event| matches!(event, WorkerEvent::Started(_))),
      stopped: self::count(&events, |event| matches!(event, WorkerEvent::Stopped(_))),
      cancelled: self::count(&events, |event| matches!(event, WorkerEvent::Cancelled(_))),
      failed: self::count(&events, |event| matches!(event, WorkerEvent::Failed(_, _))),
      completed: self::count(&events, |event| matches!(event, WorkerEvent::Completed(_))),
      drops: values.drops,
      waiting: values.waiting,
      computing: values.computing,
      last_started: events
        .iter()
        .rev()
        .find_map(|event| match event {
          WorkerEvent::Started(id) => Some(*id),
          _ => None,
        })
        .unwrap_or(0),
      off_creator_thread: values.off_creator_thread,
    }
  }

  /// Releases the deterministic ordinary-computation barrier.
  pub fn release_computation(&self) {
    let mut values = self::lock(&self.state.values);
    values.release_computation = true;
    self.state.changed.notify_all();
  }

  /// Waits until at least `minimum` proof workers have started.
  #[must_use]
  pub fn wait_for_worker_started(
    &self,
    minimum: usize,
    timeout: Duration,
  ) -> Option<ProofSnapshot> {
    self
      .worker
      .wait_for(timeout, |events| {
        self::count(events, |event| matches!(event, WorkerEvent::Started(_))) >= minimum
      })
      .then(|| self.snapshot())
  }

  /// Waits until at least `minimum` proof workers have stopped.
  #[must_use]
  pub fn wait_for_worker_stopped(
    &self,
    minimum: usize,
    timeout: Duration,
  ) -> Option<ProofSnapshot> {
    self
      .worker
      .wait_for(timeout, |events| {
        self::count(events, |event| matches!(event, WorkerEvent::Stopped(_))) >= minimum
      })
      .then(|| self.snapshot())
  }

  /// Waits until the cancellation-aware rules barrier is entered.
  #[must_use]
  pub fn wait_for_waiting(&self, timeout: Duration) -> Option<ProofSnapshot> {
    self.wait_for_barrier(timeout, |values| values.waiting)
  }

  /// Waits until the ordinary-computation barrier is entered.
  #[must_use]
  pub fn wait_for_computing(&self, timeout: Duration) -> Option<ProofSnapshot> {
    self.wait_for_barrier(timeout, |values| values.computing)
  }

  fn wait_for_barrier(
    &self,
    timeout: Duration,
    predicate: impl Fn(&ProofValues) -> bool,
  ) -> Option<ProofSnapshot> {
    let deadline = Instant::now() + timeout;
    let mut values = self::lock(&self.state.values);
    while !predicate(&values) {
      let remaining = deadline.saturating_duration_since(Instant::now());
      if remaining.is_zero() {
        return None;
      }
      let (next, result) = self::wait_timeout(&self.state.changed, values, remaining);
      values = next;
      if result.timed_out() && !predicate(&values) {
        return None;
      }
    }
    drop(values);
    Some(self.snapshot())
  }
}

impl Drop for DropProbe {
  fn drop(&mut self) {
    let mut values = self::lock(&self.state.values);
    values.drops += 1;
    self.state.changed.notify_all();
  }
}

impl Drop for WaitingProbe {
  fn drop(&mut self) {
    let mut values = self::lock(&self.state.values);
    values.waiting = false;
    self.state.changed.notify_all();
  }
}

impl Game for ProofGame {
  type State = ProofGameState;
  type Action = ProofTask;
  type StateAnimation = ();
  type Prompt<'a> = ();
  type Context = ProofContext;

  fn logical_clone(state: &Self::State) -> Self::State {
    ProofGameState {
      generation: state.generation,
    }
  }

  fn is_legal_action(_state: &Self::State, _action: &Self::Action) -> bool {
    true
  }

  fn execute(context: &mut Self::Context, state: &mut Self::State, action: Self::Action) {
    {
      let mut values = self::lock(&context.state.values);
      values.off_creator_thread = thread::current().id() != context.state.creator_thread;
    }
    state.generation += 1;
    match action {
      ProofTask::Wait => self::outer_wait(&context.connection, Arc::clone(&context.state)),
      ProofTask::Computation => {
        let mut values = self::lock(&context.state.values);
        values.computing = true;
        context.state.changed.notify_all();
        while !values.release_computation {
          values = self::wait(&context.state.changed, values);
        }
        values.computing = false;
        drop(values);
        context.connection.wait_until_cancelled();
      }
      ProofTask::Panic => panic!("fixture genuine rules panic"),
      ProofTask::Complete => {}
    }
  }
}

fn outer_wait(connection: &WorkerConnection, state: Arc<ProofState>) -> ! {
  let _outer = DropProbe {
    state: Arc::clone(&state),
  };
  self::inner_wait(connection, state)
}

fn inner_wait(connection: &WorkerConnection, state: Arc<ProofState>) -> ! {
  let _inner = DropProbe {
    state: Arc::clone(&state),
  };
  let mut values = self::lock(&state.values);
  values.waiting = true;
  state.changed.notify_all();
  drop(values);
  let _waiting = WaitingProbe {
    state: Arc::clone(&state),
  };
  connection.wait_until_cancelled()
}

fn count(events: &[WorkerEvent], predicate: impl Fn(&WorkerEvent) -> bool) -> usize {
  events.iter().filter(|event| predicate(event)).count()
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
  mutex
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait<'a, T>(condition: &Condvar, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
  condition
    .wait(guard)
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn wait_timeout<'a, T>(
  condition: &Condvar,
  guard: MutexGuard<'a, T>,
  timeout: Duration,
) -> (MutexGuard<'a, T>, std::sync::WaitTimeoutResult) {
  condition
    .wait_timeout(guard, timeout)
    .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use crate::platform_proof::{ProofTask, WorkerProof};

  #[test]
  fn repeated_replacement_runs_only_the_latest_proof_task() {
    let proof = WorkerProof::new();
    let observer = proof.observer();
    proof.replace(ProofTask::Wait);
    observer
      .wait_for_waiting(Duration::from_secs(5))
      .expect("waiting proof barrier");
    proof.replace(ProofTask::Computation);
    observer
      .wait_for_worker_stopped(1, Duration::from_secs(5))
      .expect("first proof worker stopped");
    observer
      .wait_for_computing(Duration::from_secs(5))
      .expect("computation proof barrier");
    proof.replace(ProofTask::Complete);
    proof.replace(ProofTask::Complete);
    proof.replace(ProofTask::Complete);
    observer.release_computation();
    let snapshot = observer
      .wait_for_worker_stopped(3, Duration::from_secs(5))
      .expect("all proof workers stopped");

    assert_eq!(snapshot.started, 3);
    assert_eq!(snapshot.cancelled, 2);
    assert_eq!(snapshot.completed, 1);
    assert_eq!(snapshot.last_started, 5);
  }
}
