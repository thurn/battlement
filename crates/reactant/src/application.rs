//! Component-first application construction and native lifecycle ownership.

use std::{
  rc::Rc,
  time::{Duration, Instant},
};

use battlement::{
  Color, ControllerInputSettings, GameObject, PhysicalKey, SceneAddress, UiDocument,
  UiDocumentState,
};
use battlement_native::{
  ConnectView, Engine, EngineError, EngineResponse, FlatBufferSubmitError, UiEventActionView,
  UiEventResult,
};
use reactant_core::{app::App, hooks, portal::PortalTarget, render::Render};
use trox::{Bundle, Localizer, SourceLocale};

use crate::{
  GameConsumer,
  game_app::{Coordinator, GameApp},
  host_clock::HostClock,
};
use reactant_core::app_runtime::AppRuntime;
use reactant_rules::{Game, RulesWorker};

/// A declarative Reactant application shell.
///
/// Components own changing application and game state. This value only
/// describes native shell capabilities that exist before the first render.
pub struct Application {
  app: App,
  coordinator: Rc<Coordinator>,
}

impl Application {
  /// Creates an application for one content scene.
  pub fn new(scene: impl Into<SceneAddress>) -> Self {
    let mut app = App::new(scene).reset_on_reconnect();
    let coordinator = app.application_runtime(Coordinator::default);
    coordinator.bind(&coordinator);
    let input = coordinator.clone();
    app = app.on_core_action(move |_, body| input.input.dispatch_core(body));
    Self { app, coordinator }
  }

  /// Selects rules execution before any game is mounted; defaults to a real worker.
  pub fn rules_worker(mut self, worker: RulesWorker) -> Self {
    if worker.is_inline() {
      self.app = self.app.spawner(InlineSpawner);
    }
    self.coordinator.set_worker(worker);
    self
  }

  /// Mounts the root component in the primary document.
  pub fn child(mut self, component: impl Render) -> Self {
    self.app = self.app.ui(component);
    self
  }

  /// Customizes the primary document.
  pub fn document(mut self, configure: impl FnOnce(UiDocument) -> UiDocument) -> Self {
    self.app = self.app.document(configure);
    self
  }

  /// Customizes the primary panel.
  pub fn panel(mut self, configure: impl FnOnce(UiDocumentState) -> UiDocumentState) -> Self {
    self.app = self.app.panel(configure);
    self
  }

  /// Adds another application-owned document and component root.
  pub fn additional_document(
    mut self,
    document: UiDocument,
    component: impl Render + Clone + 'static,
  ) -> Self {
    self.app = self
      .app
      .additional_root(document, move |_| component.clone());
    self
  }

  /// Sets the camera's solid background color.
  pub fn background(mut self, color: Color) -> Self {
    self.app = self.app.background(color);
    self
  }

  /// Customizes the application camera.
  pub fn camera(mut self, configure: impl FnOnce(GameObject) -> GameObject) -> Self {
    self.app = self.app.camera(configure);
    self
  }

  /// Adds a static application-owned object.
  pub fn object(mut self, object: GameObject) -> Self {
    self.app = self.app.object(object);
    self
  }

  /// Declares keyboard keys delivered after native focus arbitration.
  pub fn global_keys(mut self, keys: impl IntoIterator<Item = PhysicalKey>) -> Self {
    let keys = keys.into_iter().collect::<Vec<_>>();
    self.coordinator.input_subscriptions.set_keys(keys.clone());
    self.app = self.app.global_keys(keys);
    self
  }

  /// Declares controller navigation and buttons for the application.
  pub fn controller_input(mut self, settings: ControllerInputSettings) -> Self {
    self
      .coordinator
      .input_subscriptions
      .set_controller(settings.clone());
    self.app = self.app.controller_input(settings);
    self
  }

  /// Uses an English source bundle for localization.
  pub fn source_bundle(mut self, source: Bundle) -> Self {
    self.app = self.app.source_bundle(source);
    self
  }

  /// Uses bundle-free source-language localization.
  pub fn source_locale(mut self, source: SourceLocale) -> Self {
    self.app = self.app.source_locale(source);
    self
  }

  /// Uses a complete target/source localizer.
  pub fn localizer(mut self, localizer: Localizer) -> Self {
    self.app = self.app.localizer(localizer);
    self
  }

