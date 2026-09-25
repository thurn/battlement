use crate::{
  FinishedMessage, MAXIMUM_MESSAGE_BYTES, ProtocolError,
  client_message_generated::battlement::flat_buffers::generated as wire,
  host_settings::{self, HostSettingsView},
  verifier_options,
};
use crate::{
  common_generated::ReducedMotionPreference as WireReducedMotionPreference,
  common_generated::Vector3d,
  ui_event_generated::{PanelPoint, PhysicalKey as WirePhysicalKey, PointerButtonKind},
};
use battlement::{
  Action, ActionBody, BatchCompleted, BatchFailed, BatchId, ControllerButton, ControllerDirection,
  ControllerNavigationSource, CoreErrorCode, InputCaptureEvent, OperationFailed, PhysicalKey,
  PointerButton, SessionId,
};

/// Constructs a native blocking-work completion receipt.
pub fn write_core_batch_completed(
  value: &BatchCompleted,
) -> Result<FinishedMessage, ProtocolError> {
  let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(128);
  let session_id = crate::common_generated::Uuid(*value.session_id.as_uuid().as_bytes());
  let batch_id = crate::common_generated::Uuid(*value.batch_id.as_uuid().as_bytes());
  let completed = wire::BatchCompleted::create(
    &mut builder,
    &wire::BatchCompletedArgs {
      session_id: Some(&session_id),
      batch_id: Some(&batch_id),
    },
  );
  self::finish_client_message(
    builder,
    wire::CoreClientMessageBody::BatchCompleted,
    completed.as_union_value(),
  )
}

/// Constructs one built-in action directly in a size-prefixed FlatBuffer.
pub fn write_core_action(value: &Action) -> Result<FinishedMessage, ProtocolError> {
  let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(512);
  let (kind, body_type, body) = write_action_body(&mut builder, &value.body)?;
  let action_id = crate::common_generated::Uuid(*value.action_id.as_uuid().as_bytes());
  let session_id = crate::common_generated::Uuid(*value.session_id.as_uuid().as_bytes());
  let action = wire::CoreAction::create(
    &mut builder,
    &wire::CoreActionArgs {
      action_id: Some(&action_id),
      session_id: Some(&session_id),
      kind,
      body_type,
      body: Some(body),
    },
  );
  finish_client_message(
    builder,
    wire::CoreClientMessageBody::CoreAction,
    action.as_union_value(),
  )
}

/// Constructs one late core-operation failure directly in a size-prefixed FlatBuffer.
pub fn write_core_operation_failure(
  value: &OperationFailed<CoreErrorCode>,
) -> Result<FinishedMessage, ProtocolError> {
  let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(256);
  let message = builder.create_string(&value.message);
  let session_id = crate::common_generated::Uuid(*value.session_id.as_uuid().as_bytes());
  let batch_id = crate::common_generated::Uuid(*value.batch_id.as_uuid().as_bytes());
  let command_id = crate::common_generated::Uuid(*value.command_id.as_uuid().as_bytes());
  let failure = wire::OperationFailed::create(
    &mut builder,
    &wire::OperationFailedArgs {
      session_id: Some(&session_id),
      batch_id: Some(&batch_id),
      command_id: Some(&command_id),
      error_code: wire::CoreErrorCode(value.error_code as u8),
      message: Some(message),
    },
  );
  finish_client_message(
    builder,
    wire::CoreClientMessageBody::OperationFailed,
    failure.as_union_value(),
  )
}

/// Constructs one core batch-failure submission directly in a size-prefixed FlatBuffer.
pub fn write_core_batch_failure(
  value: &BatchFailed<CoreErrorCode>,
) -> Result<FinishedMessage, ProtocolError> {
  let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(256);
  let message = builder.create_string(&value.message);
  let session_id = crate::common_generated::Uuid(*value.session_id.as_uuid().as_bytes());
  let batch_id = crate::common_generated::Uuid(*value.batch_id.as_uuid().as_bytes());
  let command_id = value
    .command_id
    .map(|id| crate::common_generated::Uuid(*id.as_uuid().as_bytes()));
  let failure = wire::BatchFailed::create(
    &mut builder,
    &wire::BatchFailedArgs {
      session_id: Some(&session_id),
      batch_id: Some(&batch_id),
      command_id: command_id.as_ref(),
      error_code: wire::CoreErrorCode(value.error_code as u8),
      message: Some(message),
    },
  );
  let root = wire::CoreClientMessage::create(
    &mut builder,
    &wire::CoreClientMessageArgs {
      body_type: wire::CoreClientMessageBody::BatchFailed,
      body: Some(failure.as_union_value()),
    },
  );
  wire::finish_size_prefixed_core_client_message_buffer(&mut builder, root);
  let (storage, start) = builder.collapse();
  let message = FinishedMessage::from_storage(storage, start);
  CoreClientMessageView::read(message.as_bytes())?;
  Ok(message)
}

fn finish_client_message(
  mut builder: flatbuffers::FlatBufferBuilder<'static>,
  body_type: wire::CoreClientMessageBody,
  body: flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
) -> Result<FinishedMessage, ProtocolError> {
  let root = wire::CoreClientMessage::create(
    &mut builder,
    &wire::CoreClientMessageArgs {
      body_type,
      body: Some(body),
    },
  );
  wire::finish_size_prefixed_core_client_message_buffer(&mut builder, root);
  let (storage, start) = builder.collapse();
  let message = FinishedMessage::from_storage(storage, start);
  CoreClientMessageView::read(message.as_bytes())?;
  Ok(message)
}

fn write_action_body<'a>(
  builder: &mut flatbuffers::FlatBufferBuilder<'a>,
  body: &ActionBody,
) -> Result<
  (
    wire::CoreActionKind,
    wire::CoreActionBody,
    flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
  ),
  ProtocolError,
