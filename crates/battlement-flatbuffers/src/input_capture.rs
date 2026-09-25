use battlement::{
  ControllerButton, ControllerButtonPayload, ControllerDirection, ControllerNavigationPayload,
  ControllerNavigationSource, InputCaptureCancellation, InputCaptureCommand, InputCaptureDevice,
  InputCaptureEvent, InputCaptureRequest, InputCaptureResult, ObjectId,
};
use flatbuffers::{FlatBufferBuilder, WIPOffset};

use crate::{
  ProtocolError, client_message_generated as wire, command_core_generated as command,
  common_generated::Uuid, ui_event_body, ui_event_generated::PhysicalKey,
};

pub(crate) fn write_event<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: InputCaptureEvent,
) -> Result<WIPOffset<wire::InputCaptureAction<'a>>, ProtocolError> {
  let id = Uuid(*value.id.as_uuid().as_bytes());
  let mut args = wire::InputCaptureActionArgs {
    capture_id: Some(&id),
    ..Default::default()
  };
  match value.result {
    InputCaptureResult::Key { device_id, key } => {
      args.result = wire::InputCaptureResultKind::Key;
      args.device_id = device_id;
      args.key = PhysicalKey(key as u16);
    }
    InputCaptureResult::Button(value) => {
      args.result = wire::InputCaptureResultKind::Button;
      args.device_id = value.controller_id;
      args.button = wire::ControllerButton(value.button as u8);
    }
    InputCaptureResult::Direction(value) => {
      if value.source != ControllerNavigationSource::Dpad || value.repeat {
        return Err(ProtocolError::new(
          "capture directions must be initial D-pad presses",
        ));
      }
      args.result = wire::InputCaptureResultKind::Direction;
      args.device_id = value.controller_id;
      args.direction = wire::ControllerDirection(value.direction as u8);
    }
    InputCaptureResult::Cancelled(reason) => {
      args.result = wire::InputCaptureResultKind::Cancelled;
      args.cancellation = wire::InputCaptureCancellation(reason as u8);
    }
  }
  if args.device_id < 0 {
    return Err(ProtocolError::new(
      "capture device identity must be nonnegative",
    ));
  }
  Ok(wire::InputCaptureAction::create(builder, &args))
}

pub(crate) fn read_event(
  value: wire::InputCaptureAction<'_>,
) -> Result<InputCaptureEvent, ProtocolError> {
  let id = ObjectId::from_bytes(value.capture_id().0)
    .map_err(|_| ProtocolError::new("capture identity must be nonzero"))?;
  if value.device_id() < 0 {
    return Err(ProtocolError::new(
      "capture device identity must be nonnegative",
    ));
  }
  let result = match value.result() {
    wire::InputCaptureResultKind::Key => InputCaptureResult::Key {
      device_id: value.device_id(),
      key: ui_event_body::physical_key(value.key())?,
    },
    wire::InputCaptureResultKind::Button => InputCaptureResult::Button(ControllerButtonPayload {
      controller_id: value.device_id(),
      button: match value.button() {
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
        _ => return Err(ProtocolError::new("unknown capture button")),
      },
    }),
    wire::InputCaptureResultKind::Direction => {
      if value.source() != wire::ControllerNavigationSource::Dpad || value.repeat() {
        return Err(ProtocolError::new(
          "capture directions must be initial D-pad presses",
        ));
      }
      InputCaptureResult::Direction(ControllerNavigationPayload {
        controller_id: value.device_id(),
        source: ControllerNavigationSource::Dpad,
        repeat: false,
        direction: match value.direction() {
          wire::ControllerDirection::Left => ControllerDirection::Left,
          wire::ControllerDirection::Right => ControllerDirection::Right,
          wire::ControllerDirection::Up => ControllerDirection::Up,
          wire::ControllerDirection::Down => ControllerDirection::Down,
          _ => return Err(ProtocolError::new("unknown capture direction")),
        },
      })
    }
    wire::InputCaptureResultKind::Cancelled => {
      InputCaptureResult::Cancelled(match value.cancellation() {
        wire::InputCaptureCancellation::FocusLost => InputCaptureCancellation::FocusLost,
        wire::InputCaptureCancellation::DeviceDisconnected => {
          InputCaptureCancellation::DeviceDisconnected
        }
        wire::InputCaptureCancellation::Superseded => InputCaptureCancellation::Superseded,
        _ => return Err(ProtocolError::new("unknown capture cancellation")),
      })
    }
    _ => return Err(ProtocolError::new("unknown capture result")),
  };
  Ok(InputCaptureEvent { id, result })
}

pub(crate) fn write_command<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: InputCaptureCommand,
) -> WIPOffset<command::InputCapturePayload<'a>> {
  let (id, operation) = match value {
    InputCaptureCommand::Begin(request) => (
      request.id,
      match request.device {
        InputCaptureDevice::Keyboard => command::InputCaptureOperation::BeginKeyboard,
        InputCaptureDevice::Controller => command::InputCaptureOperation::BeginController,
      },
    ),
    InputCaptureCommand::End(id) => (id, command::InputCaptureOperation::End),
  };
  let id = Uuid(*id.as_uuid().as_bytes());
  command::InputCapturePayload::create(
    builder,
    &command::InputCapturePayloadArgs {
      capture_id: Some(&id),
      operation,
    },
  )
}

pub(crate) fn read_command(
  value: command::InputCapturePayload<'_>,
) -> Result<InputCaptureCommand, ProtocolError> {
  let id = ObjectId::from_bytes(value.capture_id().0)
    .map_err(|_| ProtocolError::new("capture identity must be nonzero"))?;
  match value.operation() {
    command::InputCaptureOperation::BeginKeyboard => {
      Ok(InputCaptureCommand::Begin(InputCaptureRequest {
        id,
        device: InputCaptureDevice::Keyboard,
      }))
    }
    command::InputCaptureOperation::BeginController => {
      Ok(InputCaptureCommand::Begin(InputCaptureRequest {
        id,
        device: InputCaptureDevice::Controller,
      }))
    }
    command::InputCaptureOperation::End => Ok(InputCaptureCommand::End(id)),
    _ => Err(ProtocolError::new("unknown capture operation")),
  }
}
