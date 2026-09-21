use std::{
  any::{Any, TypeId},
  cell::{Cell, RefCell},
  collections::HashMap,
  panic::{self, AssertUnwindSafe},
  rc::{Rc, Weak},
  time::{Duration, Instant},
};

use battlement::{ObjectId, Vector3};
use battlement_native::CoreActionBodyView;
use reactant_core::{
  app::App,
  app_runtime::{AppOutput, AppRuntime},
  hooks,
};
use reactant_rules::{DisplayConnection, Game, RulesContext, RulesWorker};

use crate::{
  game_hooks::GameRenderContext,
  game_output::{GameConsumer, PendingOutput},
  game_session::{DispatchFault, GameHandle, GameSession, GameStatus, SessionData},
  input::{DragCallbacks, GlobalInput},
};

type GlobalInputHandler = Rc<dyn Fn(GlobalInput)>;

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

/// Mounts one game session for the lifetime of the calling component.
///
/// Changing `session_key` replaces the session after the new tree commits.
pub fn use_game<G, K>(
  session_key: K,
  initial_state: G::State,
  make_context: impl FnOnce(DisplayConnection<G>) -> G::Context + 'static,
) -> GameHandle<G>
where
  G: Game,
  K: hooks::Dependencies,
{
  let services = hooks::use_required_context::<reactant_core::app_runtime::ApplicationContext>()
    .value::<ServicesContext>()
    .expect("use_game requires a Reactant Application root");
  let coordinator = services
    .coordinator
    .upgrade()
    .expect("application runtime ended while rendering");
  let create = coordinator.clone();
  let handle = hooks::use_memo(
    move || create.create_game(initial_state, make_context, false),
    session_key,
  );
  let mounted = handle.clone();
  let owner = coordinator.clone();
  let session_id = handle.session.id;
  hooks::use_effect(
    move || {
      owner.attach(&mounted);
      move || owner.detach(&mounted)
    },
    session_id,
  );
  handle
}

pub(crate) struct Coordinator {
  current: RefCell<Option<Rc<dyn AttachedSession>>>,
  self_reference: RefCell<Weak<Coordinator>>,
  worker: RulesWorker,
  next: Cell<u64>,
  revision: Cell<u64>,
  observed: Cell<u64>,
  observed_game: RefCell<Option<(u64, crate::GameObservation)>>,
  global_input: RefCell<HashMap<String, GlobalInputHandler>>,
  drag: RefCell<HashMap<ObjectId, (u64, DragCallbacks)>>,
  next_input: Cell<u64>,
  now: RefCell<Rc<dyn Fn() -> Instant>>,
  timers: RefCell<HashMap<String, Timer>>,
}

struct Timer {
  due: Instant,
  interval: Option<Duration>,
  callback: Rc<dyn Fn()>,
}

impl Default for Coordinator {
  fn default() -> Self {
    Self {
      current: RefCell::default(),
      self_reference: RefCell::default(),
      worker: RulesWorker::default(),
      next: Cell::default(),
      revision: Cell::default(),
      observed: Cell::default(),
      observed_game: RefCell::default(),
      global_input: RefCell::default(),
      drag: RefCell::default(),
      next_input: Cell::default(),
      now: RefCell::new(Rc::new(Instant::now)),
      timers: RefCell::default(),
    }
  }
}

