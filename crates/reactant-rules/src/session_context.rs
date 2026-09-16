use std::{
  rc::Rc,
  sync::{Arc, Mutex, Weak},
};

use crate::{DisplayConnection, Game, RulesRun, publication::Publications, worker::WorkerSlot};

pub(crate) type PublicationTarget<G> = Arc<Mutex<Weak<Publications<G>>>>;

/// One app-thread coordinator shared across game replacements.
#[derive(Clone)]
pub struct RulesWorker {
  pub(crate) slot: Rc<WorkerSlot>,
}

/// A session's reusable domain context, transferred to each action's worker.
pub struct RulesContext<G: Game> {
  pub(crate) target: PublicationTarget<G>,
  context: Option<G::Context>,
}

/// Normal-return state and context, accepted only after output submission.
pub struct CompletedAction<G: Game> {
  pub(crate) state: G::State,
  pub(crate) context: G::Context,
  pub(crate) target: PublicationTarget<G>,
}

impl Default for RulesWorker {
  fn default() -> Self {
    Self {
      slot: Rc::new(WorkerSlot::new()),
    }
  }
}

impl RulesWorker {
  /// Reports cleanup completion without starting or joining any worker.
  pub fn is_idle(&self) -> bool {
    self.slot.is_idle()
  }
}

impl<G: Game> RulesContext<G> {
  /// Constructs the context exactly once on the caller's thread.
  pub fn new(make_context: impl FnOnce(DisplayConnection<G>) -> G::Context) -> Self {
    let target = Arc::new(Mutex::new(Weak::new()));
    let context = make_context(DisplayConnection {
      target: Arc::clone(&target),
    });
    Self {
      target,
      context: Some(context),
    }
  }

  /// Validates before cloning, then transfers this context to a bounded worker.
  pub fn start(
    &mut self,
    worker: &RulesWorker,
    state: &G::State,
    action: G::Action,
  ) -> RulesRun<G> {
    assert!(G::is_legal_action(state, &action), "illegal rules action");
    self.start_private(worker, G::logical_clone(state), action)
  }

  pub(crate) fn start_private(
    &mut self,
    worker: &RulesWorker,
    state: G::State,
    action: G::Action,
  ) -> RulesRun<G> {
    let context = self
      .context
      .take()
      .expect("rules context is already executing");
    RulesRun::spawn(worker, Arc::clone(&self.target), state, context, action)
  }

  /// Reclaims the context and accepted state after successful Rust submission.
  pub fn accept(&mut self, completion: CompletedAction<G>) -> G::State {
    assert!(
      Arc::ptr_eq(&self.target, &completion.target),
      "completion belongs to another session"
    );
    assert!(self.context.is_none(), "rules context was already returned");
    *self.target.lock().unwrap() = Weak::new();
    self.context = Some(completion.context);
    completion.state
  }
}
