//! Synchronous fake client lifecycle, responses, input, and assertions.

use std::{collections::HashSet, sync::Arc};

use masonry::{
    Action, ActionBody, ActionId, Batch, BatchId, ClientMessage, Command, CommandId, Connect,
    KeyCode, PointerButton, PointerButtonPayload, PointerEvent, PointerPayload, Response,
    ResponseMessage, ScreenPosition, ScreenSize, Validate, Vector3,
};
use masonry_native::Engine;
use uuid::Uuid;

use crate::{assets::FakeAssetCatalog, journal::ExecutedCommand, world::FakeWorld};

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
    object_id: masonry::ObjectId,
    input: PointerInput,
}

#[derive(Clone, Copy)]
struct PressedPointer {
    object_id: masonry::ObjectId,
    pointer_id: i32,
    button: PointerButton,
}

/// An in-memory Masonry client driven by a typed rules engine.
pub struct FakeClient<E>
where
    E: Engine<Command = Command>,
{
    pub(crate) engine: E,
    pub(crate) assets: Arc<FakeAssetCatalog>,
    pub(crate) connect: Connect,
    pub(crate) session_id: masonry::SessionId,
    pub(crate) world: FakeWorld,
    pub(crate) admitted_batches: HashSet<BatchId>,
    pub(crate) executed_commands: HashSet<CommandId>,
    pub(crate) next_action_number: u128,
    hovered: Option<PointerState>,
    pressed: Option<PressedPointer>,
    held_keys: HashSet<KeyCode>,
    pub(crate) journal: Vec<ExecutedCommand>,
}

