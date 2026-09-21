//! Explicit clock control for tests of temporal behavior.

use std::time::Duration;

use battlement_native::Engine;

use crate::Display;

/// Drives virtual time when elapsed time is the behavior under test.
pub struct Clock;

impl Clock {
  /// Advances rules and presentation clocks without recording a rendered frame.
  pub fn advance<E: Engine>(display: &mut Display<E>, duration: Duration) {
    display.client.advance_time(duration);
    display.flush();
  }
}
