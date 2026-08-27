//! Manually controlled time for deterministic rules-engine tests.

use std::{
  cell::Cell,
  rc::Rc,
  time::{Duration, Instant},
};

/// A cloneable monotonic clock advanced explicitly by a test.
#[derive(Clone, Debug)]
pub struct ManualClock {
  now: Rc<Cell<Instant>>,
}

impl ManualClock {
  /// Creates a clock at a caller-supplied monotonic instant.
  #[must_use]
  pub fn new(now: Instant) -> Self {
    Self {
      now: Rc::new(Cell::new(now)),
    }
  }

  /// Returns the clock's current instant.
  #[must_use]
  pub fn now(&self) -> Instant {
    self.now.get()
  }

  /// Moves the clock forward by the supplied duration.
  pub fn advance(&self, duration: Duration) {
    self.now.set(
      self
        .now
        .get()
        .checked_add(duration)
        .expect("manual clock advance overflowed"),
    );
  }
}
