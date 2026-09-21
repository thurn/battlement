use std::{
  sync::Arc,
  time::{Duration, Instant},
};

use battlement::{CommandId, Connect, ObjectId, Vector3};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::FakeClient,
  effects::{AudioOccurrence, ParticleOccurrence},
  world::{FakeAudio, FakeObject},
};
use battlement_native::Engine;

/// Stable boundary reached after synchronously driving one game action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GameActionResult {
  /// The action's final publication was accepted and rendered.
  Completed,
  /// The action presented a human-owned prompt and is waiting for a response.
  AwaitingInput,
}

/// A real engine connected to the deterministic in-memory display host.
///
/// Inputs travel through the engine's public submit APIs. Observations come
/// from the fake host after it applies verified engine responses.
pub struct Display<E = reactant::ApplicationEngine>
where
  E: Engine,
{
  pub(crate) client: FakeClient<E>,
}

impl Display<reactant::ApplicationEngine> {
  /// Mounts a fresh component-first application in the deterministic display.
  #[must_use]
  pub fn mount(
    factory: impl Fn() -> reactant::Application + 'static,
    assets: impl Into<Arc<FakeAssetCatalog>>,
  ) -> Self {
    let (client, _) = FakeClient::connect_clocked(
      move |clock| reactant::ApplicationEngine::with_clock(factory, move || clock.now()),
      assets,
    );
    Self { client }
  }

  /// Mounts an application with explicit deterministic platform metadata.
  #[must_use]
  pub fn mount_with(
    factory: impl Fn() -> reactant::Application + 'static,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    connect: Connect,
  ) -> Self {
    let (client, _) = FakeClient::connect_with_clocked(
      move |clock| reactant::ApplicationEngine::with_clock(factory, move || clock.now()),
      assets,
      connect,
    );
    Self { client }
  }

  /// Returns the active typed game's readiness.
  pub fn game_status<G: reactant::rules::Game>(&mut self) -> Option<reactant::GameStatus> {
    self.with_engine(|engine| engine.game::<G>().map(|game| game.status()))
  }

  /// Copies the active typed game's accepted state.
  pub fn game_state<G: reactant::rules::Game>(&mut self) -> Option<G::State> {
    self.with_engine(|engine| engine.game::<G>().map(|game| game.accepted_state()))
  }

  /// Runs one input operation and synchronously presents that game action's result.
  ///
  /// Worker waits use Reactant's notification sidechannel rather than sleeping
  /// or advancing virtual presentation time. The result identifies the action's
  /// completion or its human-owned prompt, not any subsequent automatic action.
  pub fn game_action<G: reactant::rules::Game>(
    &mut self,
    timeout: Duration,
    operation: impl FnOnce(&mut Self),
  ) -> GameActionResult {
    let completed = self
      .with_engine(|engine| engine.game::<G>().map(|game| game.completed_actions()))
      .expect("no game of the requested type is attached");
    operation(self);
    self.drive_game::<G>(timeout, Some(completed))
  }

  /// Runs a game action and advances presentation until its observable result appears.
  pub fn game_action_presented<G: reactant::rules::Game>(
    &mut self,
    timeout: Duration,
    operation: impl FnOnce(&mut Self),
    presented: impl Fn(&Self) -> bool,
  ) -> GameActionResult {
    let result = self.game_action::<G>(timeout, operation);
    self.until_presented(presented);
    result
  }

  /// Synchronously presents game work until it is ready or asks for human input.
  pub fn settle_game<G: reactant::rules::Game>(&mut self, timeout: Duration) -> GameActionResult {
    self.flush();
    self.drive_game::<G>(timeout, None)
  }

  /// Drives scheduled application timers until an observable result appears.
  ///
  /// Finite presentation work can be completed separately with [`Self::settle`].
  pub fn until_timer(&mut self, observed: impl Fn(&Self) -> bool) {
    self.flush();
    for _ in 0..1_000 {
      if observed(self) {
        return;
      }
      let delay = self
        .with_engine(|engine| engine.next_timer_due_in())
        .expect("observable result was not reached and no application timer remains");
      self.client.advance_time(delay);
      self.flush();
    }
    assert!(
      observed(self),
      "observable result was not reached after 1,000 application timer events"
    );
  }

  /// Waits for one typed rules publication without advancing presentation time.
  pub fn wait_for_game_output<G: reactant::rules::Game>(&mut self, timeout: Duration) -> bool {
    self.with_engine(|engine| engine.wait_for_game_output::<G>(timeout))
  }

  /// Waits for typed rules worker cleanup.
  pub fn wait_for_game_worker<G: reactant::rules::Game>(&mut self, timeout: Duration) -> bool {
    self.with_engine(|engine| engine.wait_for_game_worker::<G>(timeout))
  }

