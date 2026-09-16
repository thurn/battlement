use std::{
  collections::VecDeque,
  mem,
  sync::{
    Arc, Condvar, Mutex, MutexGuard,
    atomic::{AtomicBool, Ordering},
  },
  time::{Duration, Instant},
};

use crate::{Game, PresentedPrompt, ResponseHandle, response::Reply, worker};

const CAPACITY: usize = 32;

/// One immutable snapshot taken by the display consumer.
pub struct Checkpoint<G: Game> {
  pub(crate) state: G::State,
  pub(crate) animation: Option<G::StateAnimation>,
  pub(crate) prompt: Option<PresentedPrompt<G::Prompt<'static>>>,
  pub(crate) completion: Option<(G::State, G::Context)>,
}

/// Cumulative publication boundaries, independent of host time and frames.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PublicationObservation {
  /// Snapshots whose construction has begun after capacity reservation.
  pub builders_started: usize,
  /// Entries successfully published, including final output.
  pub published: usize,
  /// Entries taken for rendering.
  pub consumed: usize,
  /// The producer has reached the capacity boundary.
  pub waiting_for_capacity: bool,
  /// The run's pending output has been invalidated.
  pub abandoned: bool,
}

pub(crate) struct Publications<G: Game> {
  state: Mutex<Queue<G>>,
  changed: Condvar,
  pub(crate) abandoned: Arc<AtomicBool>,
}

struct Queue<G: Game> {
  entries: VecDeque<Checkpoint<G>>,
  reserved: usize,
  request: Option<Arc<dyn Reply>>,
  observation: PublicationObservation,
}

pub(crate) struct Reservation<'a, G: Game> {
  publications: &'a Publications<G>,
}

impl<G: Game> Checkpoint<G> {
  /// Returns the independent state snapshot for this publication.
  pub fn state(&self) -> &G::State {
    &self.state
  }

  /// Returns the semantic event, absent for final output.
  pub fn animation(&self) -> Option<&G::StateAnimation> {
    self.animation.as_ref()
  }

  /// Returns owned prompt data when this checkpoint presents a request.
  pub fn prompt(&self) -> Option<&PresentedPrompt<G::Prompt<'static>>> {
    self.prompt.as_ref()
  }

  /// Reports normal rules completion, without implying host submission.
  pub fn is_final(&self) -> bool {
    self.completion.is_some()
  }
}

impl<G: Game> Publications<G> {
  pub(crate) fn new() -> Self {
    Self {
      state: Mutex::new(Queue {
        entries: VecDeque::new(),
        reserved: 0,
        request: None,
        observation: PublicationObservation::default(),
      }),
      changed: Condvar::new(),
      abandoned: Arc::new(AtomicBool::new(false)),
    }
  }

  pub(crate) fn reserve(&self) -> Reservation<'_, G> {
    let mut state = self::lock(&self.state);
    loop {
      if state.observation.abandoned {
        drop(state);
        worker::unwind_cancelled();
      }
      if state.entries.len() + state.reserved < CAPACITY {
        break;
      }
      state.observation.waiting_for_capacity = true;
      self.changed.notify_all();
      state = self.changed.wait(state).unwrap_or_else(|e| e.into_inner());
    }
    state.observation.waiting_for_capacity = false;
    state.reserved += 1;
    state.observation.builders_started += 1;
    self.changed.notify_all();
    drop(state);
    let reservation = Reservation { publications: self };
    self.check_active();
    reservation
  }

  pub(crate) fn check_active(&self) {
    let abandoned = self::lock(&self.state).observation.abandoned;
    if abandoned {
      worker::unwind_cancelled();
    }
  }

  pub(crate) fn take(&self) -> Option<Checkpoint<G>> {
    let mut state = self::lock(&self.state);
    let entry = state.entries.pop_front();
    if entry.is_some() {
      state.observation.consumed += 1;
      self.changed.notify_all();
    }
    entry
  }

  pub(crate) fn abandon(&self) {
    let (entries, request) = {
      let mut state = self::lock(&self.state);
      self.abandoned.store(true, Ordering::Release);
      state.observation.abandoned = true;
      state.observation.waiting_for_capacity = false;
      let entries = mem::take(&mut state.entries);
      self.changed.notify_all();
      (entries, state.request.take())
    };
    if let Some(request) = request {
      request.cancel();
    }
    // Game-owned destructors run outside the communication lock.
    drop(entries);
  }

  pub(crate) fn register_request(&self, request: Arc<dyn Reply>) {
    let mut state = self::lock(&self.state);
    if state.observation.abandoned {
      drop(state);
      worker::unwind_cancelled();
    }
    let previous = state.request.replace(request);
    drop(state);
    drop(previous);
  }

  pub(crate) fn response_handle(&self) -> Option<ResponseHandle<G::Prompt<'static>>> {
    self::lock(&self.state)
      .request
      .as_ref()
      .map(ResponseHandle::new)
  }

  pub(crate) fn observation(&self) -> PublicationObservation {
    self::lock(&self.state).observation
  }

  pub(crate) fn wait_for(
    &self,
    timeout: Duration,
    predicate: impl Fn(PublicationObservation) -> bool,
  ) -> bool {
    let deadline = Instant::now() + timeout;
    let mut state = self::lock(&self.state);
    while !predicate(state.observation) {
      let remaining = deadline.saturating_duration_since(Instant::now());
      if remaining.is_zero() {
        return false;
      }
      state = self
        .changed
        .wait_timeout(state, remaining)
        .unwrap_or_else(|e| e.into_inner())
        .0;
    }
    true
  }
}

impl<G: Game> Reservation<'_, G> {
  pub(crate) fn publish(self, checkpoint: Checkpoint<G>) {
    let mut state = self::lock(&self.publications.state);
    if state.observation.abandoned {
      drop(state);
      drop(checkpoint);
      worker::unwind_cancelled();
    }
    state.entries.push_back(checkpoint);
    state.observation.published += 1;
    self.publications.changed.notify_all();
    // Release the reservation under the same lock as insertion, so an entry
    // never temporarily counts twice against capacity.
    state.reserved -= 1;
    mem::forget(self);
  }
}

impl<G: Game> Drop for Reservation<'_, G> {
  fn drop(&mut self) {
    let mut state = self::lock(&self.publications.state);
    state.reserved -= 1;
    self.publications.changed.notify_all();
  }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
  mutex.lock().unwrap_or_else(|e| e.into_inner())
}
