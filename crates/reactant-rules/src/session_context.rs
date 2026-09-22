use std::{
  any::Any,
  rc::Rc,
  sync::{Arc, Mutex, Weak},
};

use crate::{DisplayConnection, Game, RulesRun, publication::Publications, worker::WorkerSlot};

pub(crate) type PublicationTarget<G> = Arc<Mutex<Weak<Publications<G>>>>;

/// One app-thread coordinator shared across game replacements.
#[derive(Clone)]
pub struct RulesWorker {
  pub(crate) slot: Rc<WorkerSlot>,
  pub(crate) inline: bool,
  choices: Option<Rc<dyn Any>>,
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
      inline: false,
      choices: None,
    }
  }
}

/// An injected answer resolver retains normal typed prompt validation.
pub(crate) struct ChoiceResolver<G: Game>(pub(crate) Arc<PromptResolver<G>>);

type PromptResolver<G> = dyn for<'a> Fn(&<G as Game>::Prompt<'a>) -> usize + Send + Sync;

impl RulesWorker {
  /// Runs finite actions on the caller, collecting publications without blocking.
  /// Human prompts require an explicit answer resolver; production uses `default`.
  pub fn inline() -> Self {
    Self {
      inline: true,
      ..Self::default()
    }
  }

  /// Supplies human answers for one game's inline actions, as prompt option indices.
  pub fn answer_with<G: Game>(
    mut self,
    resolver: impl for<'a> Fn(&G::Prompt<'a>) -> usize + Send + Sync + 'static,
  ) -> Self {
    assert!(self.inline, "scripted answers require inline execution");
    self.choices = Some(Rc::new(ChoiceResolver::<G>(Arc::new(resolver))));
    self
  }

  /// Reports whether actions finish on the caller without worker synchronization.
  pub fn is_inline(&self) -> bool {
    self.inline
  }

  pub(crate) fn choices<G: Game>(&self) -> Option<ChoiceResolver<G>> {
    self.choices.as_ref().map(|value| {
      let value = value
        .downcast_ref::<ChoiceResolver<G>>()
        .expect("scripted answers belong to another game");
      ChoiceResolver(Arc::clone(&value.0))
    })
  }

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
