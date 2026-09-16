//! Executor types used by asynchronous Reactant resources.

use std::{future::Future, pin::Pin};

/// A sendable boxed future accepted by a [`Spawner`].
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Starts asynchronous work for a Reactant runtime.
pub trait Spawner: 'static {
  /// Starts one task and returns its cancellation handle.
  fn spawn(&self, task: BoxFuture<'static, ()>) -> SpawnedTask;
}

/// Owns one best-effort task cancellation request.
#[must_use]
pub struct SpawnedTask {
  cancel: Option<Box<dyn FnOnce() + Send + 'static>>,
}

impl SpawnedTask {
  /// Creates a handle that invokes `cancel` at most once.
  pub fn new(cancel: impl FnOnce() + Send + 'static) -> Self {
    Self {
      cancel: Some(Box::new(cancel)),
    }
  }

  /// Creates a task handle with no cancellation facility.
  pub const fn detached() -> Self {
    Self { cancel: None }
  }

  /// Requests cancellation and consumes the handle.
  pub fn cancel(mut self) {
    self.cancel_now();
  }

  /// Consumes the handle without requesting cancellation.
  pub fn disarm(mut self) {
    self.cancel = None;
  }

  fn cancel_now(&mut self) {
    if let Some(cancel) = self.cancel.take() {
      cancel();
    }
  }
}

impl Drop for SpawnedTask {
  fn drop(&mut self) {
    self.cancel_now();
  }
}