  fn into_parts(self) -> (App, Rc<Coordinator>) {
    (self.app, self.coordinator)
  }

  fn set_clock(&self, now: Rc<dyn Fn() -> Instant>) {
    self.coordinator.set_clock(now);
  }
}

/// Native adapter instantiated by [`export_application!`] and testing hosts.
#[doc(hidden)]
pub struct ApplicationEngine {
  factory: Box<dyn Fn() -> Application>,
  pending: Option<Application>,
  now: Rc<dyn Fn() -> Instant>,
  host_clock: Option<Rc<HostClock>>,
  app: Option<App>,
  coordinator: Option<Rc<Coordinator>>,
}

impl ApplicationEngine {
  /// Creates an adapter for a zero-argument application factory.
  pub fn new(factory: impl Fn() -> Application + 'static) -> Self {
    Self {
      factory: Box::new(factory),
      pending: None,
      now: Rc::new(Instant::now),
      host_clock: None,
      app: None,
      coordinator: None,
    }
  }

  /// Captures the first application while the native host's fixture environment is active.
  #[doc(hidden)]
  pub fn for_export(factory: impl Fn() -> Application + 'static) -> Self {
    let pending = Some(factory());
    Self {
      factory: Box::new(factory),
      pending,
      now: Rc::new(Instant::now),
      host_clock: Some(Rc::new(HostClock::new())),
      app: None,
      coordinator: None,
    }
  }

  /// Creates a single-mount adapter for low-level tests.
  #[doc(hidden)]
  pub fn once(application: Application) -> Self {
    let application = std::cell::RefCell::new(Some(application));
    Self::new(move || {
      application
        .borrow_mut()
        .take()
        .expect("single-mount application was reconnected")
    })
  }

  /// Creates an adapter with an injected monotonic clock.
  pub fn with_clock(
    factory: impl Fn() -> Application + 'static,
    now: impl Fn() -> Instant + 'static,
  ) -> Self {
    Self {
      factory: Box::new(factory),
      pending: None,
      now: Rc::new(now),
      host_clock: None,
      app: None,
      coordinator: None,
    }
  }

  fn app(&mut self) -> &mut App {
    self.app.as_mut().expect("connect application before use")
  }

  /// Counts admitted actions without exposing game state or discovering work.
  pub fn action_count(&self) -> u64 {
    self
      .coordinator
      .as_ref()
      .expect("connect before input")
      .admitted_actions
      .get()
  }

  /// Reports the configured rules executor without inspecting game state.
  pub fn rules_are_inline(&self) -> bool {
    self
      .coordinator
      .as_ref()
      .expect("connect before input")
      .is_inline()
  }

  /// Takes one already-completed inline publication or a known context change.
  /// No worker discovery, clocks, or polling entry points are involved.
  pub fn take_inline_output(&mut self) -> Option<EngineResponse> {
    let coordinator = self
      .coordinator
      .as_ref()
      .expect("connect before driving output");
    assert!(
      coordinator.is_inline(),
      "explicit output driving requires inline rules"
    );
    assert!(
      self.app.as_ref().unwrap().can_submit_output(),
      "release outstanding responses before taking inline output"
    );
    let output = coordinator.take_output();
    if output.is_none()
      && !coordinator.has_changes()
      && !self.app.as_ref().unwrap().has_ready_changes()
    {
      return None;
    }
    Some(
      self
        .app()
        .submit_ready_output(output)
        .expect("inline output submission failed"),
    )
  }

  /// Applies an explicitly changed external dependency through normal composition.
  pub fn invalidate_inline(&self) {
    let coordinator = self
      .coordinator
      .as_ref()
      .expect("connect before invalidating");
    assert!(
      coordinator.is_inline(),
      "inline dependency changes require inline rules"
    );
    coordinator.changed();
  }

  /// Runs timers at the manual clock's current instant, never advancing that clock.
  pub fn fire_inline_timers(&self) {
    let coordinator = self
      .coordinator
      .as_ref()
      .expect("connect before driving timers");
    assert!(
      coordinator.is_inline(),
      "inline timers require inline rules"
    );
    coordinator.fire_due_timers();
  }

