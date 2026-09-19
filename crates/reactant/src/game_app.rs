use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  panic::{self, AssertUnwindSafe},
  rc::Rc,
};

use reactant_core::{
  app::App,
  app_runtime::{AppOutput, AppRuntime},
};
use reactant_rules::{DisplayConnection, Game, RulesContext, RulesWorker};

use crate::{
  game_hooks::GameRenderContext,
  game_output::{GameConsumer, PendingOutput},
  game_session::{DispatchFault, GameHandle, GameSession, GameStatus, SessionData},
};

/// App-owned game attachment and the Rust publication consumer.
/// Import this trait from `reactant::prelude` to extend the existing UI-only App.
pub trait GameApp {
  /// Accepts initial state, constructs its context once, and replaces the old game.
  fn start_game<G: Game>(
    &mut self,
    initial_state: G::State,
    make_context: impl FnOnce(DisplayConnection<G>) -> G::Context,
  ) -> GameHandle<G>;
  /// Returns the attached game's consumer for rendering and output submission.
  fn game_consumer<G: Game>(&mut self) -> GameConsumer<G>;
}

#[derive(Default)]
pub(crate) struct Coordinator {
  current: RefCell<Option<Rc<dyn AttachedSession>>>,
  worker: RulesWorker,
  next: Cell<u64>,
  revision: Cell<u64>,
  observed: Cell<u64>,
  observed_game: RefCell<Option<(u64, crate::GameObservation)>>,
}

pub(crate) trait AttachedSession: Any {
  fn stop(&self);
  fn refresh(&self);
  fn fail(&self, message: String);
  fn view(&self, revision: u64) -> GameRenderContext;
  fn observation(&self) -> crate::GameObservation;
  fn id(&self) -> u64;
  fn active(&self) -> bool;
  fn output(self: Rc<Self>) -> Option<Box<dyn AppOutput>>;
}

impl<M: 'static> GameApp for App<M> {
  fn start_game<G: Game>(
    &mut self,
    initial_state: G::State,
    make_context: impl FnOnce(DisplayConnection<G>) -> G::Context,
  ) -> GameHandle<G> {
    let coordinator = self.application_runtime(Coordinator::default);
    if let Some(previous) = coordinator.current.borrow_mut().take() {
      previous.stop();
    }
    let context = RulesContext::new(make_context);
    let rendered = Rc::new(G::logical_clone(&initial_state));
    let id = coordinator
      .next
      .get()
      .checked_add(1)
      .expect("session identity overflow");
    coordinator.next.set(id);
    let session = Rc::new(GameSession {
      id,
      automatic: Cell::new(true),
      worker: coordinator.worker.clone(),
      app: Rc::downgrade(&coordinator),
      data: RefCell::new(SessionData {
        accepted: initial_state,
        context: Some(context),
        run: None,
        status: GameStatus::Busy,
        initial_submitted: false,
        pending: Some(PendingOutput {
          sequence: 1,
          completion: None,
          animation: None,
        }),
        rendered,
        prompt: None,
        sequence: 1,
        diagnostic: None,
      }),
    });
    *coordinator.current.borrow_mut() = Some(session.clone());
    coordinator.changed();
    GameHandle { session }
  }

  fn game_consumer<G: Game>(&mut self) -> GameConsumer<G> {
    let coordinator = self.application_runtime(Coordinator::default);
    let session = coordinator
      .current
      .borrow()
      .clone()
      .expect("no game is attached");
    let erased: Rc<dyn Any> = session;
    let session = erased
      .downcast::<GameSession<G>>()
      .unwrap_or_else(|_| panic!("attached game type mismatch"));
    session.automatic.set(false);
    GameConsumer { session }
  }
}

impl Coordinator {
  pub(crate) fn changed(&self) {
    self.revision.set(
      self
        .revision
        .get()
        .checked_add(1)
        .expect("game revision overflow"),
    );
  }
}

