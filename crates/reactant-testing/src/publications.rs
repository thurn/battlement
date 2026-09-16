use std::time::Duration;

use reactant_rules::{
  Checkpoint, DisplayConnection, Game, PublicationObservation, RulesRun, RunObservation,
};

const TIMEOUT: Duration = Duration::from_secs(5);

/// Drives the real rules worker's public display consumer without host playback.
///
/// Publication waits only synchronize worker scheduling. Taking a checkpoint
/// releases its slot; neither operation advances virtual time or a frame.
pub struct PublicationDisplay<G: Game> {
  run: RulesRun<G>,
}

impl<G: Game> PublicationDisplay<G> {
  /// Starts an action with independent private state and a game-owned context.
  pub fn start(
    state: &G::State,
    action: G::Action,
    make_context: impl FnOnce(DisplayConnection<G>) -> G::Context,
  ) -> Self {
    Self {
      run: RulesRun::start(state, action, make_context),
    }
  }

  /// Waits for the supplied cumulative publication boundary, without consumption.
  pub fn wait_for_publication(&self, predicate: impl Fn(PublicationObservation) -> bool) {
    assert!(
      self.run.wait_for_publication(TIMEOUT, predicate),
      "publication wait timed out: {:?}; worker: {:?}",
      self.publication_observation(),
      self.worker_observation()
    );
  }

  /// Waits for a checkpoint and releases its slot as soon as it is taken.
  pub fn take_checkpoint(&self) -> Checkpoint<G> {
    self.wait_for_publication(|o| o.published > o.consumed || o.abandoned);
    self
      .run
      .take_checkpoint()
      .expect("run was abandoned before checkpoint consumption")
  }

  /// Takes currently available output without waiting or advancing time.
  pub fn try_take_checkpoint(&self) -> Option<Checkpoint<G>> {
    self.run.take_checkpoint()
  }

  /// Invalidates output and wakes blocked helpers without joining the worker.
  pub fn stop(&self) {
    self.run.stop();
  }

  /// Waits for real worker entry, without advancing time or frames.
  pub fn wait_for_worker_started(&self) {
    assert!(
      self.run.wait_for_worker_started(TIMEOUT),
      "worker did not start"
    );
  }

  /// Waits for worker cleanup, including interrupted builder destructors.
  pub fn wait_for_worker_stopped(&self) {
    assert!(
      self.run.wait_for_worker_stopped(TIMEOUT),
      "worker did not stop"
    );
  }

  /// Returns public publication observations, not private channel contents.
  pub fn publication_observation(&self) -> PublicationObservation {
    self.run.publication_observation()
  }

  /// Returns out-of-band lifecycle and failure observations.
  pub fn worker_observation(&self) -> RunObservation {
    self.run.observation()
  }
}