  /// Looks up a committed Reactant presentation identity.
  pub fn presentation(
    &mut self,
    id: uuid::Uuid,
  ) -> Option<reactant::presentation::PresentationObservation> {
    self.with_engine(|engine| engine.presentation(id))
  }

  fn drive_game<G: reactant::rules::Game>(
    &mut self,
    timeout: Duration,
    completed_before: Option<u64>,
  ) -> GameActionResult {
    let deadline = Instant::now() + timeout;
    loop {
      let (status, completed, waiting, diagnostic) = self.with_engine(|engine| {
        let game = engine
          .game::<G>()
          .expect("no game of the requested type is attached");
        (
          game.status(),
          game.completed_actions(),
          game.waiting_for_input(),
          game.diagnostic(),
        )
      });
      if completed_before.is_some_and(|before| completed > before) {
        self.flush();
        return GameActionResult::Completed;
      }
      if waiting {
        return GameActionResult::AwaitingInput;
      }
      match status {
        reactant::GameStatus::Ready if completed_before.is_none() => {
          return GameActionResult::Completed;
        }
        reactant::GameStatus::Failed => {
          panic!(
            "game failed while synchronizing: {}",
            diagnostic.as_deref().unwrap_or("no diagnostic")
          )
        }
        reactant::GameStatus::Stopped => panic!("game stopped while synchronizing"),
        reactant::GameStatus::Ready | reactant::GameStatus::Busy => {}
      }
      let remaining = deadline.saturating_duration_since(Instant::now());
      assert!(
        !remaining.is_zero(),
        "game synchronization timed out after {timeout:?}"
      );
      assert!(
        self.wait_for_game_output::<G>(remaining),
        "game synchronization timed out after {timeout:?}"
      );
      self.client.poll();
    }
  }
}