> {
  use wire::{CoreActionBody as Body, CoreActionKind as Kind};
  let encoded = match body {
    ActionBody::Activate(value) => {
      let object_id = crate::common_generated::Uuid(*value.object_id.as_uuid().as_bytes());
      let value = wire::ActivationAction::create(
        builder,
        &wire::ActivationActionArgs {
          object_id: Some(&object_id),
        },
      );
      (
        Kind::Activate,
        Body::ActivationAction,
        value.as_union_value(),
      )
    }
    ActionBody::PointerEnter(value) | ActionBody::PointerExit(value) => {
      if value.pointer_id < 0 {
        return Err(error("pointer identity must be nonnegative"));
      }
      let kind = if matches!(body, ActionBody::PointerEnter(_)) {
        Kind::PointerEnter
      } else {
        Kind::PointerExit
      };
      let object_id = crate::common_generated::Uuid(*value.object_id.as_uuid().as_bytes());
      let screen = PanelPoint::new(value.screen_position.x, value.screen_position.y);
      let world = Vector3d::new(value.world_hit.x, value.world_hit.y, value.world_hit.z);
      let value = wire::PointerAction::create(
        builder,
        &wire::PointerActionArgs {
          object_id: Some(&object_id),
          pointer_id: value.pointer_id,
          screen_position: Some(&screen),
          world_hit: Some(&world),
        },
      );
      (kind, Body::PointerAction, value.as_union_value())
    }
    ActionBody::PointerDown(value)
    | ActionBody::PointerUp(value)
    | ActionBody::PointerClick(value) => {
      if value.pointer_id < 0 {
        return Err(error("pointer identity must be nonnegative"));
      }
      let kind = match body {
        ActionBody::PointerDown(_) => Kind::PointerDown,
        ActionBody::PointerUp(_) => Kind::PointerUp,
        _ => Kind::PointerClick,
      };
      let object_id = crate::common_generated::Uuid(*value.object_id.as_uuid().as_bytes());
      let screen = PanelPoint::new(value.screen_position.x, value.screen_position.y);
      let world = Vector3d::new(value.world_hit.x, value.world_hit.y, value.world_hit.z);
      let (button_kind, other) = match value.button {
        PointerButton::Left => (PointerButtonKind::Left, 0),
        PointerButton::Middle => (PointerButtonKind::Middle, 0),
        PointerButton::Right => (PointerButtonKind::Right, 0),
        PointerButton::Other(_) => {
          return Err(error("core actions require a built-in pointer button"));
        }
      };
      let button = crate::ui_event_generated::PointerButton::create(
        builder,
        &crate::ui_event_generated::PointerButtonArgs {
          kind: button_kind,
          other,
        },
      );
      let value = wire::PointerButtonAction::create(
        builder,
        &wire::PointerButtonActionArgs {
          object_id: Some(&object_id),
          pointer_id: value.pointer_id,
          screen_position: Some(&screen),
          world_hit: Some(&world),
          button: Some(button),
        },
      );
      (kind, Body::PointerButtonAction, value.as_union_value())
    }
    ActionBody::DragStart(value) | ActionBody::DragEnd(value) => {
      if value.pointer_id < 0 {
        return Err(error("pointer identity must be nonnegative"));
      }
      let kind = if matches!(body, ActionBody::DragStart(_)) {
        Kind::DragStart
      } else {
        Kind::DragEnd
      };
      let object_id = crate::common_generated::Uuid(*value.object_id.as_uuid().as_bytes());
      let screen = PanelPoint::new(value.screen_position.x, value.screen_position.y);
      let world = Vector3d::new(
        value.world_position.x,
        value.world_position.y,
        value.world_position.z,
      );
      let value = wire::DragAction::create(
        builder,
        &wire::DragActionArgs {
          object_id: Some(&object_id),
          pointer_id: value.pointer_id,
          screen_position: Some(&screen),
          world_position: Some(&world),
        },
      );
      (kind, Body::DragAction, value.as_union_value())
    }
    ActionBody::KeyDown(value) | ActionBody::KeyUp(value) => {
      let kind = if matches!(body, ActionBody::KeyDown(_)) {
        Kind::KeyDown
      } else {
        Kind::KeyUp
      };
      let value = wire::KeyAction::create(
        builder,
        &wire::KeyActionArgs {
          key: WirePhysicalKey(value.key as u16),
        },
      );
      (kind, Body::KeyAction, value.as_union_value())
    }
    ActionBody::ControllerButtonDown(value) | ActionBody::ControllerButtonUp(value) => {
      if value.controller_id < 0 {
        return Err(error("controller identity must be nonnegative"));
      }
      let kind = if matches!(body, ActionBody::ControllerButtonDown(_)) {
        Kind::ControllerButtonDown
      } else {
        Kind::ControllerButtonUp
      };
      let value = wire::ControllerButtonAction::create(
        builder,
        &wire::ControllerButtonActionArgs {
          controller_id: value.controller_id,
          button: wire::ControllerButton(value.button as u8),
        },
      );
      (kind, Body::ControllerButtonAction, value.as_union_value())
    }
    ActionBody::InputCaptured(value) => {
      let action = crate::input_capture::write_event(builder, *value)?;
      (
        Kind::InputCaptured,
        Body::InputCaptureAction,
        action.as_union_value(),
      )
    }
    ActionBody::ControllerNavigate(value) => {
      if value.controller_id < 0 {
        return Err(error("controller identity must be nonnegative"));
      }
      let value = wire::ControllerNavigateAction::create(
        builder,
        &wire::ControllerNavigateActionArgs {
          controller_id: value.controller_id,
          direction: wire::ControllerDirection(value.direction as u8),
          source: wire::ControllerNavigationSource(value.source as u8),
          repeat: value.repeat,
        },
      );
      (
        Kind::ControllerNavigate,
        Body::ControllerNavigateAction,
        value.as_union_value(),
      )
    }
    ActionBody::HostSettingsChanged(value) => {
      let settings = host_settings::write(builder, value)?;
      let action = wire::HostSettingsAction::create(
        builder,
        &wire::HostSettingsActionArgs {
          value: Some(settings),
        },
      );
      (
        Kind::HostSettingsChanged,
        Body::HostSettingsAction,
        action.as_union_value(),
      )
    }
    ActionBody::ApplicationStateChanged(value) => {
      let state = crate::common_generated::ApplicationState::create(
        builder,
        &crate::common_generated::ApplicationStateArgs {
          focused: value.focused,
          paused: value.paused,
        },
      );
      let value = wire::ApplicationStateAction::create(
        builder,
        &wire::ApplicationStateActionArgs { value: Some(state) },
      );
      (
        Kind::ApplicationStateChanged,
        Body::ApplicationStateAction,
        value.as_union_value(),
      )
    }
    ActionBody::ReducedMotionPreferenceChanged(value) => {
      let value = wire::ReducedMotionPreferenceAction::create(
        builder,
        &wire::ReducedMotionPreferenceActionArgs {
          value: WireReducedMotionPreference(*value as u8),
        },
      );
      (
        Kind::ReducedMotionPreferenceChanged,
        Body::ReducedMotionPreferenceAction,
        value.as_union_value(),
      )
    }
    ActionBody::GeometryObservations(value) => {
      let value = crate::core_action_geometry::write(builder, value)?;
      (
        Kind::GeometryObservations,
        Body::GeometryAction,
        value.as_union_value(),
      )
    }
    ActionBody::MotionEvents(value) => {
      let value = crate::core_action_motion::write(builder, value)?;
      (
        Kind::MotionEvents,
        Body::MotionAction,
        value.as_union_value(),
      )
    }
  };
  Ok(encoded)
}

