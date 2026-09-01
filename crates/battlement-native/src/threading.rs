//! Adaptive threading for rules-engine background work.

use std::{
  panic::{AssertUnwindSafe, catch_unwind},
  sync::{Arc, Mutex},
};

use rayon::{ThreadPool, ThreadPoolBuildError, ThreadPoolBuilder};

use crate::{EngineError, panic_capture};

/// A Rayon executor that adapts its worker count to the runtime platform.
pub struct AdaptiveThreadPool {
  pool: ThreadPool,
  panic: Arc<Mutex<Option<String>>>,
}

impl AdaptiveThreadPool {
  /// Creates an executor using the platform's preferred scheduling policy.
  pub fn new() -> Result<Self, EngineError> {
    let panic = Arc::new(Mutex::new(None));
    self::build_pool()
      .map(|pool| Self { pool, panic })
      .map_err(|error| EngineError::new(format!("could not create thread pool: {error}")))
  }

  /// Schedules work on a background worker.
  pub fn execute(&self, operation: impl FnOnce() + Send + 'static) {
    let panic = Arc::clone(&self.panic);
    let guarded = move || {
      panic_capture::prepare();
      if let Err(payload) = catch_unwind(AssertUnwindSafe(operation)) {
        *panic.lock().unwrap_or_else(|error| error.into_inner()) = Some(panic_capture::describe(
          "background worker",
          payload.as_ref(),
        ));
      }
    };
    self.pool.spawn(guarded);
  }

  /// Takes a panic raised by background work so the engine can fail its next poll.
  pub fn take_panic(&self) -> Option<EngineError> {
    self
      .panic
      .lock()
      .unwrap_or_else(|error| error.into_inner())
      .take()
      .map(EngineError::new)
  }
}

#[cfg(not(target_arch = "wasm32"))]
fn build_pool() -> Result<ThreadPool, ThreadPoolBuildError> {
  ThreadPoolBuilder::new().build()
}

#[cfg(target_arch = "wasm32")]
fn build_pool() -> Result<ThreadPool, ThreadPoolBuildError> {
  // SAFETY: Unity links this symbol from BattlementWebThreading.jslib. init.js
  // initializes its positive thread count before Unity loads the Wasm module.
  let thread_count = unsafe { self::battlement_web_thread_count() }.max(1) as usize;
  ThreadPoolBuilder::new().num_threads(thread_count).build()
}

#[cfg(target_arch = "wasm32")]
unsafe extern "C" {
  fn battlement_web_thread_count() -> i32;
}

#[cfg(test)]
mod tests {
  use std::{thread, time::Duration};

  use crate::threading::AdaptiveThreadPool;

  #[test]
  fn background_panics_are_returned_as_engine_errors() {
    let pool = AdaptiveThreadPool::new().unwrap();
    pool.execute(|| panic!("search exploded"));

    let mut error = None;
    for _ in 0..1_000 {
      error = pool.take_panic();
      if error.is_some() {
        break;
      }
      thread::sleep(Duration::from_millis(1));
    }

    let diagnostic = error.unwrap().to_string();
    assert!(diagnostic.contains("Rust panic in background worker"));
    assert!(diagnostic.contains("Message:  \u{1b}[0msearch exploded"));
    assert!(diagnostic.contains(" BACKTRACE "));
    assert!(!diagnostic.contains("battlement_native::panic_capture"));
    assert!(pool.take_panic().is_none());
  }
}
