//! Monotonic host time for exported application timers.

use std::{
  cell::Cell,
  time::{Duration, Instant},
};

pub(crate) struct HostClock {
  base: Instant,
  origin: Cell<Option<Duration>>,
  elapsed: Cell<Duration>,
}

impl HostClock {
  pub(crate) fn new() -> Self {
    Self {
      base: Instant::now(),
      origin: Cell::new(None),
      elapsed: Cell::new(Duration::ZERO),
    }
  }

  pub(crate) fn now(&self) -> Instant {
    self
      .base
      .checked_add(self.elapsed.get())
      .expect("application clock overflow")
  }

  pub(crate) fn observe(&self, native: Duration) {
    let origin = self.origin.get().unwrap_or_else(|| {
      self.origin.set(Some(native));
      native
    });
    let elapsed = native
      .checked_sub(origin)
      .expect("native application clock moved backward");
    assert!(
      elapsed >= self.elapsed.get(),
      "native application clock moved backward"
    );
    self.elapsed.set(elapsed);
  }
}