#[derive(Clone)]
pub(crate) struct ServicesContext {
  pub(crate) coordinator: Weak<Coordinator>,
  pub(crate) game: Option<GameRenderContext>,
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
    coordinator.bind(&coordinator);
    coordinator.create_game(initial_state, make_context, true)
  }

  fn game_consumer<G: Game>(&mut self) -> GameConsumer<G> {
    let coordinator = self.application_runtime(Coordinator::default);
    coordinator.bind(&coordinator);
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
  pub(crate) fn bind(&self, owner: &Rc<Self>) {
    if self.self_reference.borrow().upgrade().is_none() {
      *self.self_reference.borrow_mut() = Rc::downgrade(owner);
    }
  }

  pub(crate) fn game<G: Game>(&self) -> Option<GameHandle<G>> {
    let session = self.current.borrow().clone()?;
    let erased: Rc<dyn Any> = session;
    erased
      .downcast::<GameSession<G>>()
      .ok()
      .map(|session| GameHandle { session })
  }

  pub(crate) fn wait_for_output<G: Game>(&self, timeout: Duration) -> bool {
    self.game::<G>().is_none_or(|game| {
      GameConsumer {
        session: game.session,
      }
      .wait_for_output(timeout)
    })
  }

  pub(crate) fn wait_for_worker_stopped<G: Game>(&self, timeout: Duration) -> bool {
    self.game::<G>().is_none_or(|game| {
      GameConsumer {
        session: game.session,
      }
      .wait_for_worker_stopped(timeout)
    })
  }

  pub(crate) fn create_game<G: Game>(
    &self,
    initial_state: G::State,
    make_context: impl FnOnce(DisplayConnection<G>) -> G::Context,
    attach: bool,
  ) -> GameHandle<G> {
    if attach && let Some(previous) = self.current.borrow_mut().take() {
      previous.stop();
    }
    let context = RulesContext::new(make_context);
    let rendered = Rc::new(G::logical_clone(&initial_state));
    let id = self
      .next
      .get()
      .checked_add(1)
      .expect("session identity overflow");
    self.next.set(id);
    let session = Rc::new(GameSession {
      id,
      automatic: Cell::new(true),
      worker: self.worker.clone(),
      app: self.self_reference.borrow().clone(),
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
        completed_actions: 0,
        diagnostic: None,
      }),
    });
    if attach {
      *self.current.borrow_mut() = Some(session.clone());
      self.changed();
    }
    GameHandle { session }
  }

  pub(crate) fn attach<G: Game>(&self, handle: &GameHandle<G>) {
    let next: Rc<dyn AttachedSession> = handle.session.clone();
    let mut current = self.current.borrow_mut();
    if current
      .as_ref()
      .is_some_and(|session| session.id() == next.id())
    {
      return;
    }
    if let Some(previous) = current.replace(next) {
      previous.stop();
    }
    drop(current);
    self.changed();
  }

  pub(crate) fn detach<G: Game>(&self, handle: &GameHandle<G>) {
    let mut current = self.current.borrow_mut();
    if current
      .as_ref()
      .is_some_and(|session| session.id() == handle.session.id)
    {
      let session = current.take().expect("checked current session");
      session.stop();
      drop(current);
      self.changed();
    }
  }

  pub(crate) fn changed(&self) {
    self.revision.set(
      self
        .revision
        .get()
        .checked_add(1)
        .expect("game revision overflow"),
    );
  }

  pub(crate) fn register_global_input(&self, identity: String, handler: GlobalInputHandler) {
    self.global_input.borrow_mut().insert(identity, handler);
  }

  pub(crate) fn unregister_global_input(&self, identity: &str) {
    self.global_input.borrow_mut().remove(identity);
  }

  pub(crate) fn register_drag(&self, object: ObjectId, callbacks: DragCallbacks) -> u64 {
    let token = self
      .next_input
      .get()
      .checked_add(1)
      .expect("input identity overflow");
    self.next_input.set(token);
    self.drag.borrow_mut().insert(object, (token, callbacks));
    token
  }

  pub(crate) fn unregister_drag(&self, object: ObjectId, token: u64) {
    let mut drag = self.drag.borrow_mut();
    if drag
      .get(&object)
      .is_some_and(|registered| registered.0 == token)
    {
      drag.remove(&object);
    }
  }

  pub(crate) fn dispatch_core(&self, body: CoreActionBodyView<'_>) {
    let global = match body {
      CoreActionBodyView::KeyDown(value) => Some(GlobalInput::KeyDown(value.physical_key())),
      CoreActionBodyView::KeyUp(value) => Some(GlobalInput::KeyUp(value.physical_key())),
      CoreActionBodyView::ControllerButtonDown(value) => {
        Some(GlobalInput::ControllerButtonDown(value.controller_button()))
      }
      CoreActionBodyView::ControllerButtonUp(value) => {
        Some(GlobalInput::ControllerButtonUp(value.controller_button()))
      }
      CoreActionBodyView::ControllerNavigate(value) => Some(GlobalInput::ControllerNavigate(
        value.controller_direction(),
      )),
      CoreActionBodyView::DragStart(value) => {
        let object = ObjectId::from_bytes(value.object_id()).expect("validated drag object");
        let callback = self
          .drag
          .borrow()
          .get(&object)
          .and_then(|entry| entry.1.start.clone());
        if let Some(callback) = callback {
          callback();
        }
        None
      }
      CoreActionBodyView::DragEnd(value) => {
        let object = ObjectId::from_bytes(value.object_id()).expect("validated drag object");
        let callback = self
          .drag
          .borrow()
          .get(&object)
          .and_then(|entry| entry.1.end.clone());
        if let Some(callback) = callback {
          let [x, y, z] = value.world_position();
          callback(Vector3::new(x, y, z));
        }
        None
      }
      _ => None,
    };
    if let Some(input) = global {
      let handlers = self
        .global_input
        .borrow()
        .values()
        .cloned()
        .collect::<Vec<_>>();
      for handler in handlers {
        handler(input);
      }
    }
  }

  pub(crate) fn set_clock(&self, now: Rc<dyn Fn() -> Instant>) {
    *self.now.borrow_mut() = now;
  }

  pub(crate) fn register_timer(
    &self,
    identity: String,
    delay: Duration,
    interval: Option<Duration>,
    callback: Rc<dyn Fn()>,
  ) {
    let due = (self.now.borrow())()
      .checked_add(delay)
      .expect("Reactant timer deadline overflow");
    self.timers.borrow_mut().insert(
      identity,
      Timer {
        due,
        interval,
        callback,
      },
    );
  }

  pub(crate) fn unregister_timer(&self, identity: &str) {
    self.timers.borrow_mut().remove(identity);
  }

  pub(crate) fn next_timer_due_in(&self) -> Option<Duration> {
    let now = (self.now.borrow())();
    self
      .timers
      .borrow()
      .values()
      .map(|timer| timer.due.saturating_duration_since(now))
      .min()
  }

  fn poll_timers(&self) -> bool {
    let now = (self.now.borrow())();
    let due = self
      .timers
      .borrow()
      .iter()
      .filter(|(_, timer)| timer.due <= now)
      .map(|(identity, _)| identity.clone())
      .collect::<Vec<_>>();
    for identity in &due {
      let callback = {
        let mut timers = self.timers.borrow_mut();
        let Some(timer) = timers.get_mut(identity) else {
          continue;
        };
        let callback = timer.callback.clone();
        if let Some(interval) = timer.interval {
          timer.due = now
            .checked_add(interval)
            .expect("Reactant interval deadline overflow");
        } else {
          timers.remove(identity);
        }
        callback
      };
      callback();
    }
    !due.is_empty()
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
    let game = self
      .current
      .borrow()
      .as_ref()
      .map(|session| session.view(self.revision.get()));
    Rc::new(ServicesContext {
      coordinator: self.self_reference.borrow().clone(),
      game,
    })
  }

  fn poll(&self) -> bool {
    let timer_changed = self.poll_timers();
    let session = self.current.borrow().clone();
    if let Some(session) = &session {
      session.refresh();
    }
    let game = session.map(|session| (session.id(), session.observation()));
    let game_changed = self.observed_game.replace(game.clone()) != game;
    let revision = self.revision.get();
    let revision_changed = self.observed.replace(revision) != revision;
    timer_changed || game_changed || revision_changed
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
    self.timers.borrow_mut().clear();
    self.global_input.borrow_mut().clear();
    self.drag.borrow_mut().clear();
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
