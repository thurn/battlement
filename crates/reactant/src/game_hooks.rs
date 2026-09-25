use std::{
  any::{Any, TypeId},
  rc::Rc,
  time::Duration,
};

use battlement::{Command, CommandBody, ObjectId, WaitPayload};
use reactant_core::{
  animation_controls::{AnimationScope, AnimationSequence},
  app_context::{AppHandle, use_app},
  app_runtime::ApplicationContext,
  component::Component,
  context::ContextProvider,
  hooks,
  key::KeyRenderExt,
  motion_value::AnimationPlayback,
  render::{Node, Render},
  work_scope::WorkScope,
};
use reactant_rules::{ChoiceOwner, Game, PresentedPrompt, PublicationObservation, RunObservation};

use crate::game_app::ServicesContext;
use crate::game_session::{GameObservation, GameStatus};

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
  pub(crate) motion_ready: bool,
  pub(crate) state: Rc<dyn Any>,
  pub(crate) prompt: Option<Rc<dyn Any>>,
  pub(crate) animation_sequence: Option<u64>,
  pub(crate) animation: Option<Rc<dyn Any>>,
  pub(crate) worker: RunObservation,
  pub(crate) publications: PublicationObservation,
}

/// One native operation authored from a queued semantic game event.
pub struct SnapshotAnimation {
  operation: SnapshotAnimationOperation,
}

/// App-owned control for the currently attached game's presentation clock.
#[derive(Clone)]
pub struct GamePresentation {
  app: AppHandle,
  scope: u64,
  owner_id: ObjectId,
}

enum SnapshotAnimationOperation {
  Sequence {
    scope: AnimationScope,
    sequence: AnimationSequence,
    blocking: bool,
  },
  Wait(Duration),
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
      .map(|context| {
        ContextProvider::new()
          .context(WorkScope(context.id))
          .child(self.child.clone().key(context.id))
      })
  }
}

/// Reads the currently rendered immutable snapshot and subscribes to its changes.
pub fn use_game_state<G: Game>() -> Rc<G::State> {
  self::context::<G>()
    .state
    .downcast()
    .unwrap_or_else(|_| panic!("game state type mismatch"))
}

/// Selects part of the immutable rendered game state using value equality.
pub fn use_game_selector<G: Game, V: Clone + PartialEq + 'static>(
  select: impl Fn(&G::State) -> V + 'static,
) -> V {
  self::use_game_selector_with::<G, V>(select, PartialEq::eq)
}

