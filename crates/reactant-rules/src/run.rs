use std::{sync::Arc, thread, time::Duration};

use crate::{
  Checkpoint, DisplayConnection, Game, PublicationObservation, ResponseHandle,
  publication::Publications,
  worker::{WorkerEvent, WorkerObserver, WorkerSlot},
};

/// The display consumer of one synchronous rules action on a real worker.
///
/// Taking output releases publication capacity, independently of host playback.
/// The owner must stop or drop the run when abandoning its display lifetime.
/// Application acceptance remains the responsibility of the consuming session.
pub struct RulesRun<G: Game> {
  slot: WorkerSlot,
  observer: WorkerObserver,
  publications: Arc<Publications<G>>,
}

/// Lifecycle observations occur outside the bounded publication FIFO.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunObservation {
  /// The worker has entered its rules boundary.
  pub started: bool,
  /// All worker-owned interrupted state and context have been destroyed.
  pub stopped: bool,
  /// Rules returned and published their final checkpoint.
  pub completed: bool,
  /// Expected cancellation unwound the worker.
  pub cancelled: bool,
  /// An unexpected rules panic, if any.
  pub failure: Option<String>,
}

struct FailureGuard<'a, G: Game>(&'a Publications<G>);

impl<G: Game> Drop for FailureGuard<'_, G> {
  fn drop(&mut self) {
    if thread::panicking() {
      self.0.abandon();
    }
  }
}

impl<G: Game> RulesRun<G> {
  /// Clones private state, creates the context on the caller, and schedules rules.
  ///
  /// No host clock or rendered frame is advanced. This consumer does not install
  /// accepted state or attach a game to an application.
  pub fn start(
    state: &G::State,
    action: G::Action,
    make_context: impl FnOnce(DisplayConnection<G>) -> G::Context,
  ) -> Self {
    assert!(G::is_legal_action(state, &action), "illegal rules action");
    let mut state = G::logical_clone(state);
    let publications = Arc::new(Publications::new());
    let mut context = make_context(DisplayConnection {
      publications: Arc::clone(&publications),
    });
    let slot = WorkerSlot::new();
    let observer = slot.observer();
    let output = Arc::clone(&publications);
    slot.replace(move |_| {
      let _failure_guard = FailureGuard(output.as_ref());
      G::execute(&mut context, &mut state, action);
      output.check_active();
      let reservation = output.reserve();
      let snapshot = G::logical_clone(&state);
      output.check_active();
      reservation.publish(Checkpoint {
        state: snapshot,
        animation: None,
        prompt: None,
        completion: Some((state, context)),
      });
    });
    Self {
      slot,
      observer,
      publications,
    }
  }

  /// Takes the oldest checkpoint and immediately releases its pending slot.
  pub fn take_checkpoint(&self) -> Option<Checkpoint<G>> {
    self.publications.take()
  }

  /// Returns the current request connection without consuming its checkpoint.
  pub fn response_handle(&self) -> Option<ResponseHandle<G::Prompt<'static>>> {
    self.publications.response_handle()
  }

  /// Invalidates pending output and wakes publication waits without joining.
  pub fn stop(&self) {
    self.publications.abandon();
    self.slot.close();
  }

  /// Returns cumulative public publication boundaries.
  pub fn publication_observation(&self) -> PublicationObservation {
    self.publications.observation()
  }

  /// Waits for a publication boundary without consuming output or advancing time.
  pub fn wait_for_publication(
    &self,
    timeout: Duration,
    predicate: impl Fn(PublicationObservation) -> bool,
  ) -> bool {
    self.publications.wait_for(timeout, predicate)
  }

  /// Returns worker lifecycle status without taking any checkpoint.
  pub fn observation(&self) -> RunObservation {
    let mut result = RunObservation::default();
    for event in self.observer.events() {
      match event {
        WorkerEvent::Started(_) => result.started = true,
        WorkerEvent::Stopped(_) => result.stopped = true,
        WorkerEvent::Completed(_) => result.completed = true,
        WorkerEvent::Cancelled(_) => result.cancelled = true,
        WorkerEvent::Failed(_, message) => result.failure = Some(message),
      }
    }
    result
  }

  /// Waits for worker entry without advancing presentation time or frames.
  pub fn wait_for_worker_started(&self, timeout: Duration) -> bool {
    self.observer.wait_for(timeout, |events| {
      events.iter().any(|e| matches!(e, WorkerEvent::Started(_)))
    })
  }

  /// Waits for cleanup without advancing presentation time or frames.
  pub fn wait_for_worker_stopped(&self, timeout: Duration) -> bool {
    self.observer.wait_for(timeout, |events| {
      events.iter().any(|e| matches!(e, WorkerEvent::Stopped(_)))
    })
  }
}

impl<G: Game> Drop for RulesRun<G> {
  fn drop(&mut self) {
    self.stop();
  }
}