/// One structurally verified and semantically checked core client submission.
#[derive(Clone, Copy)]
pub enum CoreClientMessageView<'a> {
  /// A discrete built-in input action.
  Action(CoreActionView<'a>),
  /// Successful completion of a scoped batch.
  BatchCompleted(BatchCompleted),
  /// A batch validation or execution failure.
  BatchFailed(BatchFailedView<'a>),
  /// A late failure from a nonblocking operation.
  OperationFailed(OperationFailedView<'a>),
}

impl<'a> CoreClientMessageView<'a> {
  /// Verifies and validates one complete size-prefixed core submission.
  pub fn read(bytes: &'a [u8]) -> Result<Self, ProtocolError> {
    if bytes.len() > MAXIMUM_MESSAGE_BYTES {
      return Err(error("core client message exceeds 16 MiB"));
    }
    if !size_prefix_matches(bytes) {
      return Err(error(
        "core client message size prefix does not match its finished range",
      ));
    }
    if bytes.len() < 12 || !wire::core_client_message_size_prefixed_buffer_has_identifier(bytes) {
      return Err(error("core client message has the wrong file identifier"));
    }
    let root =
      wire::size_prefixed_root_as_core_client_message_with_opts(&verifier_options(), bytes)
        .map_err(|failure| error(format!("invalid core client FlatBuffer: {failure}")))?;
    match root.body_type() {
      wire::CoreClientMessageBody::CoreAction => {
        let value = root
          .body_as_core_action()
          .ok_or_else(|| error("core action payload is missing"))?;
        validate_action(value)?;
        Ok(Self::Action(CoreActionView { value }))
      }
      wire::CoreClientMessageBody::BatchCompleted => {
        let value = root
          .body_as_batch_completed()
          .ok_or_else(|| error("batch-completed payload is missing"))?;
        let session_id = SessionId::from_uuid(uuid::Uuid::from_bytes(crate::ui_event::uuid_bytes(
          value.session_id(),
        )))
        .map_err(|_| error("batch-completed session is zero"))?;
        let batch_id = BatchId::from_uuid(uuid::Uuid::from_bytes(crate::ui_event::uuid_bytes(
          value.batch_id(),
        )))
        .map_err(|_| error("batch-completed batch is zero"))?;
        Ok(Self::BatchCompleted(BatchCompleted {
          session_id,
          batch_id,
        }))
      }
      wire::CoreClientMessageBody::BatchFailed => {
        let value = root
          .body_as_batch_failed()
          .ok_or_else(|| error("batch-failure payload is missing"))?;
        validate_batch_failure(value)?;
        Ok(Self::BatchFailed(BatchFailedView { value }))
      }
      wire::CoreClientMessageBody::OperationFailed => {
        let value = root
          .body_as_operation_failed()
          .ok_or_else(|| error("operation-failure payload is missing"))?;
        validate_operation_failure(value)?;
        Ok(Self::OperationFailed(OperationFailedView { value }))
      }
      _ => Err(error("unknown core client message union tag")),
    }
  }
}

/// A verified built-in action and its borrowed payload.
#[derive(Clone, Copy)]
pub struct CoreActionView<'a> {
  value: wire::CoreAction<'a>,
}

impl<'a> CoreActionView<'a> {
  /// Returns the canonical action UUID bytes.
  #[must_use]
  pub fn action_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.action_id())
  }

  /// Returns the canonical session UUID bytes.
  #[must_use]
  pub fn session_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.session_id())
  }

  /// Returns the closed borrowed action payload.
  #[must_use]
  pub fn body(self) -> CoreActionBodyView<'a> {
    action_body(self.value).expect("CoreActionView stores a validated kind and union")
  }
}