/// Selects rendered game state with an explicit comparison for subscriber updates.
pub fn use_game_selector_with<G: Game, V: Clone + 'static>(
  select: impl Fn(&G::State) -> V + 'static,
  equal: impl Fn(&V, &V) -> bool + 'static,
) -> V {
  hooks::use_required_context_selector::<ApplicationContext, V>(
    move |application| {
      let context = application
        .value::<ServicesContext>()
        .and_then(|context| context.game.clone())
        .expect("no game is attached");
      assert!(
        context.game == TypeId::of::<G>(),
        "attached game type mismatch"
      );
      let state = context
        .state
        .downcast::<G::State>()
        .unwrap_or_else(|_| panic!("game state type mismatch"));
      select(&state)
    },
    equal,
  )
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

/// Reports completion of blocking native sequences authored through `use_animate`.
/// Rules readiness is independent; nonblocking motion and fixed pacing waits are excluded.
pub fn use_game_motion_ready<G: Game>() -> bool {
  self::context::<G>().motion_ready
}

/// Reads the typed event attached to the current game publication.
pub fn use_game_publication<G: Game>() -> Option<Rc<G::StateAnimation>> {
  self::context::<G>().animation.map(|animation| {
    animation
      .downcast::<G::StateAnimation>()
      .unwrap_or_else(|_| panic!("game publication type mismatch"))
  })
}

/// Subscribes to the attached session's worker and publication diagnostics.
pub fn use_game_observation<G: Game>() -> GameObservation {
  let context = self::context::<G>();
  GameObservation {
    status: context.status,
    worker: context.worker,
    publications: context.publications,
  }
}

/// Captures a reference-counted pause owner for the currently attached game.
///
/// Each hook call owns an independent pause. A release resumes gameplay only
/// after every owner has released, and a handle captured from an old game can
/// never affect its replacement.
pub fn use_game_presentation() -> GamePresentation {
  let context = self::attached().expect("no game is attached");
  GamePresentation {
    app: use_app(),
    scope: context.id,
    owner_id: hooks::use_memo(ObjectId::new_v4, ()),
  }
}

impl GamePresentation {
  /// Freezes this game's active and queued presentation work.
  pub fn pause(&self) {
    self
      .app
      .set_game_presentation_paused(self.scope, self.owner_id, true);
  }

  /// Releases this owner's pause without disturbing other pause owners.
  pub fn resume(&self) {
    self
      .app
      .set_game_presentation_paused(self.scope, self.owner_id, false);
  }
}

/// Authors at most one native operation from the current snapshot's typed event.
///
/// The operation is submitted with the consuming render and starts only once,
/// even when display-local state rerenders while that output remains pending.
pub fn use_animate<G: Game>(author: impl FnOnce(&G::StateAnimation) -> Option<SnapshotAnimation>) {
  let context = self::context::<G>();
  let app = use_app();
  let services = hooks::use_required_context::<ApplicationContext>()
    .value::<ServicesContext>()
    .expect("snapshot animation requires a Reactant Application root");
  let coordinator = services.coordinator.upgrade().expect("active application");
  let game = coordinator.game::<G>().expect("attached animation game");
  let identity = format!("{}:{:?}", hooks::use_id(), context.animation_sequence);
  let animation = context.animation.map(|animation| {
    animation
      .downcast::<G::StateAnimation>()
      .unwrap_or_else(|_| panic!("game animation type mismatch"))
  });
  let plan = animation.as_deref().and_then(author);
  hooks::use_commit_effect(
    move || {
      if let Some(plan) = plan
        && let Some(playback) = plan.submit(&app)
      {
        game.track_blocking_motion(identity, playback);
      }
    },
    (context.id, context.animation_sequence),
  );
}

impl SnapshotAnimation {
  /// Creates a gameplay sequence that blocks later queued gameplay work.
  #[must_use]
  pub fn sequence(scope: AnimationScope, sequence: AnimationSequence) -> Self {
    Self {
      operation: SnapshotAnimationOperation::Sequence {
        scope,
        sequence,
        blocking: true,
      },
    }
  }

  /// Lets later queued work advance while this sequence continues locally.
  #[must_use]
  pub fn nonblocking(mut self) -> Self {
    let SnapshotAnimationOperation::Sequence { blocking, .. } = &mut self.operation else {
      panic!("a fixed wait cannot be nonblocking")
    };
    *blocking = false;
    self
  }

  /// Creates a blocking fixed-duration pacing operation.
  #[must_use]
  pub fn wait(duration: Duration) -> Self {
    assert!(
      !duration.is_zero(),
      "snapshot animation wait must be positive"
    );
    Self {
      operation: SnapshotAnimationOperation::Wait(duration),
    }
  }

  fn submit(self, app: &AppHandle) -> Option<AnimationPlayback> {
    match self.operation {
      SnapshotAnimationOperation::Sequence {
        scope,
        sequence,
        blocking,
      } => {
        if blocking {
          Some(scope.start_blocking(sequence))
        } else {
          scope.start(sequence);
          None
        }
      }
      SnapshotAnimationOperation::Wait(duration) => {
        let micros = duration.as_micros();
        let millis = micros.div_ceil(1_000);
        let duration_ms = u64::try_from(millis).expect("snapshot animation wait is too long");
        app.send(Command::new_v4(CommandBody::TimeWait(WaitPayload {
          duration_ms,
        })));
        None
      }
    }
  }
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
    .value::<ServicesContext>()
    .and_then(|context| context.game.clone())
}
