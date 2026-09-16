use std::{
  any::{Any, TypeId},
  rc::Rc,
};

use reactant_core::{
  app_runtime::ApplicationContext,
  component::Component,
  hooks,
  key::KeyRenderExt,
  render::{Node, Render},
};
use reactant_rules::{ChoiceOwner, Game, PresentedPrompt};

use crate::game_session::GameStatus;

/// A game subtree with a fresh component and native lifetime on replacement.
/// Place persistent menus alongside this boundary.
pub struct GameRoot {
  child: Node,
}

#[derive(Clone)]
pub(crate) struct GameRenderContext {
  pub(crate) game: TypeId,
  pub(crate) id: u64,
  pub(crate) revision: u64,
  pub(crate) status: GameStatus,
  pub(crate) state: Rc<dyn Any>,
  pub(crate) prompt: Option<Rc<dyn Any>>,
}

impl PartialEq for GameRenderContext {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id && self.revision == other.revision
  }
}

impl GameRoot {
  /// Declares the portion of the app whose lifetime belongs to its attached game.
  pub fn new(child: impl Render) -> Self {
    Self {
      child: Node::new(child),
    }
  }
}

impl Component for GameRoot {
  fn render(&self) -> impl Render {
    let context = self::attached();
    context
      .filter(|context| context.status != GameStatus::Stopped)
      .map(|context| self.child.clone().key(context.id))
  }
}

/// Reads the currently rendered immutable snapshot and subscribes to its changes.
pub fn use_game_state<G: Game>() -> Rc<G::State> {
  self::context::<G>()
    .state
    .downcast()
    .unwrap_or_else(|_| panic!("game state type mismatch"))
}

/// Reads the rendered prompt; resolved human prompts close immediately.
pub fn use_game_prompt<G: Game>() -> Option<Rc<PresentedPrompt<G::Prompt<'static>>>> {
  let context = self::context::<G>();
  let prompt = context
    .prompt?
    .downcast::<PresentedPrompt<G::Prompt<'static>>>()
    .unwrap_or_else(|_| panic!("game prompt type mismatch"));
  if prompt.handle.owner() == ChoiceOwner::Human && !prompt.handle.is_active() {
    return None;
  }
  Some(prompt)
}

/// Subscribes to the attached session's readiness and recovery state.
pub fn use_game_status<G: Game>() -> GameStatus {
  self::context::<G>().status
}

fn context<G: Game>() -> GameRenderContext {
  let context = self::attached().expect("no game is attached");
  assert!(
    context.game == TypeId::of::<G>(),
    "attached game type mismatch"
  );
  context
}

fn attached() -> Option<GameRenderContext> {
  hooks::use_required_context::<ApplicationContext>()
    .value::<Option<GameRenderContext>>()
    .and_then(|context| (*context).clone())
}
