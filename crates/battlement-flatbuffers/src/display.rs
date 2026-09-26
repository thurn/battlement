use battlement::{
  CommandId,
  display::DisplayCommand,
  host_settings::{DisplayConfiguration, DisplayMode, DisplayResolution},
};
use flatbuffers::{FlatBufferBuilder, WIPOffset};
use uuid::Uuid;

use crate::{ProtocolError, command_core_generated as wire, common_generated as common};

pub(crate) fn write<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: DisplayCommand,
) -> Result<WIPOffset<wire::DisplayCommandPayload<'a>>, ProtocolError> {
  if !value.is_valid() {
    return Err(ProtocolError::new("invalid display configuration"));
  }
  let (operation, configuration, preview_id) = match value {
    DisplayCommand::Preview(value) => (
      wire::DisplayOperation::Preview,
      Some(common::DisplayConfiguration::create(
        builder,
        &common::DisplayConfigurationArgs {
          mode: common::DisplayMode(value.mode as u8),
          resolution: Some(&common::DisplayResolution::new(
            value.resolution.width,
            value.resolution.height,
            value.resolution.refresh_numerator,
            value.resolution.refresh_denominator,
          )),
        },
      )),
      None,
    ),
    DisplayCommand::Confirm(id) => (
      wire::DisplayOperation::Confirm,
      None,
      Some(common::Uuid(*id.as_uuid().as_bytes())),
    ),
    DisplayCommand::Cancel(id) => (
      wire::DisplayOperation::Cancel,
      None,
      Some(common::Uuid(*id.as_uuid().as_bytes())),
    ),
  };
  Ok(wire::DisplayCommandPayload::create(
    builder,
    &wire::DisplayCommandPayloadArgs {
      operation,
      configuration,
      preview_id: preview_id.as_ref(),
    },
  ))
}

pub(crate) fn read(
  value: wire::DisplayCommandPayload<'_>,
) -> Result<DisplayCommand, ProtocolError> {
  let invalid = || ProtocolError::new("invalid display command payload");
  let result = match value.operation() {
    wire::DisplayOperation::Preview => {
      if value.preview_id().is_some() {
        return Err(invalid());
      }
      let configuration = value.configuration().ok_or_else(invalid)?;
      let resolution = configuration.resolution();
      DisplayCommand::Preview(DisplayConfiguration {
        mode: match configuration.mode().0 {
          0 => DisplayMode::Windowed,
          1 => DisplayMode::Borderless,
          2 => DisplayMode::Fullscreen,
          _ => return Err(invalid()),
        },
        resolution: DisplayResolution {
          width: resolution.width(),
          height: resolution.height(),
          refresh_numerator: resolution.refresh_numerator(),
          refresh_denominator: resolution.refresh_denominator(),
        },
      })
    }
    operation @ (wire::DisplayOperation::Confirm | wire::DisplayOperation::Cancel) => {
      if value.configuration().is_some() {
        return Err(invalid());
      }
      let id = CommandId::from_uuid(Uuid::from_bytes(value.preview_id().ok_or_else(invalid)?.0))
        .map_err(|_| invalid())?;
      if operation == wire::DisplayOperation::Confirm {
        DisplayCommand::Confirm(id)
      } else {
        DisplayCommand::Cancel(id)
      }
    }
    _ => return Err(invalid()),
  };
  if !result.is_valid() {
    return Err(invalid());
  }
  Ok(result)
}
