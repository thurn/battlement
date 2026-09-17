use std::{
  cell::{Cell, RefCell},
  panic,
  rc::{Rc, Weak},
};

use reactant_rules::{ChoiceOwner, Game, PresentedPrompt, RulesContext, RulesRun, RulesWorker};

use crate::{game_app::Coordinator, game_output::PendingOutput};

/// Whether a session can accept a new rules action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
  /// Accepts a legal action immediately.
  Ready,
  /// Holds initial output, active rules, or final output awaiting submission.
  Busy,
  /// Retains accepted state for explicit recovery after failure.
  Failed,
  /// Ended publicly; worker cleanup may still be in progress.
  Stopped,
}

/// The synchronous admission result; Started does not imply completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DispatchResult {
  /// Admitted one action without waiting for its completion.
  Started,
  /// Admitted nothing and did not validate the supplied action.
  Busy,
}

/// A cloneable app-thread handle to one specific attached session.
pub struct GameHandle<G: Game> {
  pub(crate) session: Rc<GameSession<G>>,
}

pub(crate) struct GameSession<G: Game> {
  pub(crate) id: u64,
  pub(crate) automatic: Cell<bool>,
  pub(crate) worker: RulesWorker,
  pub(crate) app: Weak<Coordinator>,
  pub(crate) data: RefCell<SessionData<G>>,
}

pub(crate) struct SessionData<G: Game> {
  pub(crate) accepted: G::State,
  pub(crate) context: Option<RulesContext<G>>,
  pub(crate) run: Option<RulesRun<G>>,
  pub(crate) status: GameStatus,
  pub(crate) initial_submitted: bool,
  pub(crate) pending: Option<PendingOutput<G>>,
  pub(crate) rendered: Rc<G::State>,
  pub(crate) prompt: Option<Rc<PresentedPrompt<G::Prompt<'static>>>>,
  pub(crate) sequence: u64,
  pub(crate) diagnostic: Option<String>,
}

pub(crate) struct DispatchFault {
  pub(crate) session: u64,
  pub(crate) message: &'static str,
}

impl<G: Game> Clone for GameHandle<G> {
  fn clone(&self) -> Self {
    Self {
      session: Rc::clone(&self.session),
    }
  }
}

impl<G: Game> GameHandle<G> {
  /// Returns Busy before any legality check or queued action when occupied.
  pub fn dispatch(&self, action: G::Action) -> DispatchResult {
    self.session.refresh();
    let mut data = self.session.data.borrow_mut();
    match data.status {
      GameStatus::Busy => return DispatchResult::Busy,
      GameStatus::Stopped | GameStatus::Failed => panic::panic_any(DispatchFault {
        session: self.session.id,
        message: "cannot dispatch to a failed or stopped game",
      }),
      GameStatus::Ready => {}
    }
    let SessionData {
      accepted, context, ..
    } = &mut *data;
    let run =
      context
        .as_mut()
        .expect("ready context")
        .start(&self.session.worker, accepted, action);
    data.run = Some(run);
    data.status = GameStatus::Busy;
    drop(data);
    self.session.changed();
    DispatchResult::Started
  }

  /// Copies the last accepted state in every session status.
  pub fn accepted_state(&self) -> G::State {
    G::logical_clone(&self.session.data.borrow().accepted)
  }

  /// Stops immediately, retaining accepted state without joining a worker.
  pub fn stop(&self) {
    self.session.stop();
  }

  /// Returns readiness independent of native playback.
  pub fn status(&self) -> GameStatus {
    self.session.refresh();
    self.session.data.borrow().status
  }

  /// Returns detailed failure information for diagnostics and recovery UI.
  pub fn diagnostic(&self) -> Option<String> {
    self.session.refresh();
    self.session.data.borrow().diagnostic.clone()
  }
}

impl<G: Game> GameSession<G> {
  pub(crate) fn changed(&self) {
    if let Some(app) = self.app.upgrade() {
      app.changed();
    }
  }

  pub(crate) fn refresh(&self) {
    let mut data = self.data.borrow_mut();
    if data.status != GameStatus::Busy {
      return;
    }
    if data.prompt.as_ref().is_some_and(|prompt| {
      prompt.handle.owner() == ChoiceOwner::Human && !prompt.handle.is_active()
    }) {
      data.prompt = None;
      self.changed();
    }
    if let Some(message) = data.run.as_ref().and_then(|run| run.observation().failure) {
      drop(data);
      self.fail(message);
      return;
    }
    if data.run.is_none() && data.initial_submitted && self.worker.is_idle() {
      data.status = GameStatus::Ready;
      drop(data);
      self.changed();
    }
  }

  pub(crate) fn stop(&self) {
    let mut data = self.data.borrow_mut();
    if data.status == GameStatus::Stopped {
      return;
    }
    data.status = GameStatus::Stopped;
    if let Some(run) = &data.run {
      run.stop();
    }
    data.pending = None;
    data.prompt = None;
    data.context = None;
    drop(data);
    self.changed();
  }

  pub(crate) fn fail(&self, message: String) {
    let mut data = self.data.borrow_mut();
    if data.status == GameStatus::Stopped {
      return;
    }
    data.status = GameStatus::Failed;
    data.diagnostic = Some(message);
    if let Some(run) = &data.run {
      run.stop();
    }
    data.pending = None;
    data.prompt = None;
    data.context = None;
    drop(data);
    self.changed();
  }
}