/// A borrowed core action payload.
#[derive(Clone, Copy)]
pub enum CoreActionBodyView<'a> {
  /// Coordinate-free activation.
  Activate(ActivationActionView<'a>),
  /// Pointer entered an object.
  PointerEnter(PointerActionView<'a>),
  /// Pointer exited an object.
  PointerExit(PointerActionView<'a>),
  /// Pointer button went down.
  PointerDown(PointerButtonActionView<'a>),
  /// Pointer button went up.
  PointerUp(PointerButtonActionView<'a>),
  /// Pointer clicked an object.
  PointerClick(PointerButtonActionView<'a>),
  /// Drag started.
  DragStart(DragActionView<'a>),
  /// Drag ended.
  DragEnd(DragActionView<'a>),
  /// Physical key went down.
  KeyDown(KeyActionView<'a>),
  /// Physical key went up.
  KeyUp(KeyActionView<'a>),
  /// Controller button went down.
  ControllerButtonDown(ControllerButtonActionView<'a>),
  /// Controller button went up.
  ControllerButtonUp(ControllerButtonActionView<'a>),
  /// Controller navigation step.
  ControllerNavigate(ControllerNavigateActionView<'a>),
  /// Terminal exclusive physical input capture result.
  InputCaptured(InputCaptureEvent),
  /// One coherent generation of changed geometry observations.
  GeometryObservations(crate::GeometryObservationBatchView<'a>),
  /// One normalized Motion event batch.
  MotionEvents(crate::MotionEventBatchView<'a>),
  /// Application focus or suspension changed.
  ApplicationStateChanged(ApplicationStateActionView<'a>),
  /// Reduced-motion preference changed.
  ReducedMotionPreferenceChanged(ReducedMotionPreferenceActionView<'a>),
  /// Host settings observations changed.
  /// Changed host capabilities and applied values.
  HostSettingsChanged(HostSettingsView<'a>),
}

/// Borrowed activation payload.
#[derive(Clone, Copy)]
pub struct ActivationActionView<'a> {
  value: wire::ActivationAction<'a>,
}

impl ActivationActionView<'_> {
  /// Returns the target object UUID bytes.
  #[must_use]
  pub fn object_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.object_id())
  }
}

/// Borrowed pointer-location payload.
#[derive(Clone, Copy)]
pub struct PointerActionView<'a> {
  value: wire::PointerAction<'a>,
}

impl PointerActionView<'_> {
  /// Returns the target object UUID bytes.
  #[must_use]
  pub fn object_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.object_id())
  }
  /// Returns the stable pointer identity.
  #[must_use]
  pub fn pointer_id(self) -> i32 {
    self.value.pointer_id()
  }
  /// Returns the screen position.
  #[must_use]
  pub fn screen_position(self) -> [f64; 2] {
    let value = self.value.screen_position();
    [value.x(), value.y()]
  }
  /// Returns the world hit position.
  #[must_use]
  pub fn world_hit(self) -> [f64; 3] {
    world_vector(self.value.world_hit())
  }
}

/// Borrowed pointer-button payload.
#[derive(Clone, Copy)]
pub struct PointerButtonActionView<'a> {
  value: wire::PointerButtonAction<'a>,
}

impl PointerButtonActionView<'_> {
  /// Returns the target object UUID bytes.
  #[must_use]
  pub fn object_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.object_id())
  }
  /// Returns the stable pointer identity.
  #[must_use]
  pub fn pointer_id(self) -> i32 {
    self.value.pointer_id()
  }
  /// Returns the screen position.
  #[must_use]
  pub fn screen_position(self) -> [f64; 2] {
    let value = self.value.screen_position();
    [value.x(), value.y()]
  }
  /// Returns the world hit position.
  #[must_use]
  pub fn world_hit(self) -> [f64; 3] {
    world_vector(self.value.world_hit())
  }
  /// Returns the stable pointer-button ordinal.
  #[must_use]
  pub fn button(self) -> u8 {
    self.value.button().kind().0
  }

  /// Returns the validated built-in pointer button.
  #[must_use]
  pub fn pointer_button(self) -> PointerButton {
    match self.value.button().kind() {
      PointerButtonKind::Left => PointerButton::Left,
      PointerButtonKind::Middle => PointerButton::Middle,
      PointerButtonKind::Right => PointerButton::Right,
      _ => unreachable!("core action validation closes pointer-button kinds"),
    }
  }
}

/// Borrowed drag payload.
#[derive(Clone, Copy)]
pub struct DragActionView<'a> {
  value: wire::DragAction<'a>,
}

impl DragActionView<'_> {
  /// Returns the target object UUID bytes.
  #[must_use]
  pub fn object_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.object_id())
  }
  /// Returns the stable pointer identity.
  #[must_use]
  pub fn pointer_id(self) -> i32 {
    self.value.pointer_id()
  }
  /// Returns the screen position.
  #[must_use]
  pub fn screen_position(self) -> [f64; 2] {
    let value = self.value.screen_position();
    [value.x(), value.y()]
  }
  /// Returns the world position.
  #[must_use]
  pub fn world_position(self) -> [f64; 3] {
    world_vector(self.value.world_position())
  }
}

/// Borrowed key payload.
#[derive(Clone, Copy)]
pub struct KeyActionView<'a> {
  value: wire::KeyAction<'a>,
}

impl KeyActionView<'_> {
  /// Returns the stable physical-key ordinal.
  #[must_use]
  pub fn key(self) -> u16 {
    self.value.key().0
  }

  /// Returns the validated physical key.
  #[must_use]
  pub fn physical_key(self) -> PhysicalKey {
    crate::ui_event_body::physical_key(self.value.key())
      .expect("core action validation closes physical-key ordinals")
  }
}

/// Borrowed controller-button payload.
#[derive(Clone, Copy)]
pub struct ControllerButtonActionView<'a> {
  value: wire::ControllerButtonAction<'a>,
}

impl ControllerButtonActionView<'_> {
  /// Returns the controller identity.
  #[must_use]
  pub fn controller_id(self) -> i32 {
    self.value.controller_id()
  }
  /// Returns the stable controller-button ordinal.
  #[must_use]
  pub fn button(self) -> u8 {
    self.value.button().0
  }

  /// Returns the validated controller button.
  #[must_use]
  pub fn controller_button(self) -> ControllerButton {
    match self.value.button() {
      wire::ControllerButton::South => ControllerButton::South,
      wire::ControllerButton::East => ControllerButton::East,
      wire::ControllerButton::West => ControllerButton::West,
      wire::ControllerButton::North => ControllerButton::North,
      wire::ControllerButton::LeftShoulder => ControllerButton::LeftShoulder,
      wire::ControllerButton::RightShoulder => ControllerButton::RightShoulder,
      wire::ControllerButton::LeftStickButton => ControllerButton::LeftStickButton,
      wire::ControllerButton::RightStickButton => ControllerButton::RightStickButton,
      wire::ControllerButton::Start => ControllerButton::Start,
      wire::ControllerButton::Select => ControllerButton::Select,
      _ => unreachable!("core action validation closes controller-button ordinals"),
    }
  }
}

