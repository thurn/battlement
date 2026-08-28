//! Synchronous fake client lifecycle, responses, input, and assertions.

use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};

use battlement::{
  Action, ActionBody, ActionId, Batch, BatchId, ClientMessage, Command, CommandId, Connect,
  ControllerButton, ControllerButtonPayload, ControllerDirection, ControllerNavigationPayload,
  ControllerNavigationSource, DragPayload, GeometryRegistry, ImageState, PhysicalKey,
  PointerButton, PointerButtonPayload, PointerEvent, PointerPayload, Response, ResponseMessage,
  ScreenPosition, ScreenSize, Validate, Vector3,
};
use battlement_native::Engine;
use battlement_ui_fake::UiWorld;
use uuid::Uuid;

use crate::{
  assertions,
  assets::FakeAssetCatalog,
  client::ui::{
    MinMaxSliderInteraction, ScrollInteraction, ScrollerInteraction, SliderIntInteraction,
    TextFieldInteraction, UiClient,
  },
  journal::{CommandCheckpoint, ExecutedCommand},
  time::ManualClock,
  world::FakeWorld,
};

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
  E: Engine<Command = Command>,
{
  pub(crate) engine: E,
  pub(crate) assets: Arc<FakeAssetCatalog>,
  pub(crate) connect: Connect,
  pub(crate) session_id: battlement::SessionId,
  pub(crate) world: FakeWorld,
  pub(crate) ui_world: UiWorld,
  pub(crate) geometry_registry: GeometryRegistry,
  pub(crate) admitted_batches: HashSet<BatchId>,
  pub(crate) executed_commands: HashSet<CommandId>,
  pub(crate) next_action_number: u128,
  hovered: Option<PointerState>,
  pressed: Option<PressedPointer>,
  drag: Option<ActiveDrag>,
  held_keys: HashSet<PhysicalKey>,
  held_controller_buttons: HashSet<ControllerButton>,
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
  E: Engine<Command = Command>,
{
  /// Connects an engine with deterministic fake platform metadata.
  #[must_use]
  pub fn connect(engine: E, assets: impl Into<Arc<FakeAssetCatalog>>) -> Self {
    Self::connect_with(
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

  /// Connects an engine with explicit connection metadata.
  #[must_use]
  pub fn connect_with(
    mut engine: E,
    assets: impl Into<Arc<FakeAssetCatalog>>,
    connect: Connect,
  ) -> Self {
    let assets = assets.into();
    let response = engine
      .connect(connect.clone())
      .unwrap_or_else(|error| panic!("connect failed: {error}"));
    assert!(
      !response.session_id.as_uuid().is_nil(),
      "connect returned a zero session"
    );
    let session_id = response.session_id;
    let mut client = Self {
      engine,
      assets,
      connect,
      session_id,
      world: FakeWorld::default(),
      ui_world: UiWorld::default(),
      geometry_registry: GeometryRegistry::default(),
      admitted_batches: HashSet::new(),
      executed_commands: HashSet::new(),
      next_action_number: 1,
      hovered: None,
      pressed: None,
      drag: None,
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
    client.apply_response(response, ResponseMode::Initial);
    client
  }

  /// Reconnects the owned engine using the original connection metadata.
  pub fn reconnect(&mut self) {
    let response = self
      .engine
      .connect(self.connect.clone())
      .unwrap_or_else(|error| panic!("reconnect failed for session {}: {error}", self.session_id));
    assert!(
      !response.session_id.as_uuid().is_nil(),
      "reconnect returned a zero session"
    );
    assert!(
      response.session_id != self.session_id,
      "reconnect reused session {}",
      self.session_id
    );
    self.session_id = response.session_id;
    self.world = FakeWorld::default();
    self.ui_world = UiWorld::default();
    self.geometry_registry = GeometryRegistry::default();
    self.admitted_batches.clear();
    self.executed_commands.clear();
    self.next_action_number = 1;
    self.clear_device_state();
    self.scroll_interactions.clear();
    self.scroller_interactions.clear();
    self.slider_interactions.clear();
    self.slider_int_interactions.clear();
    self.min_max_slider_interactions.clear();
    self.text_field_interactions.clear();
    self.ui_link_identities.clear();
    self.apply_response(response, ResponseMode::Initial);
  }

  /// Applies exactly one queued engine response, when one is available.
  pub fn poll(&mut self) {
    let response = self
      .engine
      .poll()
      .unwrap_or_else(|error| panic!("poll failed for session {}: {error}", self.session_id));
    if let Some(response) = response {
      self.apply_response(response, ResponseMode::Existing);
    }
  }

  /// Returns a facade for UI state inspection and synthetic gestures.
  pub fn ui(&mut self) -> UiClient<'_, E> {
    UiClient { client: self }
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
    assert!(
      self.world.global_keys().contains(&key),
      "key is not enabled: {key:?}"
    );
    if !self.held_keys.insert(key) {
      return;
    }
    self.submit_action(ActionBody::KeyDown(battlement::KeyPayload { key }));
    self.reconcile_device_state();
  }

  /// Sends a physical key-up transition when the key is enabled and held.
  pub fn key_up(&mut self, key: PhysicalKey) {
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
    self.require_input_enabled();
    self.require_controller_button(button);
    if !self.held_controller_buttons.insert(button) {
      return;
    }
    self.submit_action(ActionBody::ControllerButtonDown(ControllerButtonPayload {
      controller_id,
      button,
    }));
    self.reconcile_device_state();
  }

  /// Sends an enabled controller-button up transition when it is held.
  pub fn controller_button_up(&mut self, controller_id: i32, button: ControllerButton) {
    if !self.world.input_enabled() {
      self.held_controller_buttons.remove(&button);
      return;
    }
    self.require_controller_button(button);
    if !self.held_controller_buttons.remove(&button) {
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
    assert!(
      self
        .world
        .controller_input()
        .is_some_and(|settings| settings.navigation_enabled),
      "controller navigation is not enabled"
    );
    self.submit_action(ActionBody::ControllerNavigate(
      ControllerNavigationPayload {
        controller_id,
        direction,
        source,
        repeat,
      },
    ));
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
          self
            .ui_world
            .replace(snapshot.ui.clone())
            .unwrap_or_else(|error| panic!("UI snapshot replacement failed: {error:?}"));
          self.world.replace_snapshot(snapshot, &self.assets);
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

  fn apply_batch(&mut self, batch: Batch) {
    if self.admitted_batches.contains(&batch.batch_id) {
      return;
    }
    assert!(
      !batch.groups.is_empty(),
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
    for (group_index, group) in batch.groups.into_iter().enumerate() {
      for (command_index, command) in group.commands.into_iter().enumerate() {
        self.execute_command(command, batch.batch_id, group_index, command_index);
      }
    }
  }

  fn submit_action(&mut self, body: ActionBody) {
    let action_id = ActionId::from_uuid(Uuid::from_u128(self.next_action_number))
      .expect("deterministic action ID must be nonzero");
    self.next_action_number += 1;
    let response = self
      .engine
      .submit(ClientMessage::Action(Action::new(
        action_id,
        self.session_id,
        body,
      )))
      .unwrap_or_else(|error| panic!("submit failed for session {}: {error}", self.session_id));
    self.apply_response(response, ResponseMode::Existing);
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
    self.hovered = None;
    self.pressed = None;
    self.drag = None;
    self.held_keys.clear();
    self.held_controller_buttons.clear();
  }

  pub(crate) fn reconcile_device_state(&mut self) {
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
      .retain(|button| enabled_buttons.contains(button));
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