  /// Fails immediately if known inline work ended without accepting its final state.
  pub fn assert_inline_complete(&self) {
    self
      .coordinator
      .as_ref()
      .expect("connect before checking completion")
      .assert_inline_complete();
  }

  /// Returns the active typed game handle for testing and host inspection.
  pub fn game<G: reactant_rules::Game>(&self) -> Option<crate::GameHandle<G>> {
    self.coordinator.as_ref()?.game::<G>()
  }

  /// Takes manual control of publication consumption and output submission.
  pub fn game_consumer<G: Game>(&mut self) -> GameConsumer<G> {
    self.app().game_consumer()
  }

  /// Reports the next application timer deadline for deterministic test hosts.
  #[doc(hidden)]
  pub fn next_timer_due_in(&self) -> Option<std::time::Duration> {
    self.coordinator.as_ref()?.next_timer_due_in()
  }

  /// Waits for the active game's next rules publication.
  pub fn wait_for_game_output<G: reactant_rules::Game>(
    &self,
    timeout: std::time::Duration,
  ) -> bool {
    self
      .coordinator
      .as_ref()
      .is_none_or(|coordinator| coordinator.wait_for_output::<G>(timeout))
  }

  /// Waits for the active game's worker to stop.
  pub fn wait_for_game_worker<G: reactant_rules::Game>(
    &self,
    timeout: std::time::Duration,
  ) -> bool {
    self
      .coordinator
      .as_ref()
      .is_none_or(|coordinator| coordinator.wait_for_worker_stopped::<G>(timeout))
  }

  /// Looks up the committed presentation for a stable component identity.
  pub fn presentation(
    &self,
    id: uuid::Uuid,
  ) -> Option<reactant_core::presentation::PresentationObservation> {
    self.app.as_ref()?.presentation(id)
  }
}

impl Engine for ApplicationEngine {
  const WIRE_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_DIGEST_C;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    let application = self.pending.take().unwrap_or_else(|| (self.factory)());
    if let Some(clock) = self.host_clock.clone() {
      application.set_clock(Rc::new(move || clock.now()));
    } else {
      application.set_clock(self.now.clone());
    }
    let (app, coordinator) = application.into_parts();
    self.app = Some(app);
    self.coordinator = Some(coordinator);
    self.app().connect(message)
  }

  fn submit(&mut self, bytes: &[u8]) -> Result<EngineResponse, FlatBufferSubmitError> {
    self.app().submit(bytes)
  }

  fn submit_ui_event(
    &mut self,
    action: UiEventActionView<'_>,
  ) -> Result<UiEventResult, EngineError> {
    self.app().submit_ui_event(action)
  }

  fn set_time(&mut self, elapsed: Duration) {
    if let Some(clock) = &self.host_clock {
      clock.observe(elapsed);
    }
  }

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
    assert!(
      !self.coordinator.as_ref().is_some_and(|c| c.is_inline()),
      "inline engine must never be polled"
    );
    self.app().poll()
  }
}

/// Allocates a stable portal target owned by the mounted component.
pub fn use_portal_target() -> PortalTarget {
  let allocator = hooks::use_required_context::<reactant_core::portal::PortalAllocator>();
  hooks::use_memo(move || allocator.allocate(), ())
}

/// Exports a zero-argument [`Application`] factory through Reactant's native ABI.
#[macro_export]
macro_rules! export_application {
  ($factory:path $(,)?) => {
    #[doc(hidden)]
    fn __reactant_create_application_engine()
    -> ::core::result::Result<$crate::ApplicationEngine, $crate::__native::EngineError> {
      Ok($crate::ApplicationEngine::for_export($factory))
    }

    $crate::__native::export_deterministic_engine!(
      self::__reactant_create_application_engine,
      clock = virtualized,
      randomness = seeded,
      external_state = isolated,
      persistent_state = reset,
      input = semantic,
      visible_output = flatbuffers,
    );
  };
}

/// Inline scenarios cannot complete asynchronous resources on their calling stack.
struct InlineSpawner;
impl reactant_core::executor::Spawner for InlineSpawner {
  fn spawn(
    &self,
    _: reactant_core::executor::BoxFuture<'static, ()>,
  ) -> reactant_core::executor::SpawnedTask {
    panic!("inline application cannot spawn asynchronous resources; inject a synchronous service")
  }
}