/// Borrowed controller-navigation payload.
#[derive(Clone, Copy)]
pub struct ControllerNavigateActionView<'a> {
  value: wire::ControllerNavigateAction<'a>,
}

impl ControllerNavigateActionView<'_> {
  /// Returns the controller identity.
  #[must_use]
  pub fn controller_id(self) -> i32 {
    self.value.controller_id()
  }
  /// Returns the stable direction ordinal.
  #[must_use]
  pub fn direction(self) -> u8 {
    self.value.direction().0
  }

  /// Returns the validated cardinal direction.
  #[must_use]
  pub fn controller_direction(self) -> ControllerDirection {
    match self.value.direction() {
      wire::ControllerDirection::Left => ControllerDirection::Left,
      wire::ControllerDirection::Right => ControllerDirection::Right,
      wire::ControllerDirection::Up => ControllerDirection::Up,
      wire::ControllerDirection::Down => ControllerDirection::Down,
      _ => unreachable!("core action validation closes controller directions"),
    }
  }
  /// Returns the stable source ordinal.
  #[must_use]
  pub fn source(self) -> u8 {
    self.value.source().0
  }
  /// Returns the validated physical navigation source.
  #[must_use]
  pub fn navigation_source(self) -> ControllerNavigationSource {
    match self.value.source() {
      wire::ControllerNavigationSource::Dpad => ControllerNavigationSource::Dpad,
      wire::ControllerNavigationSource::LeftStick => ControllerNavigationSource::LeftStick,
      _ => unreachable!("core action validation closes controller navigation sources"),
    }
  }
  /// Returns whether this was a held-input repeat.
  #[must_use]
  pub fn is_repeat(self) -> bool {
    self.value.repeat()
  }
}

/// Borrowed application-state payload.
#[derive(Clone, Copy)]
pub struct ApplicationStateActionView<'a> {
  value: wire::ApplicationStateAction<'a>,
}

impl ApplicationStateActionView<'_> {
  /// Returns whether the application owns focus.
  #[must_use]
  pub fn focused(self) -> bool {
    self.value.value().focused()
  }
  /// Returns whether the application is paused.
  #[must_use]
  pub fn paused(self) -> bool {
    self.value.value().paused()
  }
}

/// Borrowed reduced-motion payload.
#[derive(Clone, Copy)]
pub struct ReducedMotionPreferenceActionView<'a> {
  value: wire::ReducedMotionPreferenceAction<'a>,
}

impl ReducedMotionPreferenceActionView<'_> {
  /// Returns the stable preference ordinal.
  #[must_use]
  pub fn value(self) -> u8 {
    self.value.value().0
  }
}

/// Borrowed batch-failure report.
#[derive(Clone, Copy)]
pub struct BatchFailedView<'a> {
  value: wire::BatchFailed<'a>,
}

impl<'a> BatchFailedView<'a> {
  /// Returns the session UUID bytes.
  #[must_use]
  pub fn session_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.session_id())
  }
  /// Returns the batch UUID bytes.
  #[must_use]
  pub fn batch_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.batch_id())
  }
  /// Returns the optional command UUID bytes.
  #[must_use]
  pub fn command_id(self) -> Option<[u8; 16]> {
    self.value.command_id().map(crate::ui_event::uuid_bytes)
  }
  /// Returns the stable core error-code ordinal.
  #[must_use]
  pub fn error_code(self) -> u8 {
    self.value.error_code().0
  }
  /// Returns the borrowed diagnostic text.
  #[must_use]
  pub fn message(self) -> &'a str {
    self.value.message()
  }
}

/// Borrowed late operation-failure report.
#[derive(Clone, Copy)]
pub struct OperationFailedView<'a> {
  value: wire::OperationFailed<'a>,
}

impl<'a> OperationFailedView<'a> {
  /// Returns the session UUID bytes.
  #[must_use]
  pub fn session_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.session_id())
  }
  /// Returns the batch UUID bytes.
  #[must_use]
  pub fn batch_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.batch_id())
  }
  /// Returns the command UUID bytes.
  #[must_use]
  pub fn command_id(self) -> [u8; 16] {
    crate::ui_event::uuid_bytes(self.value.command_id())
  }
  /// Returns the stable core error-code ordinal.
  #[must_use]
  pub fn error_code(self) -> u8 {
    self.value.error_code().0
  }
  /// Returns the borrowed diagnostic text.
  #[must_use]
  pub fn message(self) -> &'a str {
    self.value.message()
  }
}

fn validate_action(value: wire::CoreAction<'_>) -> Result<(), ProtocolError> {
  if nil(value.action_id()) || nil(value.session_id()) {
    return Err(error("core action UUIDs must be nonzero"));
  }
  action_body(value).map(|_| ())
}