impl<E> FakeClient<E>
where
    E: Engine<Command = Command>,
{
    /// Connects an engine with deterministic fake platform metadata.
    #[must_use]
    pub fn connect(engine: E, assets: Arc<FakeAssetCatalog>) -> Self {
        Self::connect_with(
            engine,
            assets,
            Connect::new(
                "masonry-fake",
                "masonry-fake",
                ScreenSize {
                    width: 1_920,
                    height: 1_080,
                },
            ),
        )
    }

    /// Connects an engine with explicit connection metadata.
    #[must_use]
    pub fn connect_with(mut engine: E, assets: Arc<FakeAssetCatalog>, connect: Connect) -> Self {
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
            admitted_batches: HashSet::new(),
            executed_commands: HashSet::new(),
            next_action_number: 1,
            hovered: None,
            pressed: None,
            held_keys: HashSet::new(),
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
            .unwrap_or_else(|error| {
                panic!("reconnect failed for session {}: {error}", self.session_id)
            });
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
        self.admitted_batches.clear();
        self.executed_commands.clear();
        self.next_action_number = 1;
        self.clear_device_state();
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

    /// Performs a complete semantic mouse click on one object.
    pub fn click(&mut self, object_id: masonry::ObjectId) {
        self.require_input_enabled();
        self.require_clickable(object_id);
        let input = PointerInput {
            pointer_id: 0,
            screen_position: ScreenPosition {
                x: f64::from(self.connect.screen.width) / 2.0,
                y: f64::from(self.connect.screen.height) / 2.0,
            },
            world_hit: self.world.world_transform(object_id).position,
            button: PointerButton::Left,
        };

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

    /// Moves a semantic pointer to an object or off all objects.
    pub fn move_pointer(&mut self, object_id: Option<masonry::ObjectId>, input: PointerInput) {
        self.require_input_enabled();
        validate_pointer_input(input);
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
    pub fn pointer_down(&mut self, object_id: masonry::ObjectId, input: PointerInput) {
        self.require_input_enabled();
        validate_pointer_input(input);
        self.require_pointer_target(object_id);
        assert!(
            self.hovered
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
    pub fn pointer_up(&mut self, object_id: masonry::ObjectId, input: PointerInput) {
        self.require_input_enabled();
        validate_pointer_input(input);
        self.require_pointer_target(object_id);
        assert!(
            self.hovered
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
    }

    /// Sends a physical key-down transition when the key is enabled and unheld.
    pub fn key_down(&mut self, key: KeyCode) {
        self.require_input_enabled();
        assert!(
            self.world.global_keys().contains(&key),
            "key is not enabled: {key:?}"
        );
        if !self.held_keys.insert(key) {
            return;
        }
        self.submit_action(ActionBody::KeyDown(masonry::KeyPayload { key }));
        self.reconcile_device_state();
    }

    /// Sends a physical key-up transition when the key is enabled and held.
    pub fn key_up(&mut self, key: KeyCode) {
        self.require_input_enabled();
        assert!(
            self.world.global_keys().contains(&key),
            "key is not enabled: {key:?}"
        );
        if !self.held_keys.remove(&key) {
            return;
        }
        self.submit_action(ActionBody::KeyUp(masonry::KeyPayload { key }));
        self.reconcile_device_state();
    }

    /// Returns the current fake world.
    #[must_use]
    pub fn world(&self) -> &FakeWorld {
        &self.world
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
    pub fn assert_object(&self, id: masonry::ObjectId) -> &crate::world::FakeObject {
        self.world
            .object(id)
            .unwrap_or_else(|| panic!("expected object to exist: {id}"))
    }

    /// Asserts that an object ID is absent from the current world.
    pub fn assert_object_absent(&self, id: masonry::ObjectId) {
        assert!(
            self.world.object(id).is_none(),
            "expected object to be absent: {id}"
        );
    }

    /// Asserts complete protocol kind equality for an object.
    pub fn assert_object_kind(&self, id: masonry::ObjectId, expected: &masonry::GameObjectKind) {
        assert_eq!(
            self.assert_object(id).kind(),
            expected,
            "object kind mismatch: {id}"
        );
    }

    /// Asserts a local transform with an absolute component tolerance.
    pub fn assert_local_transform(
        &self,
        id: masonry::ObjectId,
        expected: masonry::LocalTransform,
        tolerance: f64,
    ) {
        assert_transform_close(
            self.assert_object(id).local_transform(),
            expected,
            tolerance,
            "local",
        );
    }

    /// Asserts a computed world transform with an absolute component tolerance.
    pub fn assert_world_transform(
        &self,
        id: masonry::ObjectId,
        expected: crate::world::WorldTransform,
        tolerance: f64,
    ) {
        assert_transform_close_world(self.world.world_transform(id), expected, tolerance, "world");
    }

    /// Asserts only a computed world position with an absolute component tolerance.
    pub fn assert_world_position(&self, id: masonry::ObjectId, expected: Vector3, tolerance: f64) {
        let actual = self.world.world_transform(id).position;
        assert_vector_close(actual, expected, tolerance, "world position");
    }

    /// Asserts that the journal is empty.
    pub fn assert_no_commands(&self) {
        assert!(
            self.journal.is_empty(),
            "expected no commands; journal: {:?}",
            self.journal
        );
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
            .unwrap_or_else(|error| {
                panic!("submit failed for session {}: {error}", self.session_id)
            });
        self.apply_response(response, ResponseMode::Existing);
    }

    fn send_pointer_event(
        &mut self,
        event: PointerEvent,
        object_id: masonry::ObjectId,
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

    fn require_clickable(&self, object_id: masonry::ObjectId) {
        self.require_pointer_target(object_id);
        assert!(
            self.world
                .require_object(object_id)
                .pointer_events()
                .contains(&PointerEvent::Click),
            "object is not clickable: {object_id}"
        );
    }

    fn require_complete_click_target(&mut self, object_id: masonry::ObjectId) {
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

    fn require_pointer_target(&self, object_id: masonry::ObjectId) {
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
        self.held_keys.clear();
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
        self.held_keys
            .retain(|key| self.world.global_keys().contains(key));
    }
}

enum ResponseMode {
    Initial,
    Existing,
}

trait FakeObjectExt {
    fn valid_target(&self) -> bool;
}

impl FakeObjectExt for crate::world::FakeObject {
    fn valid_target(&self) -> bool {
        self.active_in_hierarchy()
    }
}

fn validate_pointer_input(input: PointerInput) {
    assert!(input.pointer_id >= 0, "pointer ID must be nonnegative");
    assert!(
        input.screen_position.x.is_finite(),
        "pointer screen x must be finite"
    );
    assert!(
        input.screen_position.y.is_finite(),
        "pointer screen y must be finite"
    );
    assert!(
        input.world_hit.x.is_finite(),
        "pointer world x must be finite"
    );
    assert!(
        input.world_hit.y.is_finite(),
        "pointer world y must be finite"
    );
    assert!(
        input.world_hit.z.is_finite(),
        "pointer world z must be finite"
    );
}

fn assert_transform_close(
    actual: masonry::LocalTransform,
    expected: masonry::LocalTransform,
    tolerance: f64,
    label: &str,
) {
    assert_vector_close(actual.position, expected.position, tolerance, label);
    assert_vector_close(actual.scale, expected.scale, tolerance, label);
    assert_quaternion_close(actual.rotation, expected.rotation, tolerance, label);
}

fn assert_transform_close_world(
    actual: crate::world::WorldTransform,
    expected: crate::world::WorldTransform,
    tolerance: f64,
    label: &str,
) {
    assert_vector_close(actual.position, expected.position, tolerance, label);
    assert_vector_close(actual.scale, expected.scale, tolerance, label);
    assert_quaternion_close(actual.rotation, expected.rotation, tolerance, label);
}

fn assert_vector_close(actual: Vector3, expected: Vector3, tolerance: f64, label: &str) {
    assert!(tolerance >= 0.0, "tolerance must be nonnegative");
    assert!(
        (actual.x - expected.x).abs() <= tolerance,
        "{label} x mismatch"
    );
    assert!(
        (actual.y - expected.y).abs() <= tolerance,
        "{label} y mismatch"
    );
    assert!(
        (actual.z - expected.z).abs() <= tolerance,
        "{label} z mismatch"
    );
}

fn assert_quaternion_close(
    actual: masonry::Quaternion,
    expected: masonry::Quaternion,
    tolerance: f64,
    label: &str,
) {
    let direct = (actual.x - expected.x).abs() <= tolerance
        && (actual.y - expected.y).abs() <= tolerance
        && (actual.z - expected.z).abs() <= tolerance
        && (actual.w - expected.w).abs() <= tolerance;
    let negated = (actual.x + expected.x).abs() <= tolerance
        && (actual.y + expected.y).abs() <= tolerance
        && (actual.z + expected.z).abs() <= tolerance
        && (actual.w + expected.w).abs() <= tolerance;
    assert!(direct || negated, "{label} rotation mismatch");
}
