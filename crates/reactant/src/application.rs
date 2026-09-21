//! Component-first application construction and native lifecycle ownership.

use std::{rc::Rc, time::Instant};

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

use crate::game_app::Coordinator;

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
    app = app.on_core_action(move |_, body| input.dispatch_core(body));
    Self { app, coordinator }
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
    self.app = self.app.global_keys(keys);
    self
  }

  /// Declares controller navigation and buttons for the application.
  pub fn controller_input(mut self, settings: ControllerInputSettings) -> Self {
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
      app: None,
      coordinator: None,
    }
  }

  fn app(&mut self) -> &mut App {
    self.app.as_mut().expect("connect application before use")
  }

  /// Returns the active typed game handle for testing and host inspection.
  pub fn game<G: reactant_rules::Game>(&self) -> Option<crate::GameHandle<G>> {
    self.coordinator.as_ref()?.game::<G>()
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
  const WIRE_CONTRACT_DIGEST_C: &'static [u8; 65] = battlement_native::WIRE_CONTRACT_DIGEST_C;

  fn connect(&mut self, message: ConnectView<'_>) -> Result<EngineResponse, EngineError> {
    let application = self.pending.take().unwrap_or_else(|| (self.factory)());
    application.set_clock(self.now.clone());
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

  fn poll(&mut self) -> Result<Option<EngineResponse>, EngineError> {
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