fn action_body(value: wire::CoreAction<'_>) -> Result<CoreActionBodyView<'_>, ProtocolError> {
  use wire::{CoreActionBody as Body, CoreActionKind as Kind};
  let mismatch = || error("core action kind and payload type do not match");
  Ok(match (value.kind(), value.body_type()) {
    (Kind::Activate, Body::ActivationAction) => {
      CoreActionBodyView::Activate(ActivationActionView {
        value: value.body_as_activation_action().ok_or_else(mismatch)?,
      })
    }
    (Kind::PointerEnter, Body::PointerAction) => CoreActionBodyView::PointerEnter(pointer(value)?),
    (Kind::PointerExit, Body::PointerAction) => CoreActionBodyView::PointerExit(pointer(value)?),
    (Kind::PointerDown, Body::PointerButtonAction) => {
      CoreActionBodyView::PointerDown(pointer_button(value)?)
    }
    (Kind::PointerUp, Body::PointerButtonAction) => {
      CoreActionBodyView::PointerUp(pointer_button(value)?)
    }
    (Kind::PointerClick, Body::PointerButtonAction) => {
      CoreActionBodyView::PointerClick(pointer_button(value)?)
    }
    (Kind::DragStart, Body::DragAction) => CoreActionBodyView::DragStart(drag(value)?),
    (Kind::DragEnd, Body::DragAction) => CoreActionBodyView::DragEnd(drag(value)?),
    (Kind::KeyDown, Body::KeyAction) => CoreActionBodyView::KeyDown(key(value)?),
    (Kind::KeyUp, Body::KeyAction) => CoreActionBodyView::KeyUp(key(value)?),
    (Kind::ControllerButtonDown, Body::ControllerButtonAction) => {
      CoreActionBodyView::ControllerButtonDown(controller_button(value)?)
    }
    (Kind::ControllerButtonUp, Body::ControllerButtonAction) => {
      CoreActionBodyView::ControllerButtonUp(controller_button(value)?)
    }
    (Kind::InputCaptured, Body::InputCaptureAction) => CoreActionBodyView::InputCaptured(
      crate::input_capture::read_event(value.body_as_input_capture_action().ok_or_else(mismatch)?)?,
    ),
    (Kind::ControllerNavigate, Body::ControllerNavigateAction) => {
      CoreActionBodyView::ControllerNavigate(controller_navigate(value)?)
    }
    (Kind::GeometryObservations, Body::GeometryAction) => {
      let action = value.body_as_geometry_action().ok_or_else(mismatch)?;
      CoreActionBodyView::GeometryObservations(crate::GeometryObservationBatchView::new(
        action.value(),
      )?)
    }
    (Kind::MotionEvents, Body::MotionAction) => {
      let action = value.body_as_motion_action().ok_or_else(mismatch)?;
      CoreActionBodyView::MotionEvents(crate::MotionEventBatchView::new(action.value())?)
    }
    (Kind::HostSettingsChanged, Body::HostSettingsAction) => {
      let action = value.body_as_host_settings_action().ok_or_else(mismatch)?;
      CoreActionBodyView::HostSettingsChanged(HostSettingsView::new(action.value())?)
    }
    (Kind::ApplicationStateChanged, Body::ApplicationStateAction) => {
      CoreActionBodyView::ApplicationStateChanged(ApplicationStateActionView {
        value: value
          .body_as_application_state_action()
          .ok_or_else(mismatch)?,
      })
    }
    (Kind::ReducedMotionPreferenceChanged, Body::ReducedMotionPreferenceAction) => {
      let action = value
        .body_as_reduced_motion_preference_action()
        .ok_or_else(mismatch)?;
      if action.value().0 > WireReducedMotionPreference::NoPreference.0 {
        return Err(error("unknown reduced-motion preference"));
      }
      CoreActionBodyView::ReducedMotionPreferenceChanged(ReducedMotionPreferenceActionView {
        value: action,
      })
    }
    _ => return Err(mismatch()),
  })
}

fn pointer(value: wire::CoreAction<'_>) -> Result<PointerActionView<'_>, ProtocolError> {
  let value = value
    .body_as_pointer_action()
    .ok_or_else(|| error("pointer payload is missing"))?;
  validate_pointer(
    value.pointer_id(),
    value.screen_position(),
    value.world_hit(),
  )?;
  if nil(value.object_id()) {
    return Err(error("pointer object UUID must be nonzero"));
  }
  Ok(PointerActionView { value })
}

fn pointer_button(
  value: wire::CoreAction<'_>,
) -> Result<PointerButtonActionView<'_>, ProtocolError> {
  let value = value
    .body_as_pointer_button_action()
    .ok_or_else(|| error("pointer-button payload is missing"))?;
  validate_pointer(
    value.pointer_id(),
    value.screen_position(),
    value.world_hit(),
  )?;
  if nil(value.object_id()) {
    return Err(error("pointer-button object UUID must be nonzero"));
  }
  if !matches!(
    value.button().kind(),
    PointerButtonKind::Left | PointerButtonKind::Middle | PointerButtonKind::Right
  ) {
    return Err(error("unknown core pointer button"));
  }
  if value.button().other() != 0 {
    return Err(error("core pointer button has a noncanonical custom index"));
  }
  Ok(PointerButtonActionView { value })
}

fn drag(value: wire::CoreAction<'_>) -> Result<DragActionView<'_>, ProtocolError> {
  let value = value
    .body_as_drag_action()
    .ok_or_else(|| error("drag payload is missing"))?;
  validate_pointer(
    value.pointer_id(),
    value.screen_position(),
    value.world_position(),
  )?;
  if nil(value.object_id()) {
    return Err(error("drag object UUID must be nonzero"));
  }
  Ok(DragActionView { value })
}

fn key(value: wire::CoreAction<'_>) -> Result<KeyActionView<'_>, ProtocolError> {
  let value = value
    .body_as_key_action()
    .ok_or_else(|| error("key payload is missing"))?;
  if value.key().0 > WirePhysicalKey::NumpadEnter.0 {
    return Err(error("unknown physical key"));
  }
  Ok(KeyActionView { value })
}

fn controller_button(
  value: wire::CoreAction<'_>,
) -> Result<ControllerButtonActionView<'_>, ProtocolError> {
  let value = value
    .body_as_controller_button_action()
    .ok_or_else(|| error("controller-button payload is missing"))?;
  if value.controller_id() < 0 || value.button().0 > wire::ControllerButton::Select.0 {
    return Err(error("invalid controller-button payload"));
  }
  Ok(ControllerButtonActionView { value })
}

fn controller_navigate(
  value: wire::CoreAction<'_>,
) -> Result<ControllerNavigateActionView<'_>, ProtocolError> {
  let value = value
    .body_as_controller_navigate_action()
    .ok_or_else(|| error("controller-navigation payload is missing"))?;
  if value.controller_id() < 0
    || value.direction().0 > wire::ControllerDirection::Down.0
    || value.source().0 > wire::ControllerNavigationSource::LeftStick.0
  {
    return Err(error("invalid controller-navigation payload"));
  }
  Ok(ControllerNavigateActionView { value })
}

fn validate_batch_failure(value: wire::BatchFailed<'_>) -> Result<(), ProtocolError> {
  if nil(value.session_id())
    || nil(value.batch_id())
    || value.command_id().is_some_and(nil)
    || value.error_code().0 > wire::CoreErrorCode::DiagnosticsOperationFailed.0
  {
    return Err(error("invalid batch-failure identity or error code"));
  }
  Ok(())
}

fn validate_operation_failure(value: wire::OperationFailed<'_>) -> Result<(), ProtocolError> {
  if nil(value.session_id())
    || nil(value.batch_id())
    || nil(value.command_id())
    || value.error_code().0 > wire::CoreErrorCode::DiagnosticsOperationFailed.0
  {
    return Err(error("invalid operation-failure identity or error code"));
  }
  Ok(())
}

fn validate_pointer(
  pointer_id: i32,
  screen: &PanelPoint,
  world: &Vector3d,
) -> Result<(), ProtocolError> {
  if pointer_id < 0
    || !screen.x().is_finite()
    || !screen.y().is_finite()
    || !world.x().is_finite()
    || !world.y().is_finite()
    || !world.z().is_finite()
  {
    return Err(error("invalid pointer identity or coordinates"));
  }
  Ok(())
}

fn world_vector(value: &Vector3d) -> [f64; 3] {
  [value.x(), value.y(), value.z()]
}

fn nil(value: &crate::common_generated::Uuid) -> bool {
  value.bytes().iter().all(|byte| byte == 0)
}

fn size_prefix_matches(bytes: &[u8]) -> bool {
  bytes.len() >= 4
    && usize::try_from(u32::from_le_bytes(
      bytes[..4].try_into().expect("length checked"),
    ))
    .ok()
      == bytes.len().checked_sub(4)
}

fn error(message: impl Into<String>) -> ProtocolError {
  ProtocolError::new(message)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::common_generated::Uuid;
  use crate::motion_generated as motion_wire;

  const ACTION_ID: Uuid = Uuid([1; 16]);
  const SESSION_ID: Uuid = Uuid([2; 16]);
  const OBJECT_ID: Uuid = Uuid([3; 16]);

  #[test]
  fn reads_borrowed_pointer_payload_without_precision_loss() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let button = crate::ui_event_generated::PointerButton::create(
      &mut builder,
      &crate::ui_event_generated::PointerButtonArgs {
        kind: PointerButtonKind::Right,
        other: 0,
      },
    );
    let screen = PanelPoint::new(1.234_567_890_123, -9.876_543_210_987);
    let world = Vector3d::new(123_456_789.125, -0.000_000_000_123, 42.5);
    let payload = wire::PointerButtonAction::create(
      &mut builder,
      &wire::PointerButtonActionArgs {
        object_id: Some(&OBJECT_ID),
        pointer_id: 7,
        screen_position: Some(&screen),
        world_hit: Some(&world),
        button: Some(button),
      },
    );
    let bytes = action_bytes(
      builder,
      wire::CoreActionKind::PointerClick,
      wire::CoreActionBody::PointerButtonAction,
      payload.as_union_value(),
    );

    let CoreClientMessageView::Action(action) = CoreClientMessageView::read(&bytes).unwrap() else {
      panic!("expected action");
    };
    assert_eq!(action.action_id(), [1; 16]);
    let CoreActionBodyView::PointerClick(pointer) = action.body() else {
      panic!("expected pointer click");
    };
    assert_eq!(pointer.object_id(), [3; 16]);
    assert_eq!(pointer.pointer_id(), 7);
    assert_eq!(
      pointer.screen_position(),
      [1.234_567_890_123, -9.876_543_210_987]
    );
    assert_eq!(
      pointer.world_hit(),
      [123_456_789.125, -0.000_000_000_123, 42.5]
    );
    assert_eq!(pointer.button(), PointerButtonKind::Right.0);
  }

  #[test]
  fn rejects_union_mismatch_unknown_enum_and_noncanonical_button() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let key = wire::KeyAction::create(
      &mut builder,
      &wire::KeyActionArgs {
        key: WirePhysicalKey::KeyA,
      },
    );
    let mismatch = action_bytes(
      builder,
      wire::CoreActionKind::Activate,
      wire::CoreActionBody::KeyAction,
      key.as_union_value(),
    );
    assert!(CoreClientMessageView::read(&mismatch).is_err());

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let key = wire::KeyAction::create(
      &mut builder,
      &wire::KeyActionArgs {
        key: WirePhysicalKey(u16::MAX),
      },
    );
    let unknown = action_bytes(
      builder,
      wire::CoreActionKind::KeyDown,
      wire::CoreActionBody::KeyAction,
      key.as_union_value(),
    );
    assert!(CoreClientMessageView::read(&unknown).is_err());

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let button = crate::ui_event_generated::PointerButton::create(
      &mut builder,
      &crate::ui_event_generated::PointerButtonArgs {
        kind: PointerButtonKind::Right,
        other: 4,
      },
    );
    let screen = PanelPoint::new(0.0, 0.0);
    let world = Vector3d::new(0.0, 0.0, 0.0);
    let payload = wire::PointerButtonAction::create(
      &mut builder,
      &wire::PointerButtonActionArgs {
        object_id: Some(&OBJECT_ID),
        pointer_id: 0,
        screen_position: Some(&screen),
        world_hit: Some(&world),
        button: Some(button),
      },
    );
    let noncanonical = action_bytes(
      builder,
      wire::CoreActionKind::PointerDown,
      wire::CoreActionBody::PointerButtonAction,
      payload.as_union_value(),
    );
    assert!(CoreClientMessageView::read(&noncanonical).is_err());
  }

  #[test]
  fn rejects_truncation_prefix_mismatch_and_wrong_identifier() {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let preference = wire::ReducedMotionPreferenceAction::create(
      &mut builder,
      &wire::ReducedMotionPreferenceActionArgs {
        value: WireReducedMotionPreference::Reduce,
      },
    );
    let mut bytes = action_bytes(
      builder,
      wire::CoreActionKind::ReducedMotionPreferenceChanged,
      wire::CoreActionBody::ReducedMotionPreferenceAction,
      preference.as_union_value(),
    );
    assert!(CoreClientMessageView::read(&bytes).is_ok());
    assert!(CoreClientMessageView::read(&bytes[..bytes.len() - 1]).is_err());
    bytes[0] ^= 1;
    assert!(CoreClientMessageView::read(&bytes).is_err());
    bytes[0] ^= 1;
    bytes[8] ^= 1;
    assert!(CoreClientMessageView::read(&bytes).is_err());
  }

  #[test]
  fn reads_and_semantically_checks_motion_batches() {
    let bytes = motion_action_bytes(4, 4);
    let CoreClientMessageView::Action(action) = CoreClientMessageView::read(&bytes).unwrap() else {
      panic!("expected action");
    };
    let CoreActionBodyView::MotionEvents(batch) = action.body() else {
      panic!("expected Motion batch");
    };
    assert_eq!(batch.first_sequence(), 4);
    assert_eq!(batch.last_sequence(), 4);
    assert_eq!(batch.lifecycle_count(), 0);
    assert_eq!(batch.presentation_sample_count(), 0);

    assert!(CoreClientMessageView::read(&motion_action_bytes(5, 4)).is_err());
  }

  #[test]
  fn round_trips_motion_sequence_label_events() {
    let playback_id = battlement::ObjectId::new_v4();
    let expected = battlement::MotionEventBatch {
      first_sequence: battlement::MotionSequence(0),
      last_sequence: battlement::MotionSequence(0),
      events: Vec::new(),
      samples: Vec::new(),
      value_samples: Vec::new(),
      playback_events: Vec::new(),
      label_events: vec![battlement::MotionSequenceLabelEvent {
        playback_id,
        generation: 7,
        label: "settled".to_owned(),
      }],
      gesture_events: Vec::new(),
    };
    let action = battlement::Action::new(
      battlement::ActionId::new_v4(),
      battlement::SessionId::new_v4(),
      battlement::ActionBody::MotionEvents(expected.clone()),
    );
    let bytes = write_core_action(&action).expect("encode motion label event");

    let CoreClientMessageView::Action(action) =
      CoreClientMessageView::read(bytes.as_bytes()).expect("decode motion label event")
    else {
      panic!("expected action");
    };
    let CoreActionBodyView::MotionEvents(actual) = action.body() else {
      panic!("expected Motion batch");
    };
    assert_eq!(actual.to_owned(), expected);
  }

  #[test]
  fn round_trips_presentation_work_geometry() {
    let observation_id = battlement::GeometryObservationId(battlement::ObjectId::new_v4());
    let expected = battlement::GeometryObservationValue {
      observation_id,
      result: battlement::GeometryObservationResult::Current(
        battlement::GeometryValue::PresentationWork(battlement::PresentationWorkGeometry {
          queued_batches: 7,
          blocking_operations: 2,
          paused_scopes: 1,
        }),
      ),
    };
    let action = battlement::Action::new(
      battlement::ActionId::new_v4(),
      battlement::SessionId::new_v4(),
      battlement::ActionBody::GeometryObservations(battlement::GeometryObservationBatch {
        generation: battlement::GeometryGeneration(std::num::NonZeroU64::new(9).unwrap()),
        changed: vec![expected],
      }),
    );
    let bytes = write_core_action(&action).expect("encode presentation work geometry");

    let CoreClientMessageView::Action(action) =
      CoreClientMessageView::read(bytes.as_bytes()).expect("decode presentation work geometry")
    else {
      panic!("expected action");
    };
    let CoreActionBodyView::GeometryObservations(batch) = action.body() else {
      panic!("expected geometry batch");
    };
    assert_eq!(batch.generation(), 9);
    assert_eq!(batch.changed_count(), 1);
    assert_eq!(batch.changed(0).unwrap().copy_for_retention(), expected);
  }

  fn motion_action_bytes(first_sequence: u64, last_sequence: u64) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let events = builder.create_vector_from_iter(std::iter::empty::<
      flatbuffers::WIPOffset<motion_wire::MotionLifecycleEvent<'_>>,
    >());
    let samples = builder.create_vector_from_iter(std::iter::empty::<
      flatbuffers::WIPOffset<motion_wire::MotionPresentationSample<'_>>,
    >());
    let value_samples = builder.create_vector_from_iter(std::iter::empty::<
      flatbuffers::WIPOffset<motion_wire::MotionValueSample<'_>>,
    >());
    let playback_events = builder.create_vector_from_iter(std::iter::empty::<
      flatbuffers::WIPOffset<motion_wire::MotionPlaybackEvent<'_>>,
    >());
    let gesture_events = builder.create_vector_from_iter(std::iter::empty::<
      flatbuffers::WIPOffset<motion_wire::MotionGestureEvent<'_>>,
    >());
    let label_events = builder.create_vector_from_iter(std::iter::empty::<
      flatbuffers::WIPOffset<motion_wire::MotionSequenceLabelEvent<'_>>,
    >());
    let batch = motion_wire::MotionEventBatch::create(
      &mut builder,
      &motion_wire::MotionEventBatchArgs {
        first_sequence,
        last_sequence,
        events: Some(events),
        samples: Some(samples),
        value_samples: Some(value_samples),
        playback_events: Some(playback_events),
        gesture_events: Some(gesture_events),
        label_events: Some(label_events),
      },
    );
    let motion =
      wire::MotionAction::create(&mut builder, &wire::MotionActionArgs { value: Some(batch) });
    action_bytes(
      builder,
      wire::CoreActionKind::MotionEvents,
      wire::CoreActionBody::MotionAction,
      motion.as_union_value(),
    )
  }

  fn action_bytes(
    mut builder: flatbuffers::FlatBufferBuilder<'static>,
    kind: wire::CoreActionKind,
    body_type: wire::CoreActionBody,
    body: flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
  ) -> Vec<u8> {
    let action = wire::CoreAction::create(
      &mut builder,
      &wire::CoreActionArgs {
        action_id: Some(&ACTION_ID),
        session_id: Some(&SESSION_ID),
        kind,
        body_type,
        body: Some(body),
      },
    );
    let root = wire::CoreClientMessage::create(
      &mut builder,
      &wire::CoreClientMessageArgs {
        body_type: wire::CoreClientMessageBody::CoreAction,
        body: Some(action.as_union_value()),
      },
    );
    wire::finish_size_prefixed_core_client_message_buffer(&mut builder, root);
    builder.finished_data().to_vec()
  }
}