impl AppRuntime for Coordinator {
  fn work_scope(&self) -> Option<u64> {
    self
      .current
      .borrow()
      .as_ref()
      .filter(|session| session.active())
      .map(|session| session.id())
  }

  fn take_output(&self) -> Option<Box<dyn AppOutput>> {
    self.current.borrow().clone()?.output()
  }

  fn fail_work(&self, scope: u64, message: String) {
    if let Some(session) = self
      .current
      .borrow()
      .clone()
      .filter(|session| session.id() == scope)
    {
      session.fail(message);
    }
  }

  fn context(&self) -> Rc<dyn Any> {
    let view = self
      .current
      .borrow()
      .as_ref()
      .map(|session| session.view(self.revision.get()));
    Rc::new(view)
  }

  fn poll(&self) -> bool {
    let session = self.current.borrow().clone();
    if let Some(session) = &session {
      session.refresh();
    }
    let game = session.map(|session| (session.id(), session.observation()));
    let game_changed = self.observed_game.replace(game.clone()) != game;
    let revision = self.revision.get();
    let revision_changed = self.observed.replace(revision) != revision;
    game_changed || revision_changed
  }

  fn callback(&self, callback: &mut dyn FnMut()) {
    let session = self.current.borrow().clone();
    // Game state is worker-private; callbacks only mutate display-local state.
    // Request validation releases its locks before panic. Catch here, before
    // the component runtime's poison boundary and any exported C ABI return.
    if let Err(payload) = panic::catch_unwind(AssertUnwindSafe(callback)) {
      let Some(session) = session else {
        panic::resume_unwind(payload)
      };
      if let Some(fault) = payload.downcast_ref::<DispatchFault>() {
        if fault.session == session.id() {
          session.fail(fault.message.to_owned());
        }
        return;
      }
      let message = payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "non-string game callback panic".to_owned());
      session.fail(message);
    }
  }

  fn stop(&self) {
    if let Some(session) = self.current.borrow().clone() {
      session.stop();
    }
  }
}

impl<G: Game> AttachedSession for GameSession<G> {
  fn stop(&self) {
    self.stop();
  }
  fn refresh(&self) {
    self.refresh();
  }
  fn fail(&self, message: String) {
    self.fail(message);
  }
  fn id(&self) -> u64 {
    self.id
  }
  fn active(&self) -> bool {
    !matches!(
      self.data.borrow().status,
      GameStatus::Failed | GameStatus::Stopped
    )
  }
  fn output(self: Rc<Self>) -> Option<Box<dyn AppOutput>> {
    if !self.automatic.get() {
      return None;
    }
    GameConsumer { session: self }
      .take_output()
      .map(|output| Box::new(output) as Box<dyn AppOutput>)
  }
  fn view(&self, revision: u64) -> GameRenderContext {
    let data = self.data.borrow();
    let observation = self::game_observation(&data);
    GameRenderContext {
      game: TypeId::of::<G>(),
      id: self.id,
      revision,
      status: data.status,
      state: data.rendered.clone(),
      prompt: data
        .prompt
        .as_ref()
        .map(|prompt| prompt.clone() as Rc<dyn Any>),
      animation_sequence: data.pending.as_ref().map(|pending| pending.sequence),
      animation: data
        .pending
        .as_ref()
        .and_then(|pending| pending.animation.clone())
        .map(|animation| animation as Rc<dyn Any>),
      worker: observation.worker,
      publications: observation.publications,
    }
  }
  fn observation(&self) -> crate::GameObservation {
    self::game_observation(&self.data.borrow())
  }
}

fn game_observation<G: Game>(data: &SessionData<G>) -> crate::GameObservation {
  crate::GameObservation {
    status: data.status,
    worker: data
      .run
      .as_ref()
      .map_or_else(Default::default, |run| run.observation()),
    publications: data
      .run
      .as_ref()
      .map_or_else(Default::default, |run| run.publication_observation()),
  }
}
