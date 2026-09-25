//! Synchronous fake client lifecycle, responses, input, and assertions.

use battlement::application::ApplicationState;
use battlement::host_settings::HostSettings;
use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
  time::Duration,
};

use battlement::{
  Action, ActionBody, ActionId, ActivationPayload, Batch, BatchFailed, BatchId, Command, CommandId,
  Connect, ControllerButton, ControllerButtonPayload, ControllerDirection,
  ControllerNavigationPayload, ControllerNavigationSource, CoreErrorCode, DragPayload,
  GeometryObservationBatch, GeometryRegistry, ImageState, InputCaptureCancellation,
  InputCaptureDevice, InputCaptureResult, PhysicalKey, PointerButton, PointerButtonPayload,
  PointerEvent, PointerPayload, Response, ResponseMessage, ScreenPosition, ScreenSize, UiEvent,
  UiEventAction, UiEventDisposition, UiVisualElementProperties, Validate, Vector3,
};
use battlement_cloud_fake::diagnostics::DiagnosticsFake;
use battlement_native::Engine;
use battlement_ui_fake::UiWorld;
use uuid::Uuid;

use crate::{
  assertions,
  assets::FakeAssetCatalog,
  client::input_capture::Capture,
  client::ui::{
    MinMaxSliderInteraction, ScrollInteraction, ScrollerInteraction, SliderIntInteraction,
    TextFieldInteraction, UiClient,
  },
  effects::{AudioOccurrence, ParticleOccurrence},
  journal::{CommandCheckpoint, ExecutedCommand},
  operation::ScheduledOperation,
  presentation::ScheduledBatch,
  time::ManualClock,
  world::FakeWorld,
};

mod input_capture;
mod navigation;
mod pointer;
mod pointer_legacy;
pub mod ui;

/// Semantic pointer data used by the fake's lower-level pointer helpers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointerInput {
  /// Physical pointer identity; zero is the mouse pointer.
  pub pointer_id: i32,
  /// Screen-space position in physical pixels.
  pub screen_position: ScreenPosition,
  /// World-space hit point.
  pub world_hit: Vector3,
  /// Mouse-style button.
  pub button: PointerButton,
}

#[derive(Clone, Copy)]
struct PointerState {
  object_id: battlement::ObjectId,
  input: PointerInput,
}

#[derive(Clone, Copy)]
struct PressedPointer {
  object_id: battlement::ObjectId,
  pointer_id: i32,
  button: PointerButton,
}

#[derive(Clone, Copy)]
struct ActiveDrag {
  object_id: battlement::ObjectId,
  pointer_id: i32,
}

/// An in-memory Battlement client driven by a typed rules engine.
pub struct FakeClient<E>
where
  E: Engine,
{
  pub(crate) engine: E,
  pub(crate) assets: Arc<FakeAssetCatalog>,
  pub(crate) connect: Connect,
  pub(crate) diagnostics: DiagnosticsFake,
  pub(crate) session_id: battlement::SessionId,
  pub(crate) world: FakeWorld,
  pub(crate) ui_world: UiWorld,
  pub(crate) motion: crate::motion::MotionWorld,
  pub(crate) accessibility: battlement::AccessibilitySnapshot,
  pub(crate) geometry_registry: GeometryRegistry,
  pub(crate) admitted_batches: HashSet<BatchId>,
  pub(crate) executed_commands: HashSet<CommandId>,
  pub(crate) scheduled_batches: Vec<ScheduledBatch>,
  pub(crate) canceled_scopes: HashSet<u64>,
  pub(crate) paused_scopes: HashMap<u64, HashSet<battlement::ObjectId>>,
  pub(crate) work_objects: HashMap<battlement::ObjectId, (u64, bool)>,
  pub(crate) response_retention: Option<battlement_native::ResponseLease>,
  pub(crate) operations: Vec<ScheduledOperation>,
  pub(crate) presentation_ms: u64,
  pub(crate) presentation_advancing: bool,
  frame: u64,
  pub(crate) audio_occurrences: Vec<AudioOccurrence>,
  pub(crate) particle_occurrences: Vec<ParticleOccurrence>,
  pub(crate) next_action_number: u128,
  pub(crate) pointers: pointer::Pointers,
  navigation: navigation::Navigation,
  hovered: Option<PointerState>,
  pressed: Option<PressedPointer>,
  drag: Option<ActiveDrag>,
  pub(crate) capture: Capture,
  held_navigation: Option<ControllerNavigationPayload>,
  held_keys: HashSet<PhysicalKey>,
  held_controller_buttons: HashSet<(i32, ControllerButton)>,
  pub(crate) clock: Option<ManualClock>,
  scroll_interactions: HashMap<battlement::ObjectId, ScrollInteraction>,
  scroller_interactions: HashMap<battlement::ObjectId, ScrollerInteraction>,
  slider_interactions: HashMap<battlement::ObjectId, ScrollerInteraction>,
  slider_int_interactions: HashMap<battlement::ObjectId, SliderIntInteraction>,
  min_max_slider_interactions: HashMap<battlement::ObjectId, MinMaxSliderInteraction>,
  text_field_interactions: HashMap<battlement::ObjectId, TextFieldInteraction>,
  ui_link_identities: HashMap<(battlement::ObjectId, i32), (String, String)>,
  pub(crate) journal: Vec<ExecutedCommand>,
}

