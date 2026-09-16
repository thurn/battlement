use std::{error::Error, fmt, time::Duration};

use battlement::{Connect, ScreenSize};
use battlement_fake::assets::FakeAssetCatalog;
use battlement_native::EngineError;
use exported_worker_fixture::{
  FixtureEngine, ProofObserver, ProofSnapshot, WORKER_COMMAND_TYPE, WORKER_SCENE_ADDRESS,
  WorkerFixture, worker_action_object_id,
};

use crate::Display;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Configures an exported-worker display fixture and its bounded waits.
pub struct WorkerDisplayBuilder {
  timeout: Duration,
}

impl WorkerDisplayBuilder {
  /// Sets the maximum wall-clock hang timeout for each event-driven wait.
  #[must_use]
  pub fn timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }

  /// Builds the real exported engine, connects it, and retains its observer.
  pub fn build(self) -> Result<WorkerDisplay, EngineError> {
    let fixture = WorkerFixture::create()?;
    let (engine, observer) = fixture.into_parts();
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(WORKER_SCENE_ADDRESS);
    let mut connect = Connect::new(
      "reactant-testing",
      "reactant-testing",
      ScreenSize::new(1_280, 720),
    );
    connect.custom_command_types = vec![WORKER_COMMAND_TYPE.to_owned()];
    Ok(WorkerDisplay {
      display: Display::connect_with(engine, assets, connect),
      observer,
      timeout: self.timeout,
    })
  }
}

impl Default for WorkerDisplayBuilder {
  fn default() -> Self {
    Self {
      timeout: DEFAULT_TIMEOUT,
    }
  }
}

/// A display backed by the exported native worker fixture.
///
/// Worker waits block on fixture condition variables. They never advance the
/// display's virtual time, poll the engine, or record a rendered frame.
pub struct WorkerDisplay {
  display: Display<FixtureEngine>,
  observer: ProofObserver,
  timeout: Duration,
}

impl WorkerDisplay {
  /// Starts configuring a worker display.
  #[must_use]
  pub fn builder() -> WorkerDisplayBuilder {
    WorkerDisplayBuilder::default()
  }

  /// Returns the public display controls and observations.
  #[must_use]
  pub fn display(&self) -> &Display<FixtureEngine> {
    &self.display
  }

  /// Returns mutable public display controls and observations.
  #[must_use]
  pub fn display_mut(&mut self) -> &mut Display<FixtureEngine> {
    &mut self.display
  }

  /// Clicks the fixture's public world object to enter its waiting barrier.
  pub fn enter_waiting_barrier(&mut self) {
    self.display.click(worker_action_object_id());
  }

  /// Reconnects through the exported engine's public lifecycle path.
  pub fn reconnect(&mut self) {
    self.display.reconnect();
  }

  /// Waits until at least `minimum` worker tasks have started.
  pub fn wait_for_worker_started(&self, minimum: usize) -> Result<ProofSnapshot, WorkerWaitError> {
    self.finish_wait(
      format!("at least {minimum} worker task(s) to start"),
      self.observer.wait_for_worker_started(minimum, self.timeout),
    )
  }

  /// Waits until at least `minimum` worker tasks have stopped.
  pub fn wait_for_worker_stopped(&self, minimum: usize) -> Result<ProofSnapshot, WorkerWaitError> {
    self.finish_wait(
      format!("at least {minimum} worker task(s) to stop"),
      self.observer.wait_for_worker_stopped(minimum, self.timeout),
    )
  }

  /// Waits until rules enter the cancellation-aware nested wait barrier.
  pub fn wait_for_waiting_barrier(&self) -> Result<ProofSnapshot, WorkerWaitError> {
    self.finish_wait(
      "the cancellation-aware waiting barrier".to_owned(),
      self.observer.wait_for_waiting(self.timeout),
    )
  }

  /// Waits until rules enter the held ordinary-computation barrier.
  pub fn wait_for_computation_barrier(&self) -> Result<ProofSnapshot, WorkerWaitError> {
    self.finish_wait(
      "the ordinary-computation barrier".to_owned(),
      self.observer.wait_for_computing(self.timeout),
    )
  }

  /// Releases the fixture's held ordinary-computation barrier.
  pub fn release_computation(&self) {
    self.observer.release_computation();
  }

  /// Returns the current worker observation without blocking.
  #[must_use]
  pub fn worker_observation(&self) -> ProofSnapshot {
    self.observer.snapshot()
  }

  fn finish_wait(
    &self,
    condition: String,
    result: Option<ProofSnapshot>,
  ) -> Result<ProofSnapshot, WorkerWaitError> {
    result.ok_or_else(|| WorkerWaitError {
      condition,
      timeout: self.timeout,
      last_observation: self.observer.snapshot(),
    })
  }
}

/// A bounded worker wait that expired before its lifecycle condition occurred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerWaitError {
  condition: String,
  timeout: Duration,
  last_observation: ProofSnapshot,
}

impl WorkerWaitError {
  /// Returns the worker observation captured when the timeout expired.
  #[must_use]
  pub fn last_observation(&self) -> ProofSnapshot {
    self.last_observation
  }

  /// Returns the configured maximum wait duration.
  #[must_use]
  pub fn timeout(&self) -> Duration {
    self.timeout
  }
}

impl fmt::Display for WorkerWaitError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      formatter,
      "timed out after {:?} waiting for {}; last worker observation: {:?}",
      self.timeout, self.condition, self.last_observation
    )
  }
}

impl Error for WorkerWaitError {}