impl<E> Display<E>
where
  E: Engine,
{
  /// Connects an engine with deterministic fake platform metadata.
  #[must_use]
  pub fn connect(engine: E, assets: impl Into<Arc<FakeAssetCatalog>>) -> Self {
    Self {
      client: FakeClient::connect(engine, assets),
    }
  }

  /// Connects an engine with explicit deterministic platform metadata.
  #[must_use]
  pub fn connect_with(
    engine: E,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    connect: Connect,
  ) -> Self {
    Self {
      client: FakeClient::connect_with(engine, assets, connect),
    }
  }

  /// Connects an engine factory to the display's deterministic manual clock.
  #[must_use]
  pub fn connect_with_clocked(
    make_engine: impl FnOnce(battlement_fake::time::ManualClock) -> E,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    connect: Connect,
  ) -> Self {
    let (client, _) = FakeClient::connect_with_clocked(make_engine, assets, connect);
    let mut display = Self { client };
    display.flush();
    display
  }

  /// Runs a public app operation without polling, advancing time, or a frame.
  pub fn with_engine<R>(&mut self, operation: impl FnOnce(&mut E) -> R) -> R {
    operation(self.client.engine_mut())
  }

  /// Replaces the engine session without advancing time or rendering a frame.
  pub fn reconnect(&mut self) {
    self.client.reconnect();
  }

  /// Applies at most one response made available by the engine.
  ///
  /// Polling may run engine-owned main-thread work, but it does not advance
  /// presentation time or record a rendered frame.
  pub fn poll(&mut self) {
    self.client.poll();
  }

  /// Applies all engine work already available without waiting or advancing time.
  pub fn flush(&mut self) {
    while self.client.poll_available() {}
  }

  /// Presses a geometric primary pointer in upper-left screen pixels; advances no clock or frame.
  pub fn pointer_down(
    &mut self,
    pointer_id: i32,
    position: battlement::PanelPoint,
  ) -> battlement::UiEventDisposition {
    self.client.sample_pointer(pointer_id, position, true)
  }
  /// Moves a geometric pointer with its primary button state; advances no clock or frame.
  pub fn pointer_move(
    &mut self,
    pointer_id: i32,
    position: battlement::PanelPoint,
    pressed: bool,
  ) -> battlement::UiEventDisposition {
    self.client.sample_pointer(pointer_id, position, pressed)
  }
  /// Releases a geometric primary pointer; advances no clock or frame.
  pub fn pointer_up(
    &mut self,
    pointer_id: i32,
    position: battlement::PanelPoint,
  ) -> battlement::UiEventDisposition {
    self.client.sample_pointer(pointer_id, position, false)
  }
  /// Clicks the eligible target after UI/modal/world geometric arbitration.
  pub fn click_at(&mut self, position: battlement::PanelPoint) {
    self.pointer_down(0, position);
    self.pointer_up(0, position);
  }
  /// Starts the native draggable-object path without resolving screen geometry.
  pub fn drag_start(&mut self, object_id: ObjectId, input: battlement_fake::client::PointerInput) {
    self.client.drag_start(object_id, input);
  }
  /// Ends the native draggable-object path at one world-space position.
  pub fn drag_end(
    &mut self,
    object_id: ObjectId,
    input: battlement_fake::client::PointerInput,
    world_position: Vector3,
  ) {
    self.client.drag_end(object_id, input, world_position);
  }
  /// Observes the current pointer capture owner.
  pub fn pointer_capture(&self, pointer_id: i32) -> Option<ObjectId> {
    self.client.geometric_capture(pointer_id)
  }
  /// Observes host capture loss, including loss after logical destruction.
  pub fn capture_losses(&self) -> &[(i32, ObjectId)] {
    self.client.capture_losses()
  }

  /// Observes current semantic focus without advancing time or frames.
  pub fn focused(&self) -> Option<ObjectId> {
    self.client.focused()
  }
  /// Moves focus using current displayed geometry, without synthesizing a pointer.
  pub fn navigate(&mut self, direction: battlement::NavigationDirection) {
    self.client.navigate(direction);
  }
  /// Activates the current eligible focus owner; advances no time or frame.
  pub fn activate_focused(&mut self) {
    self.client.activate_focused();
  }

  /// Sends one enabled physical-key transition through the public engine action path.
  pub fn key_down(&mut self, key: battlement::PhysicalKey) {
    self.client.key_down(key);
  }

  /// Releases one enabled physical key through the public engine action path.
  pub fn key_up(&mut self, key: battlement::PhysicalKey) {
    self.client.key_up(key);
  }

  /// Sends one configured controller-button press through the public engine action path.
  pub fn controller_button_down(
    &mut self,
    controller_id: i32,
    button: battlement::ControllerButton,
  ) {
    self.client.controller_button_down(controller_id, button);
  }

  /// Releases one configured controller button through the public engine action path.
  pub fn controller_button_up(&mut self, controller_id: i32, button: battlement::ControllerButton) {
    self.client.controller_button_up(controller_id, button);
  }

  /// Sends one configured discrete controller-navigation action.
  pub fn controller_navigate(
    &mut self,
    controller_id: i32,
    direction: battlement::ControllerDirection,
  ) {
    self.client.controller_navigate(
      controller_id,
      direction,
      battlement::ControllerNavigationSource::Dpad,
      false,
    );
  }
  /// Cancels through the focused logical route, including a modal's dismiss behavior.
  pub fn cancel_navigation(&mut self) {
    self.client.cancel_navigation();
  }

  /// Activates a world object through the same coordinate-free route as native Ditto.
  pub fn activate(&mut self, object_id: ObjectId) {
    self.client.activate(object_id);
  }

  /// Performs a semantic primary-pointer click on a world object.
  pub fn click(&mut self, object_id: ObjectId) {
    self.client.click(object_id);
  }

  /// Performs a native-style click on a UI button.
  pub fn click_ui(&mut self, object_id: ObjectId) {
    self.client.ui().click(object_id);
  }

  /// Activates a UI button through the shared keyboard/controller submit route.
  pub fn navigation_submit_ui(&mut self, object_id: ObjectId) {
    self.client.ui().navigation_submit(object_id);
  }

  /// Delivers a previously captured native input without re-resolving its target.
  /// Removed targets are ignored by the engine's current event routing.
  pub fn deliver_ui_event(&mut self, event: battlement::UiEvent) {
    self.client.ui().deliver_event(event);
  }

  /// Opens the fake UI surface for detailed element assertions.
  #[must_use]
  pub fn ui(&mut self) -> battlement_fake::client::ui::UiClient<'_, E> {
    self.client.ui()
  }

  /// Opens the fake world surface for detailed host assertions.
  #[must_use]
  pub fn world(&self) -> &battlement_fake::world::FakeWorld {
    self.client.world()
  }

  /// Delivers native Motion lifecycle observations without advancing host time.
  pub fn deliver_motion_events(&mut self, events: battlement::MotionEventBatch) {
    self.client.submit_motion(events);
  }

  /// Returns the active native geometry requests without advancing work or frames.
  #[must_use]
  pub fn geometry_registry(&self) -> &battlement::GeometryRegistry {
    self.client.geometry_registry()
  }

  /// Delivers one coherent native geometry sample without advancing time or frames.
  pub fn deliver_geometry(&mut self, batch: battlement::GeometryObservationBatch) {
    self.client.submit_geometry(batch);
  }

  /// Returns native commands completed since connection or the last journal clear.
  #[must_use]
  pub fn commands(&self) -> &[battlement_fake::journal::ExecutedCommand] {
    self.client.commands()
  }

  /// Clears retained command evidence without changing displayed state.
  pub fn clear_commands(&mut self) {
    self.client.clear_commands();
  }

  /// Finds a live UI descendant by its authored name.
  #[must_use]
  pub fn find_ui(&self, root: ObjectId, name: &str) -> ObjectId {
    let ui = self.client.ui_world();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
      let element = ui
        .element(id)
        .unwrap_or_else(|| panic!("UI element does not exist: {id}"));
      if element.name() == Some(name) {
        return id;
      }
      pending.extend(element.children());
    }
    panic!("missing UI element named {name}");
  }

  /// Reports whether a native UI element remains presented, including retained exits.
  #[must_use]
  pub fn contains_ui(&self, object_id: ObjectId) -> bool {
    self.client.ui_world().element(object_id).is_some()
  }

  /// Cancels a geometric gesture without clicking or advancing time.
  pub fn pointer_cancel(&mut self, pointer_id: i32) {
    self.client.cancel_pointer(pointer_id);
  }

  /// Returns one live UI element for visible-state inspection.
  #[must_use]
  pub fn ui_element(
    &self,
    object_id: ObjectId,
  ) -> &battlement_fake::battlement_ui_fake::UiElementState {
    self
      .client
      .ui_world()
      .element(object_id)
      .unwrap_or_else(|| panic!("UI element does not exist: {object_id}"))
  }

  /// Returns the currently presented accessibility tree.
  #[must_use]
  pub fn accessibility(&self) -> &battlement::AccessibilitySnapshot {
    self.client.accessibility()
  }

  /// Returns one world object when it is currently presented.
  #[must_use]
  pub fn object(&self, object_id: ObjectId) -> Option<&FakeObject> {
    self.client.world().object(object_id)
  }

  /// Iterates over the currently displayed world objects in presentation order.
  pub fn objects(&self) -> impl Iterator<Item = &FakeObject> {
    self.client.world().objects()
  }

  /// Iterates over currently displayed world images and their visible state.
  pub fn images(&self) -> impl Iterator<Item = (&FakeObject, &battlement::ImageState)> {
    self.client.world().images()
  }

  /// Iterates over currently displayed world text and its visible state.
  pub fn texts(&self) -> impl Iterator<Item = (&FakeObject, &battlement::TextState)> {
    self.client.world().texts()
  }

  /// Returns one live audio playback when it is currently presented.
  #[must_use]
  pub fn audio(&self, command_id: CommandId) -> Option<&FakeAudio> {
    self.client.world().audio(command_id)
  }

  /// Returns enabled session-wide physical keys from the accepted snapshot.
  #[must_use]
  pub fn global_keys(&self) -> &[battlement::PhysicalKey] {
    self.client.world().global_keys()
  }

  /// Returns controller input settings from the accepted snapshot.
  #[must_use]
  pub fn controller_input(&self) -> Option<&battlement::ControllerInputSettings> {
    self.client.world().controller_input()
  }

  /// Returns the deterministic Diagnostics module exposed by the fake host.
  #[must_use]
  pub fn diagnostics(&self) -> &battlement_cloud_fake::diagnostics::DiagnosticsFake {
    self.client.diagnostics()
  }

  /// Observes a presented local point without advancing time, work, or frames.
  #[must_use]
  pub fn world_point(&self, object_id: ObjectId, offset: Vector3) -> Vector3 {
    self.client.world().world_point(object_id, offset)
  }

  /// Records one rendered-frame boundary without advancing virtual time.
  pub fn advance_frame(&mut self) {
    self.client.advance_frame();
  }

  /// Advances all finite presentation work, including later automatic actions.
  /// Use [`Self::until_presented`] to stop at a particular visible result.
  pub fn settle(&mut self) {
    self.client.settle();
  }

  /// Advances scheduled presentation events until the requested state is visible.
  pub fn until_presented(&mut self, observed: impl Fn(&Self) -> bool) {
    self.flush();
    for _ in 0..10_000 {
      if observed(self) {
        return;
      }
      assert!(
        self.client.advance_to_next_presentation_event(),
        "observable result was not reached and no finite presentation event remains"
      );
    }
    assert!(
      observed(self),
      "observable result was not reached after 10,000 presentation events"
    );
  }

  /// Returns elapsed virtual presentation time.
  #[must_use]
  pub fn presentation_time(&self) -> Duration {
    self.client.presentation_time()
  }

  /// Returns the number of explicitly recorded rendered frames.
  #[must_use]
  pub fn frame(&self) -> u64 {
    self.client.frame()
  }

  /// Returns audio play occurrences in execution order.
  #[must_use]
  pub fn audio_occurrences(&self) -> &[AudioOccurrence] {
    self.client.audio_occurrences()
  }

  /// Returns temporary particle occurrences in execution order.
  #[must_use]
  pub fn particle_occurrences(&self) -> &[ParticleOccurrence] {
    self.client.particle_occurrences()
  }
}