impl<E> FakeClient<E>
where
  E: Engine,
{
  /// Publishes application focus and suspension without advancing time.
  pub fn set_application_state(&mut self, state: ApplicationState) {
    self.connect.application_state = state;
    if !state.focused || state.paused {
      self.held_keys.clear();
      self.held_controller_buttons.clear();
      self.held_navigation = None;
      self.complete_capture(InputCaptureResult::Cancelled(
        InputCaptureCancellation::FocusLost,
      ));
    }
    self.submit_action(ActionBody::ApplicationStateChanged(state));
  }

  /// Publishes an explicit host observation and retains it for reconnects.
  pub fn set_host_settings(&mut self, settings: HostSettings) {
    let keyboard_removed =
      self.connect.host_settings.keyboard_connected && !settings.keyboard_connected;
    let controllers_removed =
      self.connect.host_settings.controller_count > 0 && settings.controller_count == 0;
    self.connect.host_settings = settings.clone();
    if keyboard_removed {
      self.remove_input_device(InputCaptureDevice::Keyboard);
    }
    if controllers_removed {
      self.remove_input_device(InputCaptureDevice::Controller);
    }
    self.submit_action(ActionBody::HostSettingsChanged(settings));
  }

  /// Connects an engine with deterministic fake platform metadata.
  #[must_use]
  pub fn connect(engine: E, assets: impl Into<Arc<FakeAssetCatalog>>) -> Self {
    Self::connect_with_metadata(
      engine,
      assets.into(),
      Connect::new(
        "battlement-fake",
        "battlement-fake",
        ScreenSize {
          width: 1_920,
          height: 1_080,
        },
      ),
      DiagnosticsFake::absent(),
    )
  }

  /// Connects an engine with an explicit deterministic Diagnostics fake.
  #[must_use]
  pub fn connect_with_diagnostics(
    engine: E,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    diagnostics: DiagnosticsFake,
  ) -> Self {
    Self::connect_with_metadata(
      engine,
      assets.into(),
      Connect::new(
        "battlement-fake",
        "battlement-fake",
        ScreenSize {
          width: 1_920,
          height: 1_080,
        },
      ),
      diagnostics,
    )
  }

  /// Connects an engine factory to a manually controlled clock.
  #[must_use]
  pub fn connect_clocked(
    make_engine: impl FnOnce(ManualClock) -> E,
    assets: impl Into<Arc<FakeAssetCatalog>>,
  ) -> (Self, ManualClock) {
    let clock = ManualClock::new(std::time::Instant::now());
    let mut client = Self::connect(make_engine(clock.clone()), assets);
    client.clock = Some(clock.clone());
    (client, clock)
  }

  /// Connects an engine factory and explicit metadata to a manually controlled clock.
  #[must_use]
  pub fn connect_with_clocked(
    make_engine: impl FnOnce(ManualClock) -> E,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    connect: Connect,
  ) -> (Self, ManualClock) {
    let clock = ManualClock::new(std::time::Instant::now());
    let mut client = Self::connect_with(make_engine(clock.clone()), assets, connect);
    client.clock = Some(clock.clone());
    (client, clock)
  }

  /// Connects an engine with explicit connection metadata.
  #[must_use]
  pub fn connect_with(
    engine: E,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    connect: Connect,
  ) -> Self {
    let diagnostics = if connect
      .modules
      .iter()
      .any(|module| module == "battlement.diagnostics")
    {
      DiagnosticsFake::default()
    } else {
      DiagnosticsFake::absent()
    };
    Self::connect_with_metadata(engine, assets.into(), connect, diagnostics)
  }

  fn connect_with_metadata(
    mut engine: E,
    assets: Arc<FakeAssetCatalog>,
    mut connect: Connect,
    diagnostics: DiagnosticsFake,
  ) -> Self {
    if diagnostics.is_available()
      && !connect
        .modules
        .iter()
        .any(|module| module == "battlement.diagnostics")
    {
      connect.modules.push("battlement.diagnostics".to_owned());
    }
    let request = connect_message(&connect);
    let message = battlement_flatbuffers::ConnectView::read(request.as_bytes())
      .unwrap_or_else(|error| panic!("connect verification failed: {error}"));
    engine.set_time(Duration::ZERO);
    let response = engine
      .connect(message)
      .unwrap_or_else(|error| panic!("connect failed: {error}"));
    assert!(
      response.session_id() != [0; 16],
      "connect returned a zero session"
    );
    let session_id = battlement::SessionId::from_uuid(Uuid::from_bytes(response.session_id()))
      .expect("engine response verifies a nonzero session");
    let mut client = Self {
      engine,
      assets,
      connect,
      diagnostics,
      session_id,
      world: FakeWorld::default(),
      ui_world: UiWorld::default(),
      motion: crate::motion::MotionWorld::default(),
      accessibility: battlement::AccessibilitySnapshot::default(),
      geometry_registry: GeometryRegistry::default(),
      admitted_batches: HashSet::new(),
      executed_commands: HashSet::new(),
      scheduled_batches: Vec::new(),
      canceled_scopes: HashSet::new(),
      paused_scopes: HashMap::new(),
      work_objects: HashMap::new(),
      response_retention: None,
      operations: Vec::new(),
      presentation_ms: 0,
      presentation_advancing: false,
      frame: 0,
      audio_occurrences: Vec::new(),
      particle_occurrences: Vec::new(),
      next_action_number: 1,
      pointers: pointer::Pointers::default(),
      navigation: navigation::Navigation::default(),
      hovered: None,
      pressed: None,
      drag: None,
      capture: Capture::default(),
      held_navigation: None,
      held_keys: HashSet::new(),
      held_controller_buttons: HashSet::new(),
      clock: None,
      scroll_interactions: HashMap::new(),
      scroller_interactions: HashMap::new(),
      slider_interactions: HashMap::new(),
      slider_int_interactions: HashMap::new(),
      min_max_slider_interactions: HashMap::new(),
      text_field_interactions: HashMap::new(),
      ui_link_identities: HashMap::new(),
      journal: Vec::new(),
    };
    client.apply_response_bytes(response, ResponseMode::Initial);
    client
  }

  /// Borrows the connected engine for public application operations.
  pub fn engine_mut(&mut self) -> &mut E {
    &mut self.engine
  }

  /// Reconnects the engine using the original connection metadata.
  pub fn reconnect(&mut self) {
    let request = connect_message(&self.connect);
    let message = battlement_flatbuffers::ConnectView::read(request.as_bytes())
      .unwrap_or_else(|error| panic!("reconnect verification failed: {error}"));
    self.engine.set_time(self.presentation_time());
    let response = self
      .engine
      .connect(message)
      .unwrap_or_else(|error| panic!("reconnect failed for session {}: {error}", self.session_id));
    assert!(
      response.session_id() != [0; 16],
      "reconnect returned a zero session"
    );
    let next_session = battlement::SessionId::from_uuid(Uuid::from_bytes(response.session_id()))
      .expect("engine response verifies a nonzero session");
    assert!(
      next_session != self.session_id,
      "reconnect reused session {}",
      self.session_id
    );
    self.session_id = next_session;
    self.motion.capture(&self.world, &self.ui_world);
    self.motion.rebase_clock(self.presentation_ms * 1000);
    self.world = FakeWorld::default();
    self.ui_world = UiWorld::default();
    self.accessibility = battlement::AccessibilitySnapshot::default();
    self.geometry_registry = GeometryRegistry::default();
    self.admitted_batches.clear();
    self.canceled_scopes.clear();
    self.paused_scopes.clear();
    self.executed_commands.clear();
    self.reset_presentation();
    self.presentation_ms = 0;
    self.frame = 0;
    self.audio_occurrences.clear();
    self.particle_occurrences.clear();
    self.next_action_number = 1;
    self.capture = Capture::default();
    self.held_navigation = None;
    self.clear_device_state();
    self.scroll_interactions.clear();
    self.scroller_interactions.clear();
    self.slider_interactions.clear();
    self.slider_int_interactions.clear();
    self.min_max_slider_interactions.clear();
    self.text_field_interactions.clear();
    self.ui_link_identities.clear();
    self.apply_response_bytes(response, ResponseMode::Initial);
  }

  /// Returns the configured deterministic Diagnostics fake.
  #[must_use]
  pub fn diagnostics(&self) -> &DiagnosticsFake {
    &self.diagnostics
  }

  /// Returns the configured deterministic Diagnostics fake mutably.
  #[must_use]
  pub fn diagnostics_mut(&mut self) -> &mut DiagnosticsFake {
    &mut self.diagnostics
  }

  /// Executes one concrete response supplied by an in-process application driver.
  pub fn receive(&mut self, response: battlement_native::EngineResponse) {
    self.apply_response_bytes(response, ResponseMode::Existing);
  }

  /// Reports the next finite presentation deadline without executing engine work.
  pub fn next_presentation_in(&self) -> Option<Duration> {
    self
      .next_deadline()
      .map(|deadline| Duration::from_millis(deadline.saturating_sub(self.presentation_ms)))
  }

  /// Applies exactly one queued engine response, when one is available.
  pub fn poll(&mut self) {
    self.poll_available();
  }

  /// Applies one queued response and reports whether one was available.
  #[doc(hidden)]
  pub fn poll_available(&mut self) -> bool {
    self.engine.set_time(self.presentation_time());
    let response = self
      .engine
      .poll()
      .unwrap_or_else(|error| panic!("poll failed for session {}: {error}", self.session_id));
    if let Some(response) = response {
      self.apply_response_bytes(response, ResponseMode::Existing);
      true
    } else {
      false
    }
  }

  /// Advances virtual rules and presentation time without inventing a rendered frame.
  pub fn advance_time(&mut self, duration: Duration) {
    let milliseconds = u64::try_from(duration.as_millis())
      .expect("fake presentation time exceeds the supported range");
    assert_eq!(
      Duration::from_millis(milliseconds),
      duration,
      "fake presentation time advances in whole milliseconds"
    );
    if let Some(clock) = &self.clock {
      clock.advance(duration);
    }
    let target = self
      .presentation_ms
      .checked_add(milliseconds)
      .expect("fake presentation time overflowed");
    self.advance_presentation_to(target);
  }

  /// Records one rendered-frame boundary without advancing virtual time.
  pub fn advance_frame(&mut self) {
    self.capture_frame();
    self.frame = self
      .frame
      .checked_add(1)
      .expect("fake frame count overflowed");
    self.pump_presentation();
  }

  /// Advances through all finite presentation work and leaves infinite work active.
  pub fn settle(&mut self) {
    self.pump_presentation();
    while let Some(deadline) = self.next_deadline() {
      self.advance_presentation_to(deadline);
    }
  }

  /// Advances to the next finite presentation event, if one exists.
  #[must_use]
  pub fn advance_to_next_presentation_event(&mut self) -> bool {
    self.pump_presentation();
    let Some(deadline) = self.next_deadline() else {
      return false;
    };
    self.advance_presentation_to(deadline);
    true
  }

  /// Returns elapsed fake presentation time.
  #[must_use]
  pub fn presentation_time(&self) -> Duration {
    Duration::from_millis(self.presentation_ms)
  }

  /// Returns the number of explicitly advanced rendered frames.
  #[must_use]
  pub fn frame(&self) -> u64 {
    self.frame
  }

  /// Returns audio play occurrences in execution order.
  #[must_use]
  pub fn audio_occurrences(&self) -> &[AudioOccurrence] {
    &self.audio_occurrences
  }

  /// Returns temporary particle-effect occurrences in execution order.
  #[must_use]
  pub fn particle_occurrences(&self) -> &[ParticleOccurrence] {
    &self.particle_occurrences
  }

  /// Returns a facade for UI state inspection and synthetic gestures.
  pub fn ui(&mut self) -> UiClient<'_, E> {
    UiClient { client: self }
  }

  /// Returns the current logical UI world for read-only observation.
  #[must_use]
  pub fn ui_world(&self) -> &UiWorld {
    &self.ui_world
  }

  /// Returns the semantic tree most recently applied through the host transport.
  #[must_use]
  pub fn accessibility(&self) -> &battlement::AccessibilitySnapshot {
    &self.accessibility
  }

  /// Activates a reachable world object through the coordinate-free input route.
  pub fn activate(&mut self, object_id: battlement::ObjectId) {
    self.require_input_enabled();
    self.require_clickable(object_id);
    self.submit_action(ActionBody::Activate(ActivationPayload { object_id }));
  }

  /// Performs a complete semantic mouse click on one object.
  pub fn click(&mut self, object_id: battlement::ObjectId) {
    self.click_at(object_id, self.world.world_transform(object_id).position);
  }

  /// Performs a complete semantic mouse click at one world-space hit point.
  pub fn click_at(&mut self, object_id: battlement::ObjectId, world_hit: Vector3) {
    self.complete_click(
      object_id,
      PointerInput {
        pointer_id: 0,
        screen_position: ScreenPosition {
          x: f64::from(self.connect.screen.width) / 2.0,
          y: f64::from(self.connect.screen.height) / 2.0,
        },
        world_hit,
        button: PointerButton::Left,
      },
    );
  }

  /// Moves a semantic pointer to an object or off all objects.
  pub fn move_pointer(&mut self, object_id: Option<battlement::ObjectId>, input: PointerInput) {
    self.require_input_enabled();
    assertions::validate_pointer_input(input);
    if let Some(object_id) = object_id {
      self.require_pointer_target(object_id);
    }
    if self
      .hovered
      .is_some_and(|state| Some(state.object_id) != object_id)
    {
      self.send_exit_for_hover();
      self.hovered = None;
    }
    let Some(object_id) = object_id else {
      self.reconcile_device_state();
      return;
    };
    if self
      .hovered
      .is_some_and(|state| state.object_id == object_id)
    {
      self.hovered = Some(PointerState { object_id, input });
      return;
    }
    if self.world.object(object_id).is_none() {
      self.reconcile_device_state();
      return;
    }
    self.hovered = Some(PointerState { object_id, input });
    self.send_pointer_event(PointerEvent::Enter, object_id, input);
    self.reconcile_device_state();
  }

  /// Presses a pointer button over the currently hovered object.
  pub fn pointer_down(&mut self, object_id: battlement::ObjectId, input: PointerInput) {
    self.require_input_enabled();
    assertions::validate_pointer_input(input);
    self.require_pointer_target(object_id);
    assert!(
      self
        .hovered
        .is_some_and(|state| state.object_id == object_id),
      "pointer down requires the target to be hovered: {object_id}"
    );
    self.pressed = Some(PressedPointer {
      object_id,
      pointer_id: input.pointer_id,
      button: input.button,
    });
    self.send_pointer_event(PointerEvent::Down, object_id, input);
    self.reconcile_device_state();
  }

  /// Releases a pointer button and emits click only for a matching press.
  pub fn pointer_up(&mut self, object_id: battlement::ObjectId, input: PointerInput) {
    self.require_input_enabled();
    assertions::validate_pointer_input(input);
    self.require_pointer_target(object_id);
    assert!(
      self
        .hovered
        .is_some_and(|state| state.object_id == object_id),
      "pointer up requires the target to be hovered: {object_id}"
    );
    self.send_pointer_event(PointerEvent::Up, object_id, input);
    if !self.world.input_enabled()
      || !self
        .world
        .object(object_id)
        .is_some_and(FakeObjectExt::valid_target)
    {
      self.pressed = None;
      self.reconcile_device_state();
      return;
    }
    let matches_press = self.pressed.is_some_and(|pressed| {
      pressed.object_id == object_id
        && pressed.pointer_id == input.pointer_id
        && pressed.button == input.button
    });
    if matches_press {
      self.send_pointer_event(PointerEvent::Click, object_id, input);
    }
    self.pressed = None;
    self.reconcile_device_state();
  }

  /// Cancels the current press without emitting a protocol action.
  pub fn pointer_cancel(&mut self) {
    self.require_input_enabled();
    self.pressed = None;
    self.drag = None;
  }

  /// Starts a semantic primary-pointer drag at the object's current world position.
  pub fn drag_start(&mut self, object_id: battlement::ObjectId, input: PointerInput) {
    self.require_input_enabled();
    assertions::validate_pointer_input(input);
    self.require_pointer_target(object_id);
    assert_eq!(
      input.button,
      PointerButton::Left,
      "drag requires the primary pointer"
    );
    assert!(self.drag.is_none(), "a drag is already active");
    assert!(
      self.world.require_object(object_id).drag_mode().is_some(),
      "object is not draggable: {object_id}"
    );
    let world_position = self.world.world_transform(object_id).position;
    self.submit_action(ActionBody::DragStart(DragPayload::new(
      object_id,
      input.pointer_id,
      input.screen_position,
      world_position,
    )));
    self.drag = Some(ActiveDrag {
      object_id,
      pointer_id: input.pointer_id,
    });
    self.reconcile_device_state();
  }

  /// Ends the active drag after moving the object to a world-space position.
  pub fn drag_end(
    &mut self,
    object_id: battlement::ObjectId,
    input: PointerInput,
    world_position: Vector3,
  ) {
    self.require_input_enabled();
    assertions::validate_pointer_input(input);
    assertions::validate_world_position(world_position);
    assert!(
      self
        .drag
        .is_some_and(|drag| { drag.object_id == object_id && drag.pointer_id == input.pointer_id }),
      "drag end does not match the active drag: {object_id}"
    );
    self.world.set_world_position(object_id, world_position);
    self.drag = None;
    self.submit_action(ActionBody::DragEnd(DragPayload::new(
      object_id,
      input.pointer_id,
      input.screen_position,
      world_position,
    )));
    self.reconcile_device_state();
  }

  /// Sends a physical key-down transition when the key is enabled and unheld.
  pub fn key_down(&mut self, key: PhysicalKey) {
    self.require_input_enabled();
    if self.capture.blocks() {
      if self.held_keys.insert(key) {
        self.capture_input(InputCaptureResult::Key { device_id: 0, key });
      }
      return;
    }
    assert!(
      self.world.global_keys().contains(&key),
      "key is not enabled: {key:?}"
    );
    if !self.held_keys.insert(key) {
      return;
    }
    if !self.route_semantic_key(key) {
      self.submit_action(ActionBody::KeyDown(battlement::KeyPayload { key }));
    }
    self.reconcile_device_state();
  }

  /// Sends a physical key-up transition when the key is enabled and held.
  pub fn key_up(&mut self, key: PhysicalKey) {
    if self.capture.blocks() {
      self.held_keys.remove(&key);
      self.capture_released();
      return;
    }
    if !self.world.input_enabled() {
      self.held_keys.remove(&key);
      return;
    }
    assert!(
      self.world.global_keys().contains(&key),
      "key is not enabled: {key:?}"
    );
    if !self.held_keys.remove(&key) {
      return;
    }
    self.submit_action(ActionBody::KeyUp(battlement::KeyPayload { key }));
    self.reconcile_device_state();
  }

  /// Sends an enabled controller-button down transition when it is not already held.
  pub fn controller_button_down(&mut self, controller_id: i32, button: ControllerButton) {
    if self.capture.blocks() {
      if self.held_controller_buttons.insert((controller_id, button)) {
        self.capture_input(InputCaptureResult::Button(ControllerButtonPayload {
          controller_id,
          button,
        }));
      }
      return;
    }
    self.require_input_enabled();
    self.require_controller_button(button);
    if !self.held_controller_buttons.insert((controller_id, button)) {
      return;
    }
    if !self.route_controller_button(button) {
      self.submit_action(ActionBody::ControllerButtonDown(ControllerButtonPayload {
        controller_id,
        button,
      }));
    }
    self.reconcile_device_state();
  }

  /// Sends an enabled controller-button up transition when it is held.
  pub fn controller_button_up(&mut self, controller_id: i32, button: ControllerButton) {
    if self.capture.blocks() {
      self
        .held_controller_buttons
        .remove(&(controller_id, button));
      self.capture_released();
      return;
    }
    if !self.world.input_enabled() {
      self
        .held_controller_buttons
        .remove(&(controller_id, button));
      return;
    }
    self.require_controller_button(button);
    if !self
      .held_controller_buttons
      .remove(&(controller_id, button))
    {
      return;
    }
    self.submit_action(ActionBody::ControllerButtonUp(ControllerButtonPayload {
      controller_id,
      button,
    }));
    self.reconcile_device_state();
  }

  /// Sends one enabled discrete controller-navigation action.
  pub fn controller_navigate(
    &mut self,
    controller_id: i32,
    direction: ControllerDirection,
    source: ControllerNavigationSource,
    repeat: bool,
  ) {
    self.require_input_enabled();
    let input = ControllerNavigationPayload {
      controller_id,
      direction,
      source,
      repeat,
    };
    self.held_navigation = Some(input);
    if self.capture_input(InputCaptureResult::Direction(input)) {
      return;
    }
    assert!(
      self
        .world
        .controller_input()
        .is_some_and(|settings| settings.navigation_enabled),
      "controller navigation is not enabled"
    );
    let navigation = match direction {
      ControllerDirection::Left => battlement::NavigationDirection::Left,
      ControllerDirection::Right => battlement::NavigationDirection::Right,
      ControllerDirection::Up => battlement::NavigationDirection::Up,
      ControllerDirection::Down => battlement::NavigationDirection::Down,
    };
    if !self.route_controller_navigation(navigation) {
      self.submit_action(ActionBody::ControllerNavigate(
        ControllerNavigationPayload {
          controller_id,
          direction,
          source,
          repeat,
        },
      ));
    }
    self.reconcile_device_state();
  }

  /// Returns the current fake world.
  #[must_use]
  pub fn world(&self) -> &FakeWorld {
    &self.world
  }

  /// Returns the active geometry observation registry.
  #[must_use]
  pub fn geometry_registry(&self) -> &GeometryRegistry {
    &self.geometry_registry
  }

  /// Submits one coherent geometry sample through the rules engine.
  pub fn submit_geometry(&mut self, batch: GeometryObservationBatch) {
    self
      .geometry_registry
      .accept_batch(&batch)
      .unwrap_or_else(|error| panic!("geometry batch failed: {error:?}"));
    self.submit_action(ActionBody::GeometryObservations(batch));
  }

  /// Submits ordered Motion lifecycle boundaries through the rules engine.
  pub fn submit_motion(&mut self, batch: battlement::MotionEventBatch) {
    self.submit_action(ActionBody::MotionEvents(batch));
  }

  /// Returns commands in complete execution order.
  #[must_use]
  pub fn commands(&self) -> &[ExecutedCommand] {
    &self.journal
  }

  /// Clears the command journal without changing the world.
  pub fn clear_commands(&mut self) {
    self.journal.clear();
  }

  /// Captures the current end of the command journal.
  #[must_use]
  pub fn checkpoint(&self) -> CommandCheckpoint {
    CommandCheckpoint::new(self.journal.len())
  }

  /// Returns the sole object created after a checkpoint or panics with the matching IDs.
  #[must_use]
  pub fn assert_one_object_created_since(
    &self,
    checkpoint: CommandCheckpoint,
  ) -> battlement::ObjectId {
    assert!(
      checkpoint.length <= self.journal.len(),
      "command checkpoint was invalidated by clearing the journal"
    );
    let created = self.journal[checkpoint.length..]
      .iter()
      .filter_map(|entry| match &entry.command.body {
        battlement::CommandBody::ObjectCreate(value) => Some(value.object.object_id),
        _ => None,
      })
      .collect::<Vec<_>>();
    assert_eq!(
      created.len(),
      1,
      "expected exactly one object creation after checkpoint; created: {created:?}"
    );
    created[0]
  }

  /// Asserts that a journal command matches a caller-supplied predicate.
  pub fn assert_command(&self, description: &str, predicate: impl Fn(&Command) -> bool) {
    assert!(
      self.journal.iter().any(|entry| predicate(&entry.command)),
      "{description}; command journal: {:?}",
      self.journal
    );
  }

  /// Returns an object or panics with its identifier.
  #[must_use]
  pub fn assert_object(&self, id: battlement::ObjectId) -> &crate::world::FakeObject {
    self
      .world
      .object(id)
      .unwrap_or_else(|| panic!("expected object to exist: {id}"))
  }

  /// Asserts that an object ID is absent from the current world.
  pub fn assert_object_absent(&self, id: battlement::ObjectId) {
    assert!(
      self.world.object(id).is_none(),
      "expected object to be absent: {id}"
    );
  }

  /// Asserts complete protocol kind equality for an object.
  pub fn assert_object_kind(
    &self,
    id: battlement::ObjectId,
    expected: &battlement::GameObjectKind,
  ) {
    assert_eq!(
      self.assert_object(id).kind(),
      expected,
      "object kind mismatch: {id}"
    );
  }

  /// Asserts complete image state equality for an object.
  pub fn assert_image(&self, id: battlement::ObjectId, expected: &ImageState) {
    let actual = self
      .assert_object(id)
      .image()
      .unwrap_or_else(|| panic!("expected object to be an image: {id}"));
    assert_eq!(actual, expected, "object image mismatch: {id}");
  }

  /// Asserts the visible text content of an object.
  pub fn assert_text(&self, id: battlement::ObjectId, expected: &str) {
    let actual = self
      .assert_object(id)
      .text()
      .unwrap_or_else(|| panic!("expected object to be text: {id}"));
    assert_eq!(actual.text, expected, "object text mismatch: {id}");
  }

  /// Asserts a local transform with an absolute component tolerance.
  pub fn assert_local_transform(
    &self,
    id: battlement::ObjectId,
    expected: battlement::LocalTransform,
    tolerance: f64,
  ) {
    assertions::assert_transform_close(
      self.assert_object(id).local_transform(),
      expected,
      tolerance,
      "local",
    );
  }

  /// Asserts a computed world transform with an absolute component tolerance.
  pub fn assert_world_transform(
    &self,
    id: battlement::ObjectId,
    expected: crate::world::WorldTransform,
    tolerance: f64,
  ) {
    assertions::assert_transform_close_world(
      self.world.world_transform(id),
      expected,
      tolerance,
      "world",
    );
  }

  /// Asserts only a computed world position with an absolute component tolerance.
  pub fn assert_world_position(&self, id: battlement::ObjectId, expected: Vector3, tolerance: f64) {
    let actual = self.world.world_transform(id).position;
    assertions::assert_vector_close(actual, expected, tolerance, "world position");
  }

  /// Asserts that the journal is empty.
  pub fn assert_no_commands(&self) {
    assert!(
      self.journal.is_empty(),
      "expected no commands; journal: {:?}",
      self.journal
    );
  }

  fn complete_click(&mut self, object_id: battlement::ObjectId, input: PointerInput) {
    self.require_input_enabled();
    self.require_clickable(object_id);

    if self
      .hovered
      .is_some_and(|state| state.object_id != object_id)
    {
      self.send_exit_for_hover();
      self.hovered = None;
    }
    if self.hovered.is_none() {
      self.require_clickable(object_id);
      self.hovered = Some(PointerState { object_id, input });
      self.send_pointer_event(PointerEvent::Enter, object_id, input);
    } else {
      self.hovered = Some(PointerState { object_id, input });
    }

    self.require_clickable(object_id);
    self.pressed = Some(PressedPointer {
      object_id,
      pointer_id: input.pointer_id,
      button: input.button,
    });
    self.send_pointer_event(PointerEvent::Down, object_id, input);
    self.require_complete_click_target(object_id);
    self.send_pointer_event(PointerEvent::Up, object_id, input);
    self.require_complete_click_target(object_id);
    self.send_pointer_event(PointerEvent::Click, object_id, input);
    self.pressed = None;
    self.reconcile_device_state();
  }

  fn apply_response(&mut self, response: Response, mode: ResponseMode) {
    assert!(
      !response.session_id.as_uuid().is_nil(),
      "response has a zero session"
    );
    match mode {
      ResponseMode::Initial => {
        assert!(
          !response.messages.is_empty(),
          "initial response has no messages"
        );
        assert!(
          matches!(
            response.messages.first(),
            Some(ResponseMessage::Snapshot(_))
          ),
          "initial response must begin with a snapshot"
        );
        assert!(
          response.session_id == self.session_id,
          "initial response session mismatch"
        );
      }
      ResponseMode::Existing => assert!(
        response.session_id == self.session_id,
        "response belongs to session {}, expected {}",
        response.session_id,
        self.session_id
      ),
    }
    for message in response.messages {
      match message {
        ResponseMessage::Snapshot(snapshot) => {
          assert!(
            snapshot.session_id == response.session_id,
            "snapshot session mismatch"
          );
          snapshot.validate().unwrap_or_else(|error| {
            panic!(
              "snapshot validation failed for session {}: {error}",
              snapshot.session_id
            )
          });
          self.reset_presentation();
          self.motion.capture(&self.world, &self.ui_world);
          let mut motion = snapshot
            .objects
            .iter()
            .filter_map(|object| object.motion.as_deref().cloned())
            .collect::<Vec<_>>();
          fn collect_motion(
            node: &battlement::UiNode,
            values: &mut Vec<battlement::MotionDescriptor>,
          ) {
            if let battlement::Prop::Set(value) = &node.element.visual_element().motion {
              values.push(value.clone());
            }
            for child in &node.children {
              collect_motion(child, values);
            }
          }
          for document in &snapshot.ui {
            if let battlement::Prop::Set(value) = &document.element.motion {
              motion.push(value.clone());
            }
            for child in &document.children {
              collect_motion(child, &mut motion);
            }
          }
          self
            .ui_world
            .replace(snapshot.ui.clone())
            .unwrap_or_else(|error| panic!("UI snapshot replacement failed: {error:?}"));
          self.world.replace_snapshot(snapshot, &self.assets);
          self.motion.restore(
            motion,
            &mut self.world,
            &mut self.ui_world,
            self.presentation_ms * 1000,
          );
          self.clear_device_state();
        }
        ResponseMessage::Batch(batch) => {
          assert!(
            batch.session_id == response.session_id,
            "batch session mismatch"
          );
          self.apply_batch(batch);
        }
      }
    }
  }

  fn apply_response_bytes(
    &mut self,
    response: battlement_native::EngineResponse,
    mode: ResponseMode,
  ) {
    let decoded = crate::response_reader::read(response.as_bytes())
      .unwrap_or_else(|error| panic!("verified response decoding failed: {error}"));
    let previous = std::mem::replace(&mut self.response_retention, response.retention());
    self.apply_response(decoded, mode);
    self.response_retention = previous;
    self.pump_presentation();
  }

  fn apply_batch(&mut self, batch: Batch) {
    if self.admitted_batches.contains(&batch.batch_id) {
      return;
    }
    assert!(
      !batch.groups.is_empty()
        || batch.cancel_scope.is_some()
        || batch.presentation_control.is_some(),
      "batch has no command groups: {}",
      batch.batch_id
    );
    for group in &batch.groups {
      assert!(
        !group.commands.is_empty(),
        "empty command group in batch {}",
        batch.batch_id
      );
    }
    self.admitted_batches.insert(batch.batch_id);
    self.schedule_batch(batch);
  }

  pub(crate) fn submit_action(&mut self, body: ActionBody) {
    let action_id = ActionId::from_uuid(Uuid::from_u128(self.next_action_number))
      .expect("deterministic action ID must be nonzero");
    self.next_action_number += 1;
    let action = Action::new(action_id, self.session_id, body);
    let message = battlement_flatbuffers::write_core_action(&action)
      .unwrap_or_else(|error| panic!("action encoding failed: {error}"));
    self.engine.set_time(self.presentation_time());
    let response = self
      .engine
      .submit(message.as_bytes())
      .unwrap_or_else(|error| panic!("submit failed for session {}: {error:?}", self.session_id));
    self.apply_response_bytes(response, ResponseMode::Existing);
  }

  pub(crate) fn submit_ui_event(&mut self, event: UiEvent) -> UiEventDisposition {
    if self.suppress_captured_ui(&event.body) {
      return UiEventDisposition::PreventDefault;
    }
    let forwards_ui_events = self
      .world
      .object(event.target_id)
      .and_then(|object| object.world_pointer)
      .is_none_or(|settings| settings.forwards_ui_events);
    self.motion.handle(
      &event,
      &mut self.world,
      &mut self.ui_world,
      self.presentation_ms * 1000,
    );
    if !forwards_ui_events {
      return UiEventDisposition::Continue;
    }
    let action_id = ActionId::from_uuid(Uuid::from_u128(self.next_action_number))
      .expect("deterministic action ID must be nonzero");
    self.next_action_number += 1;
    let action = UiEventAction::new(action_id, self.session_id, event);
    let message = battlement_flatbuffers::write_ui_event_action(&action)
      .unwrap_or_else(|error| panic!("UI event encoding failed: {error}"));
    let view = battlement_flatbuffers::UiEventActionView::read(message.as_bytes())
      .unwrap_or_else(|error| panic!("UI event verification failed: {error}"));
    self.engine.set_time(self.presentation_time());
    let result = self.engine.submit_ui_event(view).unwrap_or_else(|error| {
      panic!(
        "UI event submission failed for session {}: {error}",
        self.session_id
      )
    });
    assert_eq!(
      result.response.session_id(),
      *self.session_id.as_uuid().as_bytes(),
      "UI event response belongs to another session"
    );
    let disposition = result.disposition;
    self.apply_response_bytes(result.response, ResponseMode::Existing);
    disposition
  }

  pub(crate) fn submit_batch_failure(
    &mut self,
    batch_id: BatchId,
    command_id: CommandId,
    code: CoreErrorCode,
  ) {
    self.submit_presentation_failure(
      batch_id,
      command_id,
      code,
      "A Diagnostics command failed in the fake client.",
      true,
    );
  }

  pub(crate) fn submit_presentation_failure(
    &mut self,
    batch_id: BatchId,
    command_id: CommandId,
    code: CoreErrorCode,
    message: &str,
    blocking: bool,
  ) {
    let message = if blocking {
      battlement_flatbuffers::write_core_batch_failure(&BatchFailed::new(
        self.session_id,
        batch_id,
        Some(command_id),
        code,
        message,
      ))
    } else {
      battlement_flatbuffers::write_core_operation_failure(&battlement::OperationFailed::new(
        self.session_id,
        batch_id,
        command_id,
        code,
        message,
      ))
    }
    .expect("fake presentation failure must satisfy the FlatBuffers behavior");
    self.engine.set_time(self.presentation_time());
    let response = self
      .engine
      .submit(message.as_bytes())
      .unwrap_or_else(|error| {
        let error = match error {
          battlement_native::FlatBufferSubmitError::InvalidArgument(error)
          | battlement_native::FlatBufferSubmitError::Engine(error) => error,
        };
        panic!("batch-failure submit failed: {error}")
      });
    self.apply_response_bytes(response, ResponseMode::Existing);
  }

  fn send_pointer_event(
    &mut self,
    event: PointerEvent,
    object_id: battlement::ObjectId,
    input: PointerInput,
  ) {
    if !self
      .world
      .require_object(object_id)
      .pointer_events()
      .contains(&event)
    {
      return;
    }
    let body = match event {
      PointerEvent::Enter => ActionBody::PointerEnter(PointerPayload {
        object_id,
        pointer_id: input.pointer_id,
        screen_position: input.screen_position,
        world_hit: input.world_hit,
      }),
      PointerEvent::Exit => ActionBody::PointerExit(PointerPayload {
        object_id,
        pointer_id: input.pointer_id,
        screen_position: input.screen_position,
        world_hit: input.world_hit,
      }),
      PointerEvent::Down => ActionBody::PointerDown(PointerButtonPayload {
        object_id,
        pointer_id: input.pointer_id,
        screen_position: input.screen_position,
        world_hit: input.world_hit,
        button: input.button,
      }),
      PointerEvent::Up => ActionBody::PointerUp(PointerButtonPayload {
        object_id,
        pointer_id: input.pointer_id,
        screen_position: input.screen_position,
        world_hit: input.world_hit,
        button: input.button,
      }),
      PointerEvent::Click => ActionBody::PointerClick(PointerButtonPayload {
        object_id,
        pointer_id: input.pointer_id,
        screen_position: input.screen_position,
        world_hit: input.world_hit,
        button: input.button,
      }),
    };
    self.submit_action(body);
  }

  fn send_exit_for_hover(&mut self) {
    let Some(hovered) = self.hovered else {
      return;
    };
    if self
      .world
      .object(hovered.object_id)
      .is_some_and(FakeObjectExt::valid_target)
    {
      self.send_pointer_event(PointerEvent::Exit, hovered.object_id, hovered.input);
    }
  }

  fn require_clickable(&self, object_id: battlement::ObjectId) {
    self.require_pointer_target(object_id);
    assert!(
      self
        .world
        .require_object(object_id)
        .pointer_events()
        .contains(&PointerEvent::Click),
      "object is not clickable: {object_id}"
    );
  }

  fn require_complete_click_target(&mut self, object_id: battlement::ObjectId) {
    if !self.world.input_enabled()
      || !self
        .world
        .object(object_id)
        .is_some_and(FakeObjectExt::valid_target)
      || !self
        .world
        .require_object(object_id)
        .pointer_events()
        .contains(&PointerEvent::Click)
    {
      self.pressed = None;
      self.hovered = None;
      panic!("semantic click target became invalid: {object_id}");
    }
  }

  fn require_pointer_target(&self, object_id: battlement::ObjectId) {
    let object = self.world.require_object(object_id);
    assert!(
      object.active_in_hierarchy(),
      "object is inactive: {object_id}"
    );
    assert!(
      self.world.has_collider(object_id),
      "object has no pointer collider: {object_id}"
    );
  }

  fn require_input_enabled(&self) {
    assert!(self.world.input_enabled(), "fake input is disabled");
  }

  fn clear_device_state(&mut self) {
    self.motion.clear_gestures(
      &mut self.world,
      &mut self.ui_world,
      self.presentation_ms * 1000,
    );
    self.pointers = pointer::Pointers::default();
    self.navigation = navigation::Navigation::default();
    self.hovered = None;
    self.pressed = None;
    self.drag = None;
    if !self.capture.blocks() {
      self.held_keys.clear();
      self.held_controller_buttons.clear();
      self.held_navigation = None;
    }
  }

  pub(crate) fn reconcile_device_state(&mut self) {
    self.reconcile_geometric_pointers();
    self.reconcile_navigation();
    if !self.world.input_enabled() {
      self.clear_device_state();
      return;
    }
    if self.hovered.is_some_and(|state| {
      !self
        .world
        .object(state.object_id)
        .is_some_and(FakeObjectExt::valid_target)
    }) {
      self.hovered = None;
    }
    if self.pressed.is_some_and(|pressed| {
      !self
        .world
        .object(pressed.object_id)
        .is_some_and(FakeObjectExt::valid_target)
    }) {
      self.pressed = None;
    }
    if self.drag.is_some_and(|drag| {
      !self
        .world
        .object(drag.object_id)
        .is_some_and(FakeObjectExt::valid_drag_target)
    }) {
      self.drag = None;
    }
    if self.capture.blocks() {
      return;
    }
    self
      .held_keys
      .retain(|key| self.world.global_keys().contains(key));
    let enabled_buttons = self
      .world
      .controller_input()
      .map(|settings| settings.buttons.as_slice())
      .unwrap_or_default();
    self
      .held_controller_buttons
      .retain(|(_, button)| enabled_buttons.contains(button));
  }

  fn require_controller_button(&self, button: ControllerButton) {
    assert!(
      self
        .world
        .controller_input()
        .is_some_and(|settings| settings.buttons.contains(&button)),
      "controller button is not enabled: {button:?}"
    );
  }
}

fn connect_message(connect: &Connect) -> battlement_flatbuffers::FinishedMessage {
  battlement_flatbuffers::write_connect_request(connect)
    .unwrap_or_else(|error| panic!("connect encoding failed: {error}"))
}

enum ResponseMode {
  Initial,
  Existing,
}

trait FakeObjectExt {
  fn valid_target(&self) -> bool;

  fn valid_drag_target(&self) -> bool;
}

impl FakeObjectExt for crate::world::FakeObject {
  fn valid_target(&self) -> bool {
    self.active_in_hierarchy()
  }

  fn valid_drag_target(&self) -> bool {
    self.valid_target() && self.drag_mode().is_some()
  }
}
