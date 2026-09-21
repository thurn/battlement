use std::{
  rc::{Rc, Weak},
  time::Duration,
};

use reactant_core::app_runtime::AppOutput;

use reactant_rules::{
  CompletedAction, Game, PresentedPrompt, PublicationObservation, RunObservation,
};

use crate::game_session::{GameSession, GameStatus};

/// The real session's Rust-side consumer, independent of host clocks and frames.
pub struct GameConsumer<G: Game> {
  pub(crate) session: Rc<GameSession<G>>,
}

/// One retained output whose successful submission can accept an action.
pub struct GameOutput<G: Game> {
  session: Weak<GameSession<G>>,
  sequence: u64,
  scope: u64,
  state: Rc<G::State>,
  prompt: Option<Rc<PresentedPrompt<G::Prompt<'static>>>>,
  animation: Option<Rc<G::StateAnimation>>,
  final_output: bool,
}

pub(crate) struct PendingOutput<G: Game> {
  pub(crate) sequence: u64,
  pub(crate) completion: Option<CompletedAction<G>>,
  pub(crate) animation: Option<Rc<G::StateAnimation>>,
}

impl<G: Game> GameConsumer<G> {
  /// Returns publication consumption to the app while retaining diagnostic observations.
  pub fn resume_automatic_submission(&self) {
    self.session.automatic.set(true);
  }

  /// Takes the next FIFO entry, releasing its worker slot before submission.
  /// Retains at most one unsubmitted output and returns it again until resolved.
  pub fn take_output(&self) -> Option<GameOutput<G>> {
    self.session.refresh();
    let mut data = self.session.data.borrow_mut();
    if matches!(data.status, GameStatus::Stopped | GameStatus::Failed) {
      return None;
    }
    if data.pending.is_none() {
      let checkpoint = data.run.as_ref()?.take_checkpoint()?.into_parts();
      data.sequence += 1;
      data.rendered = Rc::new(checkpoint.state);
      data.prompt = checkpoint.prompt.map(Rc::new);
      data.pending = Some(PendingOutput {
        sequence: data.sequence,
        completion: checkpoint.completion,
        animation: checkpoint.animation.map(Rc::new),
      });
      self.session.changed();
    }
    let pending = data.pending.as_ref().expect("pending output");
    Some(GameOutput {
      session: Rc::downgrade(&self.session),
      sequence: pending.sequence,
      scope: self.session.id,
      state: Rc::clone(&data.rendered),
      prompt: data.prompt.clone(),
      animation: pending.animation.clone(),
      final_output: pending.completion.is_some(),
    })
  }

  /// Waits only for worker publication, without consuming or advancing playback.
  pub fn wait_for_output(&self, timeout: Duration) -> bool {
    let data = self.session.data.borrow();
    if data.pending.is_some() {
      return true;
    }
    data.run.as_ref().is_some_and(|run| {
      run.wait_for_publication(timeout, |o| o.published > o.consumed || o.abandoned)
    })
  }

  /// Waits for a publication boundary without consuming output or advancing host time.
  pub fn wait_for_publication(
    &self,
    timeout: Duration,
    predicate: impl Fn(PublicationObservation) -> bool,
  ) -> bool {
    self
      .session
      .data
      .borrow()
      .run
      .as_ref()
      .is_some_and(|run| run.wait_for_publication(timeout, predicate))
  }

  /// Reports out-of-band worker lifecycle, including cleanup after public stop.
  pub fn worker_observation(&self) -> RunObservation {
    self
      .session
      .data
      .borrow()
      .run
      .as_ref()
      .map_or_else(RunObservation::default, |run| run.observation())
  }

  /// Reports the bounded publication boundaries for the current action.
  pub fn publication_observation(&self) -> PublicationObservation {
    self
      .session
      .data
      .borrow()
      .run
      .as_ref()
      .map_or_else(PublicationObservation::default, |run| {
        run.publication_observation()
      })
  }

  /// Waits for worker entry without changing virtual time or frames.
  pub fn wait_for_worker_started(&self, timeout: Duration) -> bool {
    self
      .session
      .data
      .borrow()
      .run
      .as_ref()
      .is_some_and(|run| run.wait_for_worker_started(timeout))
  }

  /// Waits for all current worker-owned cleanup without changing host time.
  pub fn wait_for_worker_stopped(&self, timeout: Duration) -> bool {
    self
      .session
      .data
      .borrow()
      .run
      .as_ref()
      .is_none_or(|run| run.wait_for_worker_stopped(timeout))
  }

  /// Reports a gameplay-host failure without reversing accepted state.
  pub fn fail(&self, message: impl Into<String>) {
    self.session.fail(message.into());
  }
}

impl<G: Game> GameOutput<G> {
  /// Returns the immutable snapshot for rendering.
  pub fn state(&self) -> &G::State {
    &self.state
  }
  /// Returns the request associated with this rendered snapshot.
  pub fn prompt(&self) -> Option<&PresentedPrompt<G::Prompt<'static>>> {
    self.prompt.as_deref()
  }
  /// Returns the semantic event for this output alone.
  pub fn animation(&self) -> Option<&G::StateAnimation> {
    self.animation.as_deref()
  }
  /// Reports normal-return output, independent of host playback.
  pub fn is_final(&self) -> bool {
    self.final_output
  }

  /// Acknowledges successful Rust output submission exactly once.
  /// Ended-session and duplicate submissions are harmless.
  pub fn submitted(&self) {
    let Some(session) = self.session.upgrade() else {
      return;
    };
    let mut data = session.data.borrow_mut();
    if matches!(data.status, GameStatus::Stopped | GameStatus::Failed) {
      return;
    }
    if !data
      .pending
      .as_ref()
      .is_some_and(|pending| pending.sequence == self.sequence)
    {
      return;
    }
    let pending = data.pending.take().expect("matching output");
    if let Some(completion) = pending.completion {
      data.accepted = data
        .context
        .as_mut()
        .expect("active session context")
        .accept(completion);
      data.completed_actions = data
        .completed_actions
        .checked_add(1)
        .expect("completed game action count overflow");
      data.status = GameStatus::Ready;
    } else if !data.initial_submitted {
      data.initial_submitted = true;
      if session.worker.is_idle() {
        data.status = GameStatus::Ready;
      }
    }
    drop(data);
    session.changed();
  }
}

impl<G: Game> AppOutput for GameOutput<G> {
  fn scope(&self) -> u64 {
    self.scope
  }
  fn submitted(&self) {
    self.submitted();
  }
}
