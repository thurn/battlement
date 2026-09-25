use std::{
  cell::RefCell,
  fmt,
  future::Future,
  pin::Pin,
  rc::Rc,
  sync::{Arc, Mutex},
  task::{Context, Poll},
  time::Duration,
};

use crate::{
  RunObservation,
  worker::{WorkerConnection, WorkerSlot},
  worker_observer::{WorkerEvent, WorkerObserver},
};

/// One owner-thread computation slot, independent of every rules-action slot.
/// Replacement retains only the latest request while active work cooperatively exits.
pub struct ComputationLane<T> {
  slot: Rc<WorkerSlot>,
  current: RefCell<Option<OwnedComputation<T>>>,
}

/// Cooperative worker cancellation; check at every bounded work transition.
pub struct CancellationToken {
  connection: WorkerConnection,
}

/// Observes one computation and delivers its result once after worker cleanup.
/// Dropping the lane or replacing its work invalidates this result immediately.
pub struct Computation<T> {
  #[cfg(feature = "platform-proof")]
  pub(crate) id: u64,
  output: Arc<Mutex<Output<T>>>,
  worker: WorkerObserver,
}

/// Readiness is separate from whether an invalidated worker has finished cleanup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComputationStatus {
  Pending,
  Ready,
  Cancelled,
  Failed(String),
}

/// Expected ownership cancellation or unexpected computation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComputationError {
  Cancelled,
  Failed(String),
}

struct OwnedComputation<T> {
  id: u64,
  output: Arc<Mutex<Output<T>>>,
  worker: WorkerObserver,
}

struct Output<T> {
  valid: bool,
  value: Option<T>,
}

impl<T> Default for ComputationLane<T> {
  fn default() -> Self {
    Self {
      slot: Rc::new(WorkerSlot::new()),
      current: RefCell::new(None),
    }
  }
}

impl<T: Send + 'static> ComputationLane<T> {
  /// Invalidates previous results without joining, then schedules the latest work.
  pub fn replace(
    &self,
    work: impl FnOnce(CancellationToken) -> T + Send + 'static,
  ) -> Computation<T> {
    self.cancel();
    let output = Arc::new(Mutex::new(Output {
      valid: true,
      value: None,
    }));
    let result = Arc::clone(&output);
    let worker = self.slot.observe_next();
    let id = self.slot.replace(move |connection| {
      let token = CancellationToken { connection };
      token.checkpoint();
      let value = work(token);
      let mut output = result.lock().unwrap();
      if output.valid {
        output.value = Some(value);
      }
    });
    *self.current.borrow_mut() = Some(OwnedComputation {
      id,
      output: Arc::clone(&output),
      worker: worker.clone(),
    });
    Computation {
      #[cfg(feature = "platform-proof")]
      id,
      output,
      worker,
    }
  }
}

impl<T> ComputationLane<T> {
  #[cfg(feature = "platform-proof")]
  pub(crate) fn observer(&self) -> WorkerObserver {
    self.slot.observer()
  }

  /// Cancels active/pending work and discards retained results without a UI join.
  pub fn cancel(&self) {
    if let Some(current) = self.current.borrow_mut().take() {
      let value = {
        let mut output = current.output.lock().unwrap();
        output.valid = false;
        output.value.take()
      };
      self.slot.cancel_run(current.id);
      current.worker.wake();
      drop(value);
    }
  }
}

impl<T> Drop for ComputationLane<T> {
  fn drop(&mut self) {
    self.cancel();
  }
}

impl CancellationToken {
  #[cfg(feature = "platform-proof")]
  pub(crate) fn wait_until_cancelled(&self) -> ! {
    self.connection.wait_until_cancelled()
  }

  /// Unwinds cancelled work through the existing worker cleanup boundary.
  pub fn checkpoint(&self) {
    self.connection.check_cancelled();
  }

  /// Allows algorithms to finish a bounded cleanup path before their next checkpoint.
  pub fn is_cancelled(&self) -> bool {
    self.connection.is_cancelled()
  }
}

impl<T> Computation<T> {
  /// Returns ownership readiness without blocking or taking the result.
  pub fn status(&self) -> ComputationStatus {
    if !self.output.lock().unwrap().valid {
      return ComputationStatus::Cancelled;
    }
    let observation = self.observation();
    if let Some(message) = observation.failure {
      return ComputationStatus::Failed(message);
    }
    if observation.cancelled {
      return ComputationStatus::Cancelled;
    }
    if observation.stopped && observation.completed {
      ComputationStatus::Ready
    } else {
      ComputationStatus::Pending
    }
  }

  /// Takes a current successful result once, after worker-owned cleanup.
  pub fn take_result(&self) -> Option<T> {
    if self.status() != ComputationStatus::Ready {
      return None;
    }
    let mut output = self.output.lock().unwrap();
    if output.valid {
      output.value.take()
    } else {
      None
    }
  }

  /// Observes physical worker lifecycle, including cleanup after cancellation.
  pub fn observation(&self) -> RunObservation {
    self.worker.observation()
  }

  /// A bounded diagnostic/test wait; application UI should await the computation instead.
  pub fn wait_for_stopped(&self, timeout: Duration) -> bool {
    self.worker.wait_for(timeout, |events| {
      events
        .iter()
        .any(|event| matches!(event, WorkerEvent::Stopped(_)))
    })
  }
}

impl<T> Future for Computation<T> {
  type Output = Result<T, ComputationError>;

  fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
    self.worker.register_waker(context.waker());
    let status = self.status();
    if status != ComputationStatus::Pending {
      self.worker.clear_waker();
    }
    match status {
      ComputationStatus::Pending => Poll::Pending,
      ComputationStatus::Cancelled => Poll::Ready(Err(ComputationError::Cancelled)),
      ComputationStatus::Failed(message) => Poll::Ready(Err(ComputationError::Failed(message))),
      ComputationStatus::Ready => {
        let mut output = self.output.lock().unwrap();
        if !output.valid {
          return Poll::Ready(Err(ComputationError::Cancelled));
        }
        Poll::Ready(Ok(
          output
            .value
            .take()
            .expect("computation result already consumed"),
        ))
      }
    }
  }
}

impl fmt::Display for ComputationError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Cancelled => f.write_str("computation cancelled"),
      Self::Failed(message) => f.write_str(message),
    }
  }
}

impl std::error::Error for ComputationError {}
