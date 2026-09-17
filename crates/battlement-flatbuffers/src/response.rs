use battlement::{Command, CommandBody};
#[cfg(any(test, feature = "test-support"))]
use battlement::{Response, ResponseMessage, Validate};
use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, VerifierOptions, WIPOffset};

use crate::{
  FinishedMessage, MAXIMUM_APPARENT_BYTES, MAXIMUM_MESSAGE_BYTES, MAXIMUM_TABLE_DEPTH,
  MAXIMUM_TABLE_VISITS, ProtocolError, accessibility_generated as accessibility_wire,
  client_message_generated::battlement::flat_buffers::generated as client_wire,
  command_core_generated as command_wire, common_generated as common,
  geometry_generated as geometry_wire,
  response_generated::battlement::flat_buffers::generated as wire,
  ui_event_generated as ui_event_wire, ui_generated as ui_wire, world_generated as world_wire,
};

/// A structurally verified, semantically checked borrowed response.
#[derive(Clone, Copy)]
pub struct ResponseView<'a> {
  value: wire::Response<'a>,
}

impl<'a> ResponseView<'a> {
  /// Verifies and validates one complete size-prefixed response.
  pub fn read(bytes: &'a [u8]) -> Result<Self, ProtocolError> {
    if bytes.len() > MAXIMUM_MESSAGE_BYTES {
      return Err(ProtocolError::new("response exceeds 16 MiB"));
    }
    if bytes.len() < 4
      || usize::try_from(u32::from_le_bytes(
        bytes[..4].try_into().expect("length checked"),
      ))
      .ok()
        != bytes.len().checked_sub(4)
    {
      return Err(ProtocolError::new(
        "response size prefix does not match its finished range",
      ));
    }
    if bytes.len() < 12 || !wire::response_size_prefixed_buffer_has_identifier(bytes) {
      return Err(ProtocolError::new("response has the wrong file identifier"));
    }
    let value = wire::size_prefixed_root_as_response_with_opts(&verifier_options(), bytes)
      .map_err(|error| ProtocolError::new(format!("invalid response FlatBuffer: {error}")))?;
    validate_response(value)?;
    Ok(Self { value })
  }

  /// Returns the canonical response session UUID bytes.
  #[must_use]
  pub fn session_id(self) -> [u8; 16] {
    std::array::from_fn(|index| self.value.session_id().bytes().get(index))
  }

  /// Returns the number of ordered response messages.
  #[must_use]
  pub fn message_count(self) -> usize {
    self.value.messages().len()
  }
}

/// Constructs an empty response directly into one size-prefixed FlatBuffer.
pub fn write_empty_response(session_id: [u8; 16]) -> Result<FinishedMessage, ProtocolError> {
  if session_id.iter().all(|byte| *byte == 0) {
    return Err(ProtocolError::new("response session UUID is zero"));
  }
  let mut builder = FlatBufferBuilder::with_capacity(256);
  let messages =
    builder.create_vector::<flatbuffers::WIPOffset<wire::ResponseMessageEntry<'_>>>(&[]);
  let session_id = common::Uuid::new(&session_id);
  let response = wire::Response::create(
    &mut builder,
    &wire::ResponseArgs {
      session_id: Some(&session_id),
      messages: Some(messages),
    },
  );
  wire::finish_size_prefixed_response_buffer(&mut builder, response);
  let (storage, start) = builder.collapse();
  let message = FinishedMessage::from_storage(storage, start);
  if message.as_bytes().len() > MAXIMUM_MESSAGE_BYTES {
    return Err(ProtocolError::new("response exceeds 16 MiB"));
  }
  ResponseView::read(message.as_bytes())?;
  Ok(message)
}

/// Constructs a core response directly into one size-prefixed FlatBuffer.
///
/// This owned-model fixture writer is available only to transport tests.
#[cfg(any(test, feature = "test-support"))]
pub(crate) fn write_core_response(
  value: &Response<Command>,
) -> Result<FinishedMessage, ProtocolError> {
  write_core_response_impl(value, true)
}

#[cfg(feature = "test-support")]
pub(crate) fn write_unchecked_core_response(
  value: &Response<Command>,
) -> Result<FinishedMessage, ProtocolError> {
  write_core_response_impl(value, false)
}

#[cfg(any(test, feature = "test-support"))]
fn write_core_response_impl(
  value: &Response<Command>,
  validate_domain: bool,
) -> Result<FinishedMessage, ProtocolError> {
  if validate_domain {
    for message in &value.messages {
      match message {
        ResponseMessage::Snapshot(snapshot) => snapshot
          .validate()
          .map_err(|failure| ProtocolError::new(format!("invalid response snapshot: {failure}")))?,
        ResponseMessage::Batch(batch) => {
          for group in &batch.groups {
            for command in &group.commands {
              command.validate().map_err(|failure| {
                ProtocolError::new(format!("invalid response command: {failure}"))
              })?;
            }
          }
        }
      }
    }
  }
  let mut builder = FlatBufferBuilder::with_capacity(4 * 1024);
  let mut messages = Vec::with_capacity(value.messages.len());
  for message in &value.messages {
    let (message_type, message) = match message {
      ResponseMessage::Batch(batch) => {
        let mut groups = Vec::with_capacity(batch.groups.len());
        for group in &batch.groups {
          let commands = group
            .commands
            .iter()
            .map(|command| {
              let command = write_command(&mut builder, command)?;
              Ok(wire::CommandEntry::create(
                &mut builder,
                &wire::CommandEntryArgs {
                  command_type: wire::CommandEntryPayload::CoreCommand,
                  command: Some(command.as_union_value()),
                },
              ))
            })
            .collect::<Result<Vec<_>, _>>()?;
          let commands = builder.create_vector(&commands);
          groups.push(wire::ParallelCommandGroup::create(
            &mut builder,
            &wire::ParallelCommandGroupArgs {
              commands: Some(commands),
            },
          ));
        }
        let groups = builder.create_vector(&groups);
        let batch_id = uuid(batch.batch_id.as_uuid());
        let session_id = uuid(batch.session_id.as_uuid());
        let caused_by_action_id = batch
          .caused_by_action_id
          .as_ref()
          .map(|value| uuid(value.as_uuid()));
        let batch = wire::Batch::create(
          &mut builder,
          &wire::BatchArgs {
            batch_id: Some(&batch_id),
            session_id: Some(&session_id),
            caused_by_action_id: caused_by_action_id.as_ref(),
            work_scope: batch.work_scope,
            cancel_scope: batch.cancel_scope,
            start: match batch.start {
              battlement::BatchStart::Now => wire::BatchStart::Now,
              battlement::BatchStart::AfterEarlierBlockingWork => {
                wire::BatchStart::AfterEarlierBlockingWork
              }
              battlement::BatchStart::AfterEarlierAssetPreparation => {
                wire::BatchStart::AfterEarlierAssetPreparation
              }
            },
            groups: Some(groups),
          },
        );
        (wire::ResponseMessage::Batch, batch.as_union_value())
      }
      ResponseMessage::Snapshot(snapshot) => {
        let snapshot = write_snapshot(&mut builder, snapshot)?;
        (wire::ResponseMessage::Snapshot, snapshot.as_union_value())
      }
    };
    messages.push(wire::ResponseMessageEntry::create(
      &mut builder,
      &wire::ResponseMessageEntryArgs {
        message_type,
        message: Some(message),
      },
    ));
  }
  let messages = builder.create_vector(&messages);
  let session_id = uuid(value.session_id.as_uuid());
  let response = wire::Response::create(
    &mut builder,
    &wire::ResponseArgs {
      session_id: Some(&session_id),
      messages: Some(messages),
    },
  );
  wire::finish_size_prefixed_response_buffer(&mut builder, response);
  finish(builder)
}

/// Writes one core command into a build-composed response builder.
///
/// The returned offset is type-erased only at the union boundary; the payload
/// itself is the generated, closed core command table.
#[doc(hidden)]
pub fn write_composed_core_command<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  command: &Command,
) -> Result<WIPOffset<UnionWIPOffset>, ProtocolError> {
  Ok(write_command(builder, command)?.as_union_value())
}

/// Writes one core snapshot into a build-composed response builder.
#[doc(hidden)]
pub fn write_composed_snapshot<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  snapshot: &battlement::Snapshot,
) -> Result<WIPOffset<UnionWIPOffset>, ProtocolError> {
  Ok(write_snapshot(builder, snapshot)?.as_union_value())
}

pub(crate) fn write_command<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  command: &Command,
) -> Result<WIPOffset<wire::CoreCommand<'a>>, ProtocolError> {
  let (kind, payload_type, payload): (_, _, WIPOffset<UnionWIPOffset>) = match &command.body {
    CommandBody::ApplicationOpenUrl(value) => {
      let url = builder.create_string(&value.url);
      let payload = command_wire::ExternalUrlPayload::create(
        builder,
        &command_wire::ExternalUrlPayloadArgs { url: Some(url) },
      );
      (
        wire::CoreCommandKind::ApplicationOpenUrl,
        wire::CoreCommandPayload::ExternalUrlPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ObjectDestroy(value) => {
      let object_id = uuid(value.object_id.as_uuid());
      let payload = command_wire::ObjectIdPayload::create(
        builder,
        &command_wire::ObjectIdPayloadArgs {
          object_id: Some(&object_id),
        },
      );
      (
        wire::CoreCommandKind::ObjectDestroy,
        wire::CoreCommandPayload::ObjectIdPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::Diagnostics(value) => {
      let battlement_cloud::diagnostics::DiagnosticsCommand::SetMetadata(value) = value;
      let key = builder.create_string(&value.key);
      let metadata_value = value
        .value
        .as_ref()
        .map(|value| builder.create_string(value));
      let payload = command_wire::DiagnosticsPayload::create(
        builder,
        &command_wire::DiagnosticsPayloadArgs {
          key: Some(key),
          value: metadata_value,
        },
      );
      (
        wire::CoreCommandKind::Diagnostics,
        wire::CoreCommandPayload::DiagnosticsPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AssetsReplaceSet(value) => {
      let assets = value
        .assets
        .iter()
        .map(|value| write_prepared_asset(builder, value))
        .collect::<Vec<_>>();
      let assets = builder.create_vector(&assets);
      let payload = command_wire::ReplaceAssetSetPayload::create(
        builder,
        &command_wire::ReplaceAssetSetPayloadArgs {
          assets: Some(assets),
        },
      );
      (
        wire::CoreCommandKind::AssetsReplaceSet,
        wire::CoreCommandPayload::ReplaceAssetSetPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::SceneLoad(value) => {
      let address = builder.create_string(value.address.as_str());
      let scene_id = uuid(value.scene_id.as_uuid());
      let payload = command_wire::SceneLoadPayload::create(
        builder,
        &command_wire::SceneLoadPayloadArgs {
          scene_id: Some(&scene_id),
          address: Some(address),
          make_primary: value.make_primary,
        },
      );
      (
        wire::CoreCommandKind::SceneLoad,
        wire::CoreCommandPayload::SceneLoadPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::SceneUnload(value) | CommandBody::SceneSetPrimary(value) => {
      let scene_id = uuid(value.scene_id.as_uuid());
      let payload = command_wire::SceneIdPayload::create(
        builder,
        &command_wire::SceneIdPayloadArgs {
          scene_id: Some(&scene_id),
        },
      );
      (
        if matches!(&command.body, CommandBody::SceneUnload(_)) {
          wire::CoreCommandKind::SceneUnload
        } else {
          wire::CoreCommandKind::SceneSetPrimary
        },
        wire::CoreCommandPayload::SceneIdPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ObjectCreate(body) => {
      let object = write_game_object(builder, &body.object)?;
      let payload = command_wire::ObjectCreatePayload::create(
        builder,
        &command_wire::ObjectCreatePayloadArgs {
          object: Some(object),
        },
      );
      (
        wire::CoreCommandKind::ObjectCreate,
        wire::CoreCommandPayload::ObjectCreatePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ObjectSetActive(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::ObjectSetActivePayload::create(
        builder,
        &command_wire::ObjectSetActivePayloadArgs {
          object_id: Some(&object_id),
          active: body.active,
        },
      );
      (
        wire::CoreCommandKind::ObjectSetActive,
        wire::CoreCommandPayload::ObjectSetActivePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ObjectReparent(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let parent_id = body.parent_id.as_ref().map(|value| uuid(value.as_uuid()));
      let payload = command_wire::ObjectReparentPayload::create(
        builder,
        &command_wire::ObjectReparentPayloadArgs {
          object_id: Some(&object_id),
          parent_id: parent_id.as_ref(),
          world_position_stays: body.world_position_stays,
        },
      );
      (
        wire::CoreCommandKind::ObjectReparent,
        wire::CoreCommandPayload::ObjectReparentPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TransformSetLocalPosition(body) | CommandBody::TransformSetWorldPosition(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let position = vector3(body.payload.position);
      let payload = command_wire::PositionPayload::create(
        builder,
        &command_wire::PositionPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          position: Some(&position),
        },
      );
      (
        if matches!(&command.body, CommandBody::TransformSetLocalPosition(_)) {
          wire::CoreCommandKind::TransformSetLocalPosition
        } else {
          wire::CoreCommandKind::TransformSetWorldPosition
        },
        wire::CoreCommandPayload::PositionPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TransformTweenLocalPosition(body)
    | CommandBody::TransformTweenWorldPosition(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let position = vector3(body.payload.position);
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenPositionPayload::create(
        builder,
        &command_wire::TweenPositionPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          position: Some(&position),
          tween: Some(&tween),
        },
      );
      (
        if matches!(&command.body, CommandBody::TransformTweenLocalPosition(_)) {
          wire::CoreCommandKind::TransformTweenLocalPosition
        } else {
          wire::CoreCommandKind::TransformTweenWorldPosition
        },
        wire::CoreCommandPayload::TweenPositionPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TransformSetLocalRotation(body) | CommandBody::TransformSetWorldRotation(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let rotation = quaternion(body.payload.rotation);
      let payload = command_wire::RotationPayload::create(
        builder,
        &command_wire::RotationPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          rotation: Some(&rotation),
        },
      );
      (
        if matches!(&command.body, CommandBody::TransformSetLocalRotation(_)) {
          wire::CoreCommandKind::TransformSetLocalRotation
        } else {
          wire::CoreCommandKind::TransformSetWorldRotation
        },
        wire::CoreCommandPayload::RotationPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TransformTweenLocalRotation(body)
    | CommandBody::TransformTweenWorldRotation(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let rotation = quaternion(body.payload.rotation);
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenRotationPayload::create(
        builder,
        &command_wire::TweenRotationPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          rotation: Some(&rotation),
          tween: Some(&tween),
        },
      );
      (
        if matches!(&command.body, CommandBody::TransformTweenLocalRotation(_)) {
          wire::CoreCommandKind::TransformTweenLocalRotation
        } else {
          wire::CoreCommandKind::TransformTweenWorldRotation
        },
        wire::CoreCommandPayload::TweenRotationPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TransformSetLocalScale(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let scale = vector3(body.payload.scale);
      let payload = command_wire::ScalePayload::create(
        builder,
        &command_wire::ScalePayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          scale: Some(&scale),
        },
      );
      (
        wire::CoreCommandKind::TransformSetLocalScale,
        wire::CoreCommandPayload::ScalePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TransformTweenLocalScale(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let scale = vector3(body.payload.scale);
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenScalePayload::create(
        builder,
        &command_wire::TweenScalePayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          scale: Some(&scale),
          tween: Some(&tween),
        },
      );
      (
        wire::CoreCommandKind::TransformTweenLocalScale,
        wire::CoreCommandPayload::TweenScalePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::RendererSetMaterial(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let address = builder.create_string(body.payload.address.as_str());
      let payload = command_wire::SetMaterialPayload::create(
        builder,
        &command_wire::SetMaterialPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          address: Some(address),
          slot: body.payload.slot,
        },
      );
      (
        wire::CoreCommandKind::RendererSetMaterial,
        wire::CoreCommandPayload::SetMaterialPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::CameraSetEnabled(body)
    | CommandBody::LightSetEnabled(body)
    | CommandBody::ImageSetFaceCamera(body)
    | CommandBody::TextSetRichText(body)
    | CommandBody::TextSetFaceCamera(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::ObjectEnabledPayload::create(
        builder,
        &command_wire::ObjectEnabledPayloadArgs {
          object_id: Some(&object_id),
          enabled: body.enabled,
        },
      );
      let kind = match &command.body {
        CommandBody::CameraSetEnabled(_) => wire::CoreCommandKind::CameraSetEnabled,
        CommandBody::LightSetEnabled(_) => wire::CoreCommandKind::LightSetEnabled,
        CommandBody::ImageSetFaceCamera(_) => wire::CoreCommandKind::ImageSetFaceCamera,
        CommandBody::TextSetRichText(_) => wire::CoreCommandKind::TextSetRichText,
        CommandBody::TextSetFaceCamera(_) => wire::CoreCommandKind::TextSetFaceCamera,
        _ => unreachable!(),
      };
      (
        kind,
        wire::CoreCommandPayload::ObjectEnabledPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::CameraSetPerspective(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let payload = command_wire::PerspectivePayload::create(
        builder,
        &command_wire::PerspectivePayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          field_of_view: body.payload.field_of_view,
        },
      );
      (
        wire::CoreCommandKind::CameraSetPerspective,
        wire::CoreCommandPayload::PerspectivePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::CameraTweenFieldOfView(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenFieldOfViewPayload::create(
        builder,
        &command_wire::TweenFieldOfViewPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          field_of_view: body.payload.field_of_view,
          tween: Some(&tween),
        },
      );
      (
        wire::CoreCommandKind::CameraTweenFieldOfView,
        wire::CoreCommandPayload::TweenFieldOfViewPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::CameraSetOrthographic(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let payload = command_wire::OrthographicPayload::create(
        builder,
        &command_wire::OrthographicPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          size: body.payload.size,
        },
      );
      (
        wire::CoreCommandKind::CameraSetOrthographic,
        wire::CoreCommandPayload::OrthographicPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::CameraTweenOrthographicSize(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenOrthographicSizePayload::create(
        builder,
        &command_wire::TweenOrthographicSizePayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          size: body.payload.size,
          tween: Some(&tween),
        },
      );
      (
        wire::CoreCommandKind::CameraTweenOrthographicSize,
        wire::CoreCommandPayload::TweenOrthographicSizePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::CameraSetClipping(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::CameraClippingPayload::create(
        builder,
        &command_wire::CameraClippingPayloadArgs {
          object_id: Some(&object_id),
          near: body.near,
          far: body.far,
        },
      );
      (
        wire::CoreCommandKind::CameraSetClipping,
        wire::CoreCommandPayload::CameraClippingPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::CameraSetClear(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let clear_color = body.clear_color.map(color);
      let payload = command_wire::CameraClearPayload::create(
        builder,
        &command_wire::CameraClearPayloadArgs {
          object_id: Some(&object_id),
          clear_mode: camera_clear_mode(body.clear_mode),
          clear_color: clear_color.as_ref(),
        },
      );
      (
        wire::CoreCommandKind::CameraSetClear,
        wire::CoreCommandPayload::CameraClearPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::LightSetType(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::LightTypePayload::create(
        builder,
        &command_wire::LightTypePayloadArgs {
          object_id: Some(&object_id),
          light_type: light_type(body.light_type),
        },
      );
      (
        wire::CoreCommandKind::LightSetType,
        wire::CoreCommandPayload::LightTypePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::LightSetColor(body) | CommandBody::TextSetColor(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let color = color(body.payload.color);
      let payload = command_wire::ColorPayload::create(
        builder,
        &command_wire::ColorPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          color: Some(&color),
        },
      );
      (
        if matches!(&command.body, CommandBody::LightSetColor(_)) {
          wire::CoreCommandKind::LightSetColor
        } else {
          wire::CoreCommandKind::TextSetColor
        },
        wire::CoreCommandPayload::ColorPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::LightTweenColor(body) | CommandBody::TextTweenColor(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let color = color(body.payload.color);
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenColorPayload::create(
        builder,
        &command_wire::TweenColorPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          color: Some(&color),
          tween: Some(&tween),
        },
      );
      (
        if matches!(&command.body, CommandBody::LightTweenColor(_)) {
          wire::CoreCommandKind::LightTweenColor
        } else {
          wire::CoreCommandKind::TextTweenColor
        },
        wire::CoreCommandPayload::TweenColorPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::LightSetIntensity(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let payload = command_wire::IntensityPayload::create(
        builder,
        &command_wire::IntensityPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          intensity: body.payload.intensity,
        },
      );
      (
        wire::CoreCommandKind::LightSetIntensity,
        wire::CoreCommandPayload::IntensityPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::LightTweenIntensity(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenIntensityPayload::create(
        builder,
        &command_wire::TweenIntensityPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          intensity: body.payload.intensity,
          tween: Some(&tween),
        },
      );
      (
        wire::CoreCommandKind::LightTweenIntensity,
        wire::CoreCommandPayload::TweenIntensityPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::LightSetRange(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::LightRangePayload::create(
        builder,
        &command_wire::LightRangePayloadArgs {
          object_id: Some(&object_id),
          range: body.range,
        },
      );
      (
        wire::CoreCommandKind::LightSetRange,
        wire::CoreCommandPayload::LightRangePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::LightSetSpotAngle(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::SpotAnglePayload::create(
        builder,
        &command_wire::SpotAnglePayloadArgs {
          object_id: Some(&object_id),
          outer_spot_angle: body.outer_spot_angle,
          inner_spot_angle: body.inner_spot_angle,
        },
      );
      (
        wire::CoreCommandKind::LightSetSpotAngle,
        wire::CoreCommandPayload::SpotAnglePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::LightSetShadows(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::LightShadowsPayload::create(
        builder,
        &command_wire::LightShadowsPayloadArgs {
          object_id: Some(&object_id),
          shadows: shadow_mode(body.shadows),
        },
      );
      (
        wire::CoreCommandKind::LightSetShadows,
        wire::CoreCommandPayload::LightShadowsPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ImageSetTexture(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let address = builder.create_string(body.address.as_str());
      let payload = command_wire::SetTexturePayload::create(
        builder,
        &command_wire::SetTexturePayloadArgs {
          object_id: Some(&object_id),
          address: Some(address),
        },
      );
      (
        wire::CoreCommandKind::ImageSetTexture,
        wire::CoreCommandPayload::SetTexturePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TextSetFont(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let address = builder.create_string(body.address.as_str());
      let payload = command_wire::SetFontPayload::create(
        builder,
        &command_wire::SetFontPayloadArgs {
          object_id: Some(&object_id),
          address: Some(address),
        },
      );
      (
        wire::CoreCommandKind::TextSetFont,
        wire::CoreCommandPayload::SetFontPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ImageSetSize(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::ImageSizePayload::create(
        builder,
        &command_wire::ImageSizePayloadArgs {
          object_id: Some(&object_id),
          width: body.width,
          height: body.height,
        },
      );
      (
        wire::CoreCommandKind::ImageSetSize,
        wire::CoreCommandPayload::ImageSizePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ImageSetFit(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::ImageFitPayload::create(
        builder,
        &command_wire::ImageFitPayloadArgs {
          object_id: Some(&object_id),
          fit: image_fit(body.fit),
        },
      );
      (
        wire::CoreCommandKind::ImageSetFit,
        wire::CoreCommandPayload::ImageFitPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ImageSetTint(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let tint = rgb(body.payload.tint);
      let payload = command_wire::TintPayload::create(
        builder,
        &command_wire::TintPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          tint: Some(&tint),
        },
      );
      (
        wire::CoreCommandKind::ImageSetTint,
        wire::CoreCommandPayload::TintPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ImageTweenTint(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let tint = rgb(body.payload.tint);
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenTintPayload::create(
        builder,
        &command_wire::TweenTintPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          tint: Some(&tint),
          tween: Some(&tween),
        },
      );
      (
        wire::CoreCommandKind::ImageTweenTint,
        wire::CoreCommandPayload::TweenTintPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ImageSetOpacity(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let payload = command_wire::OpacityPayload::create(
        builder,
        &command_wire::OpacityPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          opacity: body.payload.opacity,
        },
      );
      (
        wire::CoreCommandKind::ImageSetOpacity,
        wire::CoreCommandPayload::OpacityPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ImageTweenOpacity(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenOpacityPayload::create(
        builder,
        &command_wire::TweenOpacityPayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          opacity: body.payload.opacity,
          tween: Some(&tween),
        },
      );
      (
        wire::CoreCommandKind::ImageTweenOpacity,
        wire::CoreCommandPayload::TweenOpacityPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TextSetSize(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let payload = command_wire::TextSizePayload::create(
        builder,
        &command_wire::TextSizePayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          size: body.payload.size,
        },
      );
      (
        wire::CoreCommandKind::TextSetSize,
        wire::CoreCommandPayload::TextSizePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TextTweenSize(body) => {
      let object_id = uuid(body.payload.object_id.as_uuid());
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenTextSizePayload::create(
        builder,
        &command_wire::TweenTextSizePayloadArgs {
          on_conflict: conflict(body.on_conflict),
          object_id: Some(&object_id),
          size: body.payload.size,
          tween: Some(&tween),
        },
      );
      (
        wire::CoreCommandKind::TextTweenSize,
        wire::CoreCommandPayload::TweenTextSizePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TextSetAlignment(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::TextAlignmentPayload::create(
        builder,
        &command_wire::TextAlignmentPayloadArgs {
          object_id: Some(&object_id),
          horizontal: horizontal_alignment(body.horizontal),
          vertical: vertical_alignment(body.vertical),
        },
      );
      (
        wire::CoreCommandKind::TextSetAlignment,
        wire::CoreCommandPayload::TextAlignmentPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TextSetWrapping(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::TextWrappingPayload::create(
        builder,
        &command_wire::TextWrappingPayloadArgs {
          object_id: Some(&object_id),
          wrap_width: body.wrap_width,
        },
      );
      (
        wire::CoreCommandKind::TextSetWrapping,
        wire::CoreCommandPayload::TextWrappingPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AnimatorPlay(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let state = builder.create_string(&body.state);
      let payload = command_wire::AnimatorPlayPayload::create(
        builder,
        &command_wire::AnimatorPlayPayloadArgs {
          object_id: Some(&object_id),
          state: Some(state),
          layer: body.layer,
          normalized_start_time: body.normalized_start_time,
          wait_ms: body.wait_ms,
        },
      );
      (
        wire::CoreCommandKind::AnimatorPlay,
        wire::CoreCommandPayload::AnimatorPlayPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AnimatorCrossFade(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let state = builder.create_string(&body.state);
      let payload = command_wire::AnimatorCrossFadePayload::create(
        builder,
        &command_wire::AnimatorCrossFadePayloadArgs {
          object_id: Some(&object_id),
          state: Some(state),
          layer: body.layer,
          normalized_start_time: body.normalized_start_time,
          wait_ms: body.wait_ms,
          cross_fade_ms: body.cross_fade_ms,
        },
      );
      (
        wire::CoreCommandKind::AnimatorCrossFade,
        wire::CoreCommandPayload::AnimatorCrossFadePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AnimatorSetBool(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let parameter = builder.create_string(&body.parameter);
      let payload = command_wire::AnimatorBoolPayload::create(
        builder,
        &command_wire::AnimatorBoolPayloadArgs {
          object_id: Some(&object_id),
          parameter: Some(parameter),
          value: body.value,
        },
      );
      (
        wire::CoreCommandKind::AnimatorSetBool,
        wire::CoreCommandPayload::AnimatorBoolPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AnimatorSetInt(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let parameter = builder.create_string(&body.parameter);
      let payload = command_wire::AnimatorIntPayload::create(
        builder,
        &command_wire::AnimatorIntPayloadArgs {
          object_id: Some(&object_id),
          parameter: Some(parameter),
          value: body.value,
        },
      );
      (
        wire::CoreCommandKind::AnimatorSetInt,
        wire::CoreCommandPayload::AnimatorIntPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AnimatorSetFloat(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let parameter = builder.create_string(&body.parameter);
      let payload = command_wire::AnimatorFloatPayload::create(
        builder,
        &command_wire::AnimatorFloatPayloadArgs {
          object_id: Some(&object_id),
          parameter: Some(parameter),
          value: body.value,
        },
      );
      (
        wire::CoreCommandKind::AnimatorSetFloat,
        wire::CoreCommandPayload::AnimatorFloatPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AnimatorSetTrigger(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let parameter = builder.create_string(&body.parameter);
      let payload = command_wire::AnimatorParameterPayload::create(
        builder,
        &command_wire::AnimatorParameterPayloadArgs {
          object_id: Some(&object_id),
          parameter: Some(parameter),
        },
      );
      (
        wire::CoreCommandKind::AnimatorSetTrigger,
        wire::CoreCommandPayload::AnimatorParameterPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AnimatorSetSpeed(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::AnimatorSpeedPayload::create(
        builder,
        &command_wire::AnimatorSpeedPayloadArgs {
          object_id: Some(&object_id),
          speed: body.speed,
        },
      );
      (
        wire::CoreCommandKind::AnimatorSetSpeed,
        wire::CoreCommandPayload::AnimatorSpeedPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ParticlePlay(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::ParticlePlayPayload::create(
        builder,
        &command_wire::ParticlePlayPayloadArgs {
          object_id: Some(&object_id),
          restart: body.restart,
        },
      );
      (
        wire::CoreCommandKind::ParticlePlay,
        wire::CoreCommandPayload::ParticlePlayPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ParticleStop(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = command_wire::ParticleStopPayload::create(
        builder,
        &command_wire::ParticleStopPayloadArgs {
          object_id: Some(&object_id),
          clear: body.clear,
        },
      );
      (
        wire::CoreCommandKind::ParticleStop,
        wire::CoreCommandPayload::ParticleStopPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::ParticleSpawn(body) => {
      let address = builder.create_string(body.address.as_str());
      let (location_kind, object_id, world_position) = match body.location {
        battlement::ParticleSpawnLocation::GameObject(value) => (
          command_wire::ParticleSpawnLocationKind::GameObject,
          Some(uuid(value.as_uuid())),
          None,
        ),
        battlement::ParticleSpawnLocation::WorldPosition(value) => (
          command_wire::ParticleSpawnLocationKind::WorldPosition,
          None,
          Some(vector3(value)),
        ),
      };
      let payload = command_wire::ParticleSpawnPayload::create(
        builder,
        &command_wire::ParticleSpawnPayloadArgs {
          address: Some(address),
          location_kind,
          object_id: object_id.as_ref(),
          world_position: world_position.as_ref(),
          lifetime_ms: body.lifetime_ms,
        },
      );
      (
        wire::CoreCommandKind::ParticleSpawn,
        wire::CoreCommandPayload::ParticleSpawnPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AudioPlay(body) => {
      let address = builder.create_string(body.address.as_str());
      let payload = command_wire::AudioPlayPayload::create(
        builder,
        &command_wire::AudioPlayPayloadArgs {
          address: Some(address),
          volume: body.volume,
          pitch: body.pitch,
          loop_: body.r#loop,
          fade_in_ms: body.fade_in_ms,
        },
      );
      (
        wire::CoreCommandKind::AudioPlay,
        wire::CoreCommandPayload::AudioPlayPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AudioStop(body) => {
      let audio_command_id = uuid(body.audio_command_id.as_uuid());
      let payload = command_wire::AudioStopPayload::create(
        builder,
        &command_wire::AudioStopPayloadArgs {
          audio_command_id: Some(&audio_command_id),
          fade_out_ms: body.fade_out_ms,
        },
      );
      (
        wire::CoreCommandKind::AudioStop,
        wire::CoreCommandPayload::AudioStopPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AudioPause(body) | CommandBody::AudioResume(body) => {
      let audio_command_id = uuid(body.audio_command_id.as_uuid());
      let payload = command_wire::AudioPlaybackPayload::create(
        builder,
        &command_wire::AudioPlaybackPayloadArgs {
          audio_command_id: Some(&audio_command_id),
        },
      );
      (
        if matches!(&command.body, CommandBody::AudioPause(_)) {
          wire::CoreCommandKind::AudioPause
        } else {
          wire::CoreCommandKind::AudioResume
        },
        wire::CoreCommandPayload::AudioPlaybackPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AudioSeek(body) => {
      let audio_command_id = uuid(body.audio_command_id.as_uuid());
      let payload = command_wire::AudioSeekPayload::create(
        builder,
        &command_wire::AudioSeekPayloadArgs {
          audio_command_id: Some(&audio_command_id),
          position_ms: body.position_ms,
        },
      );
      (
        wire::CoreCommandKind::AudioSeek,
        wire::CoreCommandPayload::AudioSeekPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AudioSetBuffering(body) => {
      let audio_command_id = uuid(body.audio_command_id.as_uuid());
      let payload = command_wire::AudioBufferingPayload::create(
        builder,
        &command_wire::AudioBufferingPayloadArgs {
          audio_command_id: Some(&audio_command_id),
          buffering: body.buffering,
        },
      );
      (
        wire::CoreCommandKind::AudioSetBuffering,
        wire::CoreCommandPayload::AudioBufferingPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AudioReplace(body) => {
      let audio_command_id = uuid(body.audio_command_id.as_uuid());
      let address = builder.create_string(body.address.as_str());
      let payload = command_wire::AudioReplacePayload::create(
        builder,
        &command_wire::AudioReplacePayloadArgs {
          audio_command_id: Some(&audio_command_id),
          address: Some(address),
        },
      );
      (
        wire::CoreCommandKind::AudioReplace,
        wire::CoreCommandPayload::AudioReplacePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AudioSetVolume(body) => {
      let audio_command_id = uuid(body.payload.audio_command_id.as_uuid());
      let payload = command_wire::AudioVolumePayload::create(
        builder,
        &command_wire::AudioVolumePayloadArgs {
          on_conflict: conflict(body.on_conflict),
          audio_command_id: Some(&audio_command_id),
          volume: body.payload.volume,
        },
      );
      (
        wire::CoreCommandKind::AudioSetVolume,
        wire::CoreCommandPayload::AudioVolumePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::AudioTweenVolume(body) => {
      let audio_command_id = uuid(body.payload.audio_command_id.as_uuid());
      let tween = tween(body.payload.tween);
      let payload = command_wire::TweenAudioVolumePayload::create(
        builder,
        &command_wire::TweenAudioVolumePayloadArgs {
          on_conflict: conflict(body.on_conflict),
          audio_command_id: Some(&audio_command_id),
          volume: body.payload.volume,
          tween: Some(&tween),
        },
      );
      (
        wire::CoreCommandKind::AudioTweenVolume,
        wire::CoreCommandPayload::TweenAudioVolumePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::InputSetCamera(value) => {
      let object_id = uuid(value.object_id.as_uuid());
      let payload = command_wire::ObjectIdPayload::create(
        builder,
        &command_wire::ObjectIdPayloadArgs {
          object_id: Some(&object_id),
        },
      );
      (
        wire::CoreCommandKind::InputSetCamera,
        wire::CoreCommandPayload::ObjectIdPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TextSetContent(value) => {
      let content = builder.create_string(&value.content);
      let object_id = uuid(value.object_id.as_uuid());
      let payload = command_wire::TextContentPayload::create(
        builder,
        &command_wire::TextContentPayloadArgs {
          object_id: Some(&object_id),
          content: Some(content),
        },
      );
      (
        wire::CoreCommandKind::TextSetContent,
        wire::CoreCommandPayload::TextContentPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::TimeWait(value) => {
      let payload = command_wire::WaitPayload::create(
        builder,
        &command_wire::WaitPayloadArgs {
          duration_ms: value.duration_ms,
        },
      );
      (
        wire::CoreCommandKind::TimeWait,
        wire::CoreCommandPayload::WaitPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::OperationCancel(value) => {
      let command_id = uuid(value.command_id.as_uuid());
      let payload = command_wire::CancelOperationPayload::create(
        builder,
        &command_wire::CancelOperationPayloadArgs {
          command_id: Some(&command_id),
        },
      );
      (
        wire::CoreCommandKind::OperationCancel,
        wire::CoreCommandPayload::CancelOperationPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::InputSetEnabled(value) => {
      let payload = command_wire::SetInputEnabledPayload::create(
        builder,
        &command_wire::SetInputEnabledPayloadArgs {
          enabled: value.enabled,
        },
      );
      (
        wire::CoreCommandKind::InputSetEnabled,
        wire::CoreCommandPayload::SetInputEnabledPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::InputSetPointerEvents(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let events = body
        .events
        .iter()
        .copied()
        .map(pointer_event)
        .collect::<Vec<_>>();
      let events = builder.create_vector(&events);
      let payload = command_wire::PointerEventsPayload::create(
        builder,
        &command_wire::PointerEventsPayloadArgs {
          object_id: Some(&object_id),
          events: Some(events),
        },
      );
      (
        wire::CoreCommandKind::InputSetPointerEvents,
        wire::CoreCommandPayload::PointerEventsPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::InputSetGlobalKeys(body) => {
      let keys = body
        .keys
        .iter()
        .copied()
        .map(physical_key)
        .collect::<Vec<_>>();
      let keys = builder.create_vector(&keys);
      let payload = command_wire::GlobalKeysPayload::create(
        builder,
        &command_wire::GlobalKeysPayloadArgs { keys: Some(keys) },
      );
      (
        wire::CoreCommandKind::InputSetGlobalKeys,
        wire::CoreCommandPayload::GlobalKeysPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::InputSetController(body) => {
      let payload = write_controller_settings(builder, body);
      (
        wire::CoreCommandKind::InputSetController,
        wire::CoreCommandPayload::ControllerInputSettings,
        payload.as_union_value(),
      )
    }
    CommandBody::ControllerVibrate(body) => {
      let payload = command_wire::ControllerVibrationPayload::create(
        builder,
        &command_wire::ControllerVibrationPayloadArgs {
          low_frequency: body.low_frequency,
          high_frequency: body.high_frequency,
          duration_ms: body.duration_ms,
        },
      );
      (
        wire::CoreCommandKind::ControllerVibrate,
        wire::CoreCommandPayload::ControllerVibrationPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::DebugUi(body) => {
      let payload = command_wire::DebugUiPayload::create(
        builder,
        &command_wire::DebugUiPayloadArgs {
          surface: match body.surface {
            battlement::DebugUiSurface::LogViewer => command_wire::DebugUiSurface::LogViewer,
            battlement::DebugUiSurface::FpsViewer => command_wire::DebugUiSurface::FpsViewer,
          },
          visible: body.visible,
        },
      );
      (
        wire::CoreCommandKind::DebugUi,
        wire::CoreCommandPayload::DebugUiPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::VisualElementCreate(body) => {
      let nodes = crate::response_ui::write_subtree(builder, &body.node)?;
      let nodes = builder.create_vector(&nodes);
      let parent_id = uuid(body.parent_id.as_uuid());
      let root_id = uuid(body.node.object_id.as_uuid());
      let payload = ui_wire::VisualElementCreatePayload::create(
        builder,
        &ui_wire::VisualElementCreatePayloadArgs {
          parent_id: Some(&parent_id),
          child_index: body.child_index,
          nodes: Some(nodes),
          root_id: Some(&root_id),
        },
      );
      (
        wire::CoreCommandKind::VisualElementCreate,
        wire::CoreCommandPayload::VisualElementCreatePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::VisualElementUpdate(body) => {
      let (kind, object_id, element, parent_id, child_index) = match body.as_ref() {
        battlement::VisualElementUpdate::Properties { object_id, element } => (
          ui_wire::VisualElementUpdateKind::Properties,
          uuid(object_id.as_uuid()),
          Some(crate::response_ui::write_element(builder, element)?),
          None,
          None,
        ),
        battlement::VisualElementUpdate::Parent {
          object_id,
          parent_id,
          child_index,
        } => (
          ui_wire::VisualElementUpdateKind::Parent,
          uuid(object_id.as_uuid()),
          None,
          Some(uuid(parent_id.as_uuid())),
          *child_index,
        ),
        battlement::VisualElementUpdate::Index {
          object_id,
          child_index,
        } => (
          ui_wire::VisualElementUpdateKind::Index,
          uuid(object_id.as_uuid()),
          None,
          None,
          Some(*child_index),
        ),
      };
      let payload = ui_wire::VisualElementUpdatePayload::create(
        builder,
        &ui_wire::VisualElementUpdatePayloadArgs {
          kind,
          object_id: Some(&object_id),
          element,
          parent_id: parent_id.as_ref(),
          child_index,
        },
      );
      (
        wire::CoreCommandKind::VisualElementUpdate,
        wire::CoreCommandPayload::VisualElementUpdatePayload,
        payload.as_union_value(),
      )
    }
    CommandBody::VisualElementDestroy(body) => {
      let object_id = uuid(body.object_id.as_uuid());
      let payload = ui_wire::VisualElementDestroyPayload::create(
        builder,
        &ui_wire::VisualElementDestroyPayloadArgs {
          object_id: Some(&object_id),
        },
      );
      (
        wire::CoreCommandKind::VisualElementDestroy,
        wire::CoreCommandPayload::VisualElementDestroyPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::VisualElementPerformAction(body) => {
      let payload = write_visual_element_action(builder, body)?;
      (
        wire::CoreCommandKind::VisualElementPerformAction,
        wire::CoreCommandPayload::VisualElementActionPayload,
        payload.as_union_value(),
      )
    }
    CommandBody::MotionValue(body) => {
      let payload = crate::response_motion::write_value_operation(builder, body)?;
      (
        wire::CoreCommandKind::MotionValue,
        wire::CoreCommandPayload::MotionValueOperation,
        payload.as_union_value(),
      )
    }
    CommandBody::MotionValuePlayback(body) => {
      let payload = crate::response_motion::write_value_playback_operation(builder, body);
      (
        wire::CoreCommandKind::MotionValuePlayback,
        wire::CoreCommandPayload::MotionValuePlaybackOperation,
        payload.as_union_value(),
      )
    }
    CommandBody::MotionPlayback(body) => {
      let payload = crate::response_motion::write_playback_operation(builder, body);
      (
        wire::CoreCommandKind::MotionPlayback,
        wire::CoreCommandPayload::MotionPlaybackOperation,
        payload.as_union_value(),
      )
    }
    CommandBody::MotionControlledClock(body) => {
      let payload = crate::response_motion::write_controlled_clock_operation(builder, body);
      (
        wire::CoreCommandKind::MotionControlledClock,
        wire::CoreCommandPayload::MotionControlledClockOperation,
        payload.as_union_value(),
      )
    }
    CommandBody::MotionControl(body) => {
      let payload = crate::response_motion::write_control_operation(builder, body)?;
      (
        wire::CoreCommandKind::MotionControl,
        wire::CoreCommandPayload::MotionControlOperation,
        payload.as_union_value(),
      )
    }
    CommandBody::MotionScope(body) => {
      let payload = crate::response_motion::write_scope_operation(builder, body)?;
      (
        wire::CoreCommandKind::MotionScope,
        wire::CoreCommandPayload::MotionScopeOperation,
        payload.as_union_value(),
      )
    }
    CommandBody::MotionDragControl(body) => {
      let payload = crate::response_motion::write_drag_control_operation(builder, body);
      (
        wire::CoreCommandKind::MotionDragControl,
        wire::CoreCommandPayload::MotionDragControlOperation,
        payload.as_union_value(),
      )
    }
    CommandBody::GeometryObservationUpdate(body) => {
      let payload = write_geometry_update(builder, body);
      (
        wire::CoreCommandKind::GeometryObservationUpdate,
        wire::CoreCommandPayload::GeometryObservationUpdate,
        payload.as_union_value(),
      )
    }
    CommandBody::AccessibilityUpdate(body) => {
      let payload = write_accessibility_update(builder, body);
      (
        wire::CoreCommandKind::AccessibilityUpdate,
        wire::CoreCommandPayload::AccessibilityUpdate,
        payload.as_union_value(),
      )
    }
  };
  let command_id = uuid(command.command_id.as_uuid());
  Ok(wire::CoreCommand::create(
    builder,
    &wire::CoreCommandArgs {
      command_id: Some(&command_id),
      blocking: command.blocking,
      kind,
      payload_type,
      payload: Some(payload),
    },
  ))
}

fn write_visual_element_action<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::VisualElementPerformAction,
) -> Result<WIPOffset<ui_wire::VisualElementActionPayload<'a>>, ProtocolError> {
  let mut streaks = None;
  let mut pointer_id = 0;
  let mut descendant_id = None;
  let mut cursor_index = 0;
  let mut selection_index = 0;
  let kind = match &value.action {
    battlement::VisualElementAction::ParticleStreaks { streaks: values } => {
      if values.len() > 128 || values.iter().any(|value| !value.is_valid()) {
        return Err(ProtocolError::new(
          "UI particle streak action violates its bounded value contract",
        ));
      }
      let values = values
        .iter()
        .map(|value| {
          let origin = ui_wire::F32Vector2::new(value.origin[0], value.origin[1]);
          let travel = ui_wire::F32Vector2::new(value.travel[0], value.travel[1]);
          let size = ui_wire::F32Vector2::new(value.size[0], value.size[1]);
          let color =
            common::RgbaColor::new(value.color.r, value.color.g, value.color.b, value.color.a);
          ui_wire::UiParticleStreak::create(
            builder,
            &ui_wire::UiParticleStreakArgs {
              origin: Some(&origin),
              travel: Some(&travel),
              size: Some(&size),
              rotation: value.rotation,
              color: Some(&color),
              lifetime_ms: value.lifetime_ms,
              delay_ms: value.delay_ms,
            },
          )
        })
        .collect::<Vec<_>>();
      streaks = Some(builder.create_vector(&values));
      ui_wire::VisualElementActionKind::ParticleStreaks
    }
    battlement::VisualElementAction::Focus => ui_wire::VisualElementActionKind::Focus,
    battlement::VisualElementAction::Blur => ui_wire::VisualElementActionKind::Blur,
    battlement::VisualElementAction::CapturePointer { pointer_id: value } => {
      pointer_id = *value;
      ui_wire::VisualElementActionKind::CapturePointer
    }
    battlement::VisualElementAction::ReleasePointer { pointer_id: value } => {
      pointer_id = *value;
      ui_wire::VisualElementActionKind::ReleasePointer
    }
    battlement::VisualElementAction::ScrollTo {
      descendant_id: value,
    } => {
      descendant_id = Some(uuid(value.as_uuid()));
      ui_wire::VisualElementActionKind::ScrollTo
    }
    battlement::VisualElementAction::SelectText {
      cursor_index: cursor,
      selection_index: selection,
    } => {
      cursor_index = *cursor;
      selection_index = *selection;
      ui_wire::VisualElementActionKind::SelectText
    }
  };
  let object_id = uuid(value.object_id.as_uuid());
  Ok(ui_wire::VisualElementActionPayload::create(
    builder,
    &ui_wire::VisualElementActionPayloadArgs {
      object_id: Some(&object_id),
      kind,
      streaks,
      pointer_id,
      descendant_id: descendant_id.as_ref(),
      cursor_index,
      selection_index,
    },
  ))
}

fn uuid(value: &uuid::Uuid) -> common::Uuid {
  common::Uuid::new(value.as_bytes())
}

fn vector3(value: battlement::Vector3) -> common::Vector3d {
  common::Vector3d::new(value.x, value.y, value.z)
}

fn quaternion(value: battlement::Quaternion) -> common::Quaterniond {
  common::Quaterniond::new(value.x, value.y, value.z, value.w)
}

fn color(value: battlement::Color) -> common::RgbaColor {
  common::RgbaColor::new(value.r, value.g, value.b, value.a)
}

fn rgb(value: battlement::RgbColor) -> common::RgbColor {
  common::RgbColor::new(value.r, value.g, value.b)
}

fn image_fit(value: battlement::ImageFit) -> world_wire::ImageFit {
  match value {
    battlement::ImageFit::Stretch => world_wire::ImageFit::Stretch,
    battlement::ImageFit::Contain => world_wire::ImageFit::Contain,
    battlement::ImageFit::Cover => world_wire::ImageFit::Cover,
  }
}

fn horizontal_alignment(value: battlement::HorizontalAlignment) -> world_wire::HorizontalAlignment {
  match value {
    battlement::HorizontalAlignment::Left => world_wire::HorizontalAlignment::Left,
    battlement::HorizontalAlignment::Center => world_wire::HorizontalAlignment::Center,
    battlement::HorizontalAlignment::Right => world_wire::HorizontalAlignment::Right,
    battlement::HorizontalAlignment::Justified => world_wire::HorizontalAlignment::Justified,
  }
}

fn vertical_alignment(value: battlement::VerticalAlignment) -> world_wire::VerticalAlignment {
  match value {
    battlement::VerticalAlignment::Top => world_wire::VerticalAlignment::Top,
    battlement::VerticalAlignment::Middle => world_wire::VerticalAlignment::Middle,
    battlement::VerticalAlignment::Bottom => world_wire::VerticalAlignment::Bottom,
  }
}

fn camera_clear_mode(value: battlement::CameraClearMode) -> world_wire::CameraClearMode {
  match value {
    battlement::CameraClearMode::Skybox => world_wire::CameraClearMode::Skybox,
    battlement::CameraClearMode::SolidColor => world_wire::CameraClearMode::SolidColor,
    battlement::CameraClearMode::Depth => world_wire::CameraClearMode::Depth,
    battlement::CameraClearMode::Nothing => world_wire::CameraClearMode::Nothing,
  }
}

fn light_type(value: battlement::LightType) -> world_wire::LightType {
  match value {
    battlement::LightType::Directional => world_wire::LightType::Directional,
    battlement::LightType::Point => world_wire::LightType::Point,
    battlement::LightType::Spot => world_wire::LightType::Spot,
  }
}

fn shadow_mode(value: battlement::ShadowMode) -> world_wire::ShadowMode {
  match value {
    battlement::ShadowMode::None => world_wire::ShadowMode::None,
    battlement::ShadowMode::Hard => world_wire::ShadowMode::Hard,
    battlement::ShadowMode::Soft => world_wire::ShadowMode::Soft,
  }
}

fn conflict(value: battlement::ConflictPolicy) -> command_wire::ConflictPolicy {
  match value {
    battlement::ConflictPolicy::Cancel => command_wire::ConflictPolicy::Cancel,
    battlement::ConflictPolicy::Wait => command_wire::ConflictPolicy::Wait,
  }
}

fn tween(value: battlement::Tween) -> command_wire::Tween {
  let easing = match value.easing {
    battlement::Easing::Linear => command_wire::Easing::Linear,
    battlement::Easing::InSine => command_wire::Easing::InSine,
    battlement::Easing::OutSine => command_wire::Easing::OutSine,
    battlement::Easing::InOutSine => command_wire::Easing::InOutSine,
    battlement::Easing::InQuad => command_wire::Easing::InQuad,
    battlement::Easing::OutQuad => command_wire::Easing::OutQuad,
    battlement::Easing::InOutQuad => command_wire::Easing::InOutQuad,
    battlement::Easing::InCubic => command_wire::Easing::InCubic,
    battlement::Easing::OutCubic => command_wire::Easing::OutCubic,
    battlement::Easing::InOutCubic => command_wire::Easing::InOutCubic,
    battlement::Easing::InQuart => command_wire::Easing::InQuart,
    battlement::Easing::OutQuart => command_wire::Easing::OutQuart,
    battlement::Easing::InOutQuart => command_wire::Easing::InOutQuart,
    battlement::Easing::InQuint => command_wire::Easing::InQuint,
    battlement::Easing::OutQuint => command_wire::Easing::OutQuint,
    battlement::Easing::InOutQuint => command_wire::Easing::InOutQuint,
    battlement::Easing::InExpo => command_wire::Easing::InExpo,
    battlement::Easing::OutExpo => command_wire::Easing::OutExpo,
    battlement::Easing::InOutExpo => command_wire::Easing::InOutExpo,
    battlement::Easing::InCirc => command_wire::Easing::InCirc,
    battlement::Easing::OutCirc => command_wire::Easing::OutCirc,
    battlement::Easing::InOutCirc => command_wire::Easing::InOutCirc,
    battlement::Easing::InBack => command_wire::Easing::InBack,
    battlement::Easing::OutBack => command_wire::Easing::OutBack,
    battlement::Easing::InOutBack => command_wire::Easing::InOutBack,
    battlement::Easing::InElastic => command_wire::Easing::InElastic,
    battlement::Easing::OutElastic => command_wire::Easing::OutElastic,
    battlement::Easing::InOutElastic => command_wire::Easing::InOutElastic,
    battlement::Easing::InBounce => command_wire::Easing::InBounce,
    battlement::Easing::OutBounce => command_wire::Easing::OutBounce,
    battlement::Easing::InOutBounce => command_wire::Easing::InOutBounce,
  };
  let (repeat_kind, repeat_count, repeat_mode) = match value.repeat {
    battlement::TweenRepeat::Once => (
      command_wire::TweenRepeatKind::Once,
      0,
      command_wire::RepeatMode::Restart,
    ),
    battlement::TweenRepeat::Count {
      additional_traversals,
      mode,
    } => (
      command_wire::TweenRepeatKind::Count,
      additional_traversals,
      repeat_mode(mode),
    ),
    battlement::TweenRepeat::Forever(mode) => {
      (command_wire::TweenRepeatKind::Forever, 0, repeat_mode(mode))
    }
  };
  command_wire::Tween::new(
    value.delay_ms,
    value.duration_ms,
    easing,
    repeat_kind,
    repeat_count,
    repeat_mode,
  )
}

fn repeat_mode(value: battlement::RepeatMode) -> command_wire::RepeatMode {
  match value {
    battlement::RepeatMode::Restart => command_wire::RepeatMode::Restart,
    battlement::RepeatMode::PingPong => command_wire::RepeatMode::PingPong,
  }
}

fn pointer_event(value: battlement::PointerEvent) -> world_wire::PointerEventKind {
  match value {
    battlement::PointerEvent::Enter => world_wire::PointerEventKind::Enter,
    battlement::PointerEvent::Exit => world_wire::PointerEventKind::Exit,
    battlement::PointerEvent::Down => world_wire::PointerEventKind::Down,
    battlement::PointerEvent::Up => world_wire::PointerEventKind::Up,
    battlement::PointerEvent::Click => world_wire::PointerEventKind::Click,
  }
}

fn controller_button(value: battlement::ControllerButton) -> client_wire::ControllerButton {
  match value {
    battlement::ControllerButton::South => client_wire::ControllerButton::South,
    battlement::ControllerButton::East => client_wire::ControllerButton::East,
    battlement::ControllerButton::West => client_wire::ControllerButton::West,
    battlement::ControllerButton::North => client_wire::ControllerButton::North,
    battlement::ControllerButton::LeftShoulder => client_wire::ControllerButton::LeftShoulder,
    battlement::ControllerButton::RightShoulder => client_wire::ControllerButton::RightShoulder,
    battlement::ControllerButton::LeftStickButton => client_wire::ControllerButton::LeftStickButton,
    battlement::ControllerButton::RightStickButton => {
      client_wire::ControllerButton::RightStickButton
    }
    battlement::ControllerButton::Start => client_wire::ControllerButton::Start,
    battlement::ControllerButton::Select => client_wire::ControllerButton::Select,
  }
}

fn physical_key(value: battlement::PhysicalKey) -> ui_event_wire::PhysicalKey {
  use battlement::PhysicalKey as Key;
  use ui_event_wire::PhysicalKey as Wire;

  match value {
    Key::Escape => Wire::Escape,
    Key::F1 => Wire::F1,
    Key::F2 => Wire::F2,
    Key::F3 => Wire::F3,
    Key::F4 => Wire::F4,
    Key::F5 => Wire::F5,
    Key::F6 => Wire::F6,
    Key::F7 => Wire::F7,
    Key::F8 => Wire::F8,
    Key::F9 => Wire::F9,
    Key::F10 => Wire::F10,
    Key::F11 => Wire::F11,
    Key::F12 => Wire::F12,
    Key::Backquote => Wire::Backquote,
    Key::Digit0 => Wire::Digit0,
    Key::Digit1 => Wire::Digit1,
    Key::Digit2 => Wire::Digit2,
    Key::Digit3 => Wire::Digit3,
    Key::Digit4 => Wire::Digit4,
    Key::Digit5 => Wire::Digit5,
    Key::Digit6 => Wire::Digit6,
    Key::Digit7 => Wire::Digit7,
    Key::Digit8 => Wire::Digit8,
    Key::Digit9 => Wire::Digit9,
    Key::Minus => Wire::Minus,
    Key::Equal => Wire::Equal,
    Key::Backspace => Wire::Backspace,
    Key::Tab => Wire::Tab,
    Key::KeyA => Wire::KeyA,
    Key::KeyB => Wire::KeyB,
    Key::KeyC => Wire::KeyC,
    Key::KeyD => Wire::KeyD,
    Key::KeyE => Wire::KeyE,
    Key::KeyF => Wire::KeyF,
    Key::KeyG => Wire::KeyG,
    Key::KeyH => Wire::KeyH,
    Key::KeyI => Wire::KeyI,
    Key::KeyJ => Wire::KeyJ,
    Key::KeyK => Wire::KeyK,
    Key::KeyL => Wire::KeyL,
    Key::KeyM => Wire::KeyM,
    Key::KeyN => Wire::KeyN,
    Key::KeyO => Wire::KeyO,
    Key::KeyP => Wire::KeyP,
    Key::KeyQ => Wire::KeyQ,
    Key::KeyR => Wire::KeyR,
    Key::KeyS => Wire::KeyS,
    Key::KeyT => Wire::KeyT,
    Key::KeyU => Wire::KeyU,
    Key::KeyV => Wire::KeyV,
    Key::KeyW => Wire::KeyW,
    Key::KeyX => Wire::KeyX,
    Key::KeyY => Wire::KeyY,
    Key::KeyZ => Wire::KeyZ,
    Key::BracketLeft => Wire::BracketLeft,
    Key::BracketRight => Wire::BracketRight,
    Key::Backslash => Wire::Backslash,
    Key::CapsLock => Wire::CapsLock,
    Key::Semicolon => Wire::Semicolon,
    Key::Quote => Wire::Quote,
    Key::Enter => Wire::Enter,
    Key::ShiftLeft => Wire::ShiftLeft,
    Key::ShiftRight => Wire::ShiftRight,
    Key::ControlLeft => Wire::ControlLeft,
    Key::ControlRight => Wire::ControlRight,
    Key::AltLeft => Wire::AltLeft,
    Key::AltRight => Wire::AltRight,
    Key::MetaLeft => Wire::MetaLeft,
    Key::MetaRight => Wire::MetaRight,
    Key::Comma => Wire::Comma,
    Key::Period => Wire::Period,
    Key::Slash => Wire::Slash,
    Key::Space => Wire::Space,
    Key::ContextMenu => Wire::ContextMenu,
    Key::Insert => Wire::Insert,
    Key::Delete => Wire::Delete,
    Key::Home => Wire::Home,
    Key::End => Wire::End,
    Key::PageUp => Wire::PageUp,
    Key::PageDown => Wire::PageDown,
    Key::ArrowLeft => Wire::ArrowLeft,
    Key::ArrowRight => Wire::ArrowRight,
    Key::ArrowUp => Wire::ArrowUp,
    Key::ArrowDown => Wire::ArrowDown,
    Key::PrintScreen => Wire::PrintScreen,
    Key::ScrollLock => Wire::ScrollLock,
    Key::Pause => Wire::Pause,
    Key::NumLock => Wire::NumLock,
    Key::Numpad0 => Wire::Numpad0,
    Key::Numpad1 => Wire::Numpad1,
    Key::Numpad2 => Wire::Numpad2,
    Key::Numpad3 => Wire::Numpad3,
    Key::Numpad4 => Wire::Numpad4,
    Key::Numpad5 => Wire::Numpad5,
    Key::Numpad6 => Wire::Numpad6,
    Key::Numpad7 => Wire::Numpad7,
    Key::Numpad8 => Wire::Numpad8,
    Key::Numpad9 => Wire::Numpad9,
    Key::NumpadDecimal => Wire::NumpadDecimal,
    Key::NumpadAdd => Wire::NumpadAdd,
    Key::NumpadSubtract => Wire::NumpadSubtract,
    Key::NumpadMultiply => Wire::NumpadMultiply,
    Key::NumpadDivide => Wire::NumpadDivide,
    Key::NumpadEnter => Wire::NumpadEnter,
  }
}

fn write_controller_settings<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::ControllerInputSettings,
) -> WIPOffset<command_wire::ControllerInputSettings<'a>> {
  let buttons = value
    .buttons
    .iter()
    .copied()
    .map(controller_button)
    .collect::<Vec<_>>();
  let buttons = builder.create_vector(&buttons);
  command_wire::ControllerInputSettings::create(
    builder,
    &command_wire::ControllerInputSettingsArgs {
      buttons: Some(buttons),
      navigation_enabled: value.navigation_enabled,
      stick_dead_zone: value.stick_dead_zone,
      repeat_delay_ms: value.repeat_delay_ms,
      repeat_interval_ms: value.repeat_interval_ms,
    },
  )
}

fn write_prepared_asset<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::PreparedAsset,
) -> WIPOffset<world_wire::PreparedAsset<'a>> {
  let (kind, address) = match value {
    battlement::PreparedAsset::Scene(value) => {
      (world_wire::PreparedAssetKind::Scene, value.as_str())
    }
    battlement::PreparedAsset::Prefab(value) => {
      (world_wire::PreparedAssetKind::Prefab, value.as_str())
    }
    battlement::PreparedAsset::ParticleEffect(value) => (
      world_wire::PreparedAssetKind::ParticleEffect,
      value.as_str(),
    ),
    battlement::PreparedAsset::Material(value) => {
      (world_wire::PreparedAssetKind::Material, value.as_str())
    }
    battlement::PreparedAsset::Texture(value) => {
      (world_wire::PreparedAssetKind::Texture, value.as_str())
    }
    battlement::PreparedAsset::Sprite(value) => {
      (world_wire::PreparedAssetKind::Sprite, value.as_str())
    }
    battlement::PreparedAsset::VectorImage(value) => {
      (world_wire::PreparedAssetKind::VectorImage, value.as_str())
    }
    battlement::PreparedAsset::RenderTexture(value) => {
      (world_wire::PreparedAssetKind::RenderTexture, value.as_str())
    }
    battlement::PreparedAsset::AudioClip(value) => {
      (world_wire::PreparedAssetKind::AudioClip, value.as_str())
    }
    battlement::PreparedAsset::TextMeshProFont(value) => (
      world_wire::PreparedAssetKind::TextMeshProFont,
      value.as_str(),
    ),
    battlement::PreparedAsset::UiFont(value) => {
      (world_wire::PreparedAssetKind::UiFont, value.as_str())
    }
  };
  let address = builder.create_string(address);
  world_wire::PreparedAsset::create(
    builder,
    &world_wire::PreparedAssetArgs {
      kind,
      address: Some(address),
    },
  )
}

fn write_game_object<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::GameObject,
) -> Result<WIPOffset<world_wire::GameObject<'a>>, ProtocolError> {
  let (parent_scene_kind, parent_scene_id) = match value.parent_scene {
    battlement::ParentScene::PrimaryScene => (world_wire::ParentSceneKind::PrimaryScene, None),
    battlement::ParentScene::Scene(value) => (
      world_wire::ParentSceneKind::Scene,
      Some(uuid(value.as_uuid())),
    ),
    battlement::ParentScene::Persistent => (world_wire::ParentSceneKind::Persistent, None),
  };
  let parent_scene = world_wire::ParentScene::create(
    builder,
    &world_wire::ParentSceneArgs {
      kind: parent_scene_kind,
      scene_id: parent_scene_id.as_ref(),
    },
  );
  let parent_id = value.parent_id.as_ref().map(|value| uuid(value.as_uuid()));
  let pointer_events = value
    .pointer_events
    .iter()
    .copied()
    .map(pointer_event)
    .collect::<Vec<_>>();
  let pointer_events = builder.create_vector(&pointer_events);
  let local_transform = common::LocalTransform::new(
    &vector3(value.local_transform.position),
    &quaternion(value.local_transform.rotation),
    &vector3(value.local_transform.scale),
  );

  let (kind, content_type, content): (_, _, WIPOffset<UnionWIPOffset>) = match &value.kind {
    battlement::GameObjectKind::UiDocument(value) => {
      let content = write_ui_document_object(builder, value);
      (
        world_wire::GameObjectKind::UiDocument,
        world_wire::GameObjectContent::UiDocumentObject,
        content.as_union_value(),
      )
    }
    battlement::GameObjectKind::Empty => {
      let content = world_wire::EmptyObject::create(builder, &world_wire::EmptyObjectArgs {});
      (
        world_wire::GameObjectKind::Empty,
        world_wire::GameObjectContent::EmptyObject,
        content.as_union_value(),
      )
    }
    battlement::GameObjectKind::Cube { materials }
    | battlement::GameObjectKind::Sphere { materials }
    | battlement::GameObjectKind::Capsule { materials }
    | battlement::GameObjectKind::Cylinder { materials }
    | battlement::GameObjectKind::Plane { materials }
    | battlement::GameObjectKind::Quad { materials } => {
      let materials = write_materials(builder, materials);
      let content = world_wire::PrimitiveObject::create(
        builder,
        &world_wire::PrimitiveObjectArgs {
          materials: Some(materials),
        },
      );
      let kind = match &value.kind {
        battlement::GameObjectKind::Cube { .. } => world_wire::GameObjectKind::Cube,
        battlement::GameObjectKind::Sphere { .. } => world_wire::GameObjectKind::Sphere,
        battlement::GameObjectKind::Capsule { .. } => world_wire::GameObjectKind::Capsule,
        battlement::GameObjectKind::Cylinder { .. } => world_wire::GameObjectKind::Cylinder,
        battlement::GameObjectKind::Plane { .. } => world_wire::GameObjectKind::Plane,
        battlement::GameObjectKind::Quad { .. } => world_wire::GameObjectKind::Quad,
        _ => unreachable!(),
      };
      (
        kind,
        world_wire::GameObjectContent::PrimitiveObject,
        content.as_union_value(),
      )
    }
    battlement::GameObjectKind::Image { image } => {
      let texture = builder.create_string(image.texture.as_str());
      let tint = rgb(image.tint);
      let content = world_wire::ImageObject::create(
        builder,
        &world_wire::ImageObjectArgs {
          texture: Some(texture),
          width: image.width,
          height: image.height,
          fit: image_fit(image.fit),
          tint: Some(&tint),
          opacity: image.opacity,
          face_camera: image.face_camera,
        },
      );
      (
        world_wire::GameObjectKind::Image,
        world_wire::GameObjectContent::ImageObject,
        content.as_union_value(),
      )
    }
    battlement::GameObjectKind::Text { text } => {
      let content_text = builder.create_string(&text.text);
      let font = builder.create_string(text.font.as_str());
      let text_color = color(text.color);
      let content = world_wire::TextObject::create(
        builder,
        &world_wire::TextObjectArgs {
          text: Some(content_text),
          font: Some(font),
          size: text.size,
          color: Some(&text_color),
          horizontal: horizontal_alignment(text.horizontal),
          vertical: vertical_alignment(text.vertical),
          wrap_width: text.wrap_width,
          rich_text: text.rich_text,
          face_camera: text.face_camera,
        },
      );
      (
        world_wire::GameObjectKind::Text,
        world_wire::GameObjectContent::TextObject,
        content.as_union_value(),
      )
    }
    battlement::GameObjectKind::Camera { camera } => {
      let clear_color = color(camera.clear_color);
      let content = world_wire::CameraObject::create(
        builder,
        &world_wire::CameraObjectArgs {
          enabled: camera.enabled,
          projection: match camera.projection {
            battlement::CameraProjection::Perspective => world_wire::CameraProjection::Perspective,
            battlement::CameraProjection::Orthographic => {
              world_wire::CameraProjection::Orthographic
            }
          },
          field_of_view: camera.field_of_view,
          orthographic_size: camera.orthographic_size,
          near: camera.near,
          far: camera.far,
          clear_mode: camera_clear_mode(camera.clear_mode),
          clear_color: Some(&clear_color),
        },
      );
      (
        world_wire::GameObjectKind::Camera,
        world_wire::GameObjectContent::CameraObject,
        content.as_union_value(),
      )
    }
    battlement::GameObjectKind::Light { light } => {
      let light_color = color(light.color);
      let content = world_wire::LightObject::create(
        builder,
        &world_wire::LightObjectArgs {
          enabled: light.enabled,
          light_type: light_type(light.light_type),
          color: Some(&light_color),
          intensity: light.intensity,
          range: light.range,
          outer_spot_angle: light.outer_spot_angle,
          inner_spot_angle: light.inner_spot_angle,
          shadows: shadow_mode(light.shadows),
        },
      );
      (
        world_wire::GameObjectKind::Light,
        world_wire::GameObjectContent::LightObject,
        content.as_union_value(),
      )
    }
    battlement::GameObjectKind::Prefab {
      address,
      materials,
      animator,
    } => {
      let address = builder.create_string(address.as_str());
      let materials = write_materials(builder, materials);
      let animator = animator
        .as_ref()
        .map(|value| write_animator_state(builder, value));
      let content = world_wire::PrefabObject::create(
        builder,
        &world_wire::PrefabObjectArgs {
          address: Some(address),
          materials: Some(materials),
          animator,
        },
      );
      (
        world_wire::GameObjectKind::Prefab,
        world_wire::GameObjectContent::PrefabObject,
        content.as_union_value(),
      )
    }
  };
  let object_id = uuid(value.object_id.as_uuid());
  Ok(world_wire::GameObject::create(
    builder,
    &world_wire::GameObjectArgs {
      object_id: Some(&object_id),
      parent_scene: Some(parent_scene),
      parent_id: parent_id.as_ref(),
      active: value.active,
      local_transform: Some(&local_transform),
      pointer_events: Some(pointer_events),
      drag_mode: match value.drag_mode {
        None => world_wire::DragMode::None,
        Some(battlement::DragMode::SnapToPointer) => world_wire::DragMode::SnapToPointer,
        Some(battlement::DragMode::PreserveOffset) => world_wire::DragMode::PreserveOffset,
      },
      kind,
      content_type,
      content: Some(content),
    },
  ))
}

pub(crate) fn write_snapshot<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::Snapshot,
) -> Result<WIPOffset<wire::Snapshot<'a>>, ProtocolError> {
  let prepared_assets = value
    .prepared_assets
    .iter()
    .map(|value| write_prepared_asset(builder, value))
    .collect::<Vec<_>>();
  let prepared_assets = builder.create_vector(&prepared_assets);
  let scenes = value
    .scenes
    .iter()
    .map(|value| {
      let scene_id = uuid(value.scene_id.as_uuid());
      let address = builder.create_string(value.address.as_str());
      world_wire::Scene::create(
        builder,
        &world_wire::SceneArgs {
          scene_id: Some(&scene_id),
          address: Some(address),
        },
      )
    })
    .collect::<Vec<_>>();
  let scenes = builder.create_vector(&scenes);
  let objects = value
    .objects
    .iter()
    .map(|value| write_game_object(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let objects = builder.create_vector(&objects);
  let ui = value
    .ui
    .iter()
    .map(|value| crate::response_ui::write_document(builder, value))
    .collect::<Result<Vec<_>, _>>()?;
  let ui = builder.create_vector(&ui);
  let panel_input_configuration =
    write_panel_input_configuration(builder, &value.panel_input_configuration);
  let global_keys = value
    .global_keys
    .iter()
    .copied()
    .map(physical_key)
    .collect::<Vec<_>>();
  let global_keys = builder.create_vector(&global_keys);
  let controller_input = value
    .controller_input
    .as_ref()
    .map(|value| write_controller_settings(builder, value));
  let session_id = uuid(value.session_id.as_uuid());
  let primary_scene_id = value
    .primary_scene_id
    .as_ref()
    .map(|value| uuid(value.as_uuid()));
  let input_camera_id = value
    .input_camera_id
    .as_ref()
    .map(|value| uuid(value.as_uuid()));
  Ok(wire::Snapshot::create(
    builder,
    &wire::SnapshotArgs {
      session_id: Some(&session_id),
      prepared_assets: Some(prepared_assets),
      scenes: Some(scenes),
      primary_scene_id: primary_scene_id.as_ref(),
      objects: Some(objects),
      ui: Some(ui),
      panel_input_configuration: Some(panel_input_configuration),
      input_camera_id: input_camera_id.as_ref(),
      input_disabled: value.input_disabled,
      global_keys: Some(global_keys),
      controller_input,
    },
  ))
}

fn write_panel_input_configuration<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::PanelInputConfiguration,
) -> WIPOffset<world_wire::PanelInputConfiguration<'a>> {
  let (distance_kind, maximum_interaction_distance) = match value.maximum_interaction_distance {
    battlement::InteractionDistance::Unbounded => {
      (world_wire::InteractionDistanceKind::Unbounded, 0.0)
    }
    battlement::InteractionDistance::Inclusive(value) => {
      (world_wire::InteractionDistanceKind::Inclusive, value)
    }
  };
  world_wire::PanelInputConfiguration::create(
    builder,
    &world_wire::PanelInputConfigurationArgs {
      interaction_layers: value.interaction_layers.0,
      distance_kind,
      maximum_interaction_distance,
      input_redirection: match value.input_redirection {
        battlement::PanelInputRedirection::AutoSwitch => {
          world_wire::PanelInputRedirection::AutoSwitch
        }
        battlement::PanelInputRedirection::Never => world_wire::PanelInputRedirection::Never,
        battlement::PanelInputRedirection::Always => world_wire::PanelInputRedirection::Always,
      },
    },
  )
}

fn write_materials<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  values: &[battlement::MaterialAssignment],
) -> WIPOffset<
  flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<world_wire::MaterialAssignment<'a>>>,
> {
  let values = values
    .iter()
    .map(|value| {
      let address = builder.create_string(value.address.as_str());
      world_wire::MaterialAssignment::create(
        builder,
        &world_wire::MaterialAssignmentArgs {
          slot: value.slot,
          address: Some(address),
        },
      )
    })
    .collect::<Vec<_>>();
  builder.create_vector(&values)
}

fn write_animator_state<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::AnimatorState,
) -> WIPOffset<world_wire::AnimatorState<'a>> {
  let state = builder.create_string(&value.state);
  let bool_parameters = value
    .bool_parameters
    .iter()
    .map(|(name, value)| {
      let name = builder.create_string(name);
      world_wire::AnimatorBoolParameter::create(
        builder,
        &world_wire::AnimatorBoolParameterArgs {
          name: Some(name),
          value: *value,
        },
      )
    })
    .collect::<Vec<_>>();
  let bool_parameters = builder.create_vector(&bool_parameters);
  let int_parameters = value
    .int_parameters
    .iter()
    .map(|(name, value)| {
      let name = builder.create_string(name);
      world_wire::AnimatorIntParameter::create(
        builder,
        &world_wire::AnimatorIntParameterArgs {
          name: Some(name),
          value: *value,
        },
      )
    })
    .collect::<Vec<_>>();
  let int_parameters = builder.create_vector(&int_parameters);
  let float_parameters = value
    .float_parameters
    .iter()
    .map(|(name, value)| {
      let name = builder.create_string(name);
      world_wire::AnimatorFloatParameter::create(
        builder,
        &world_wire::AnimatorFloatParameterArgs {
          name: Some(name),
          value: *value,
        },
      )
    })
    .collect::<Vec<_>>();
  let float_parameters = builder.create_vector(&float_parameters);
  world_wire::AnimatorState::create(
    builder,
    &world_wire::AnimatorStateArgs {
      state: Some(state),
      layer: value.layer,
      normalized_start_time: value.normalized_start_time,
      bool_parameters: Some(bool_parameters),
      int_parameters: Some(int_parameters),
      float_parameters: Some(float_parameters),
      speed: value.speed,
    },
  )
}

fn write_ui_document_object<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::UiDocumentState,
) -> WIPOffset<world_wire::UiDocumentObject<'a>> {
  let panel_settings = write_panel_settings(builder, &value.panel_settings);
  let root_id = uuid(value.root_id().as_uuid());
  let world_space_size = screen_size(value.world_space_size);
  world_wire::UiDocumentObject::create(
    builder,
    &world_wire::UiDocumentObjectArgs {
      root_id: Some(&root_id),
      panel_settings: Some(panel_settings),
      position: match value.position {
        battlement::DocumentPosition::Relative => world_wire::DocumentPosition::Relative,
        battlement::DocumentPosition::Absolute => world_wire::DocumentPosition::Absolute,
      },
      world_space_size_mode: match value.world_space_size_mode {
        battlement::WorldSpaceSizeMode::Fixed => world_wire::WorldSpaceSizeMode::Fixed,
        battlement::WorldSpaceSizeMode::Dynamic => world_wire::WorldSpaceSizeMode::Dynamic,
      },
      world_space_size: Some(&world_space_size),
      pivot_reference_size: match value.pivot_reference_size {
        battlement::PivotReferenceSize::BoundingBox => world_wire::PivotReferenceSize::BoundingBox,
        battlement::PivotReferenceSize::Layout => world_wire::PivotReferenceSize::Layout,
      },
      pivot: document_pivot(value.pivot),
      sorting_order: value.sorting_order,
    },
  )
}

fn write_panel_settings<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::PanelSettings,
) -> WIPOffset<world_wire::PanelSettings<'a>> {
  let filters = value
    .dynamic_atlas
    .filters
    .iter()
    .copied()
    .map(|value| match value {
      battlement::DynamicAtlasFilter::Readability => world_wire::DynamicAtlasFilter::Readability,
      battlement::DynamicAtlasFilter::Size => world_wire::DynamicAtlasFilter::Size,
      battlement::DynamicAtlasFilter::Format => world_wire::DynamicAtlasFilter::Format,
      battlement::DynamicAtlasFilter::ColorSpace => world_wire::DynamicAtlasFilter::ColorSpace,
      battlement::DynamicAtlasFilter::FilterMode => world_wire::DynamicAtlasFilter::FilterMode,
    })
    .collect::<Vec<_>>();
  let filters = builder.create_vector(&filters);
  let dynamic_atlas = world_wire::DynamicAtlasSettings::create(
    builder,
    &world_wire::DynamicAtlasSettingsArgs {
      min_atlas_size: value.dynamic_atlas.min_atlas_size,
      max_atlas_size: value.dynamic_atlas.max_atlas_size,
      max_sub_texture_size: value.dynamic_atlas.max_sub_texture_size,
      filters: Some(filters),
    },
  );
  let mut scale = 1.0;
  let mut reference_dpi = 96.0;
  let mut fallback_dpi = 96.0;
  let mut reference_resolution = battlement::ScreenSize::new(1200, 800);
  let mut screen_match_mode = world_wire::PanelScreenMatchMode::MatchWidthOrHeight;
  let mut match_factor = 0.0;
  let scale_mode = match value.scale_mode {
    battlement::PanelScaleMode::ConstantPixelSize(value) => {
      scale = value.value();
      world_wire::PanelScaleMode::ConstantPixelSize
    }
    battlement::PanelScaleMode::ConstantLogicalPixelSize => {
      world_wire::PanelScaleMode::ConstantLogicalPixelSize
    }
    battlement::PanelScaleMode::ConstantPhysicalSize(value) => {
      reference_dpi = value.reference_dpi();
      fallback_dpi = value.fallback_dpi();
      world_wire::PanelScaleMode::ConstantPhysicalSize
    }
    battlement::PanelScaleMode::ScaleWithScreenSize(value) => {
      reference_resolution = value.reference_resolution();
      match value.screen_match_mode() {
        battlement::PanelScreenMatchMode::MatchWidthOrHeight(value) => {
          match_factor = value.value();
        }
        battlement::PanelScreenMatchMode::Shrink => {
          screen_match_mode = world_wire::PanelScreenMatchMode::Shrink;
        }
        battlement::PanelScreenMatchMode::Expand => {
          screen_match_mode = world_wire::PanelScreenMatchMode::Expand;
        }
      }
      world_wire::PanelScaleMode::ScaleWithScreenSize
    }
  };
  let reference_resolution = screen_size(reference_resolution);
  let clear_color = color(value.color_clear_value);
  let target_texture = value
    .target_texture
    .as_ref()
    .map(|value| builder.create_string(value.as_str()));
  world_wire::PanelSettings::create(
    builder,
    &world_wire::PanelSettingsArgs {
      render_mode: match value.render_mode {
        battlement::PanelRenderMode::ScreenSpaceOverlay => {
          world_wire::PanelRenderMode::ScreenSpaceOverlay
        }
        battlement::PanelRenderMode::WorldSpace => world_wire::PanelRenderMode::WorldSpace,
      },
      scale_mode,
      reference_sprite_pixels_per_unit: value.reference_sprite_pixels_per_unit,
      scale,
      reference_dpi,
      fallback_dpi,
      reference_resolution: Some(&reference_resolution),
      screen_match_mode,
      match_factor,
      target_display: value.target_display,
      target_texture,
      clear_depth_stencil: value.clear_depth_stencil,
      clear_color: value.clear_color,
      color_clear_value: Some(&clear_color),
      dynamic_atlas: Some(dynamic_atlas),
    },
  )
}

fn screen_size(value: battlement::ScreenSize) -> common::ScreenSize {
  common::ScreenSize::new(value.width, value.height)
}

fn document_pivot(value: battlement::DocumentPivot) -> world_wire::DocumentPivot {
  match value {
    battlement::DocumentPivot::TopLeft => world_wire::DocumentPivot::TopLeft,
    battlement::DocumentPivot::TopCenter => world_wire::DocumentPivot::TopCenter,
    battlement::DocumentPivot::TopRight => world_wire::DocumentPivot::TopRight,
    battlement::DocumentPivot::MiddleLeft => world_wire::DocumentPivot::MiddleLeft,
    battlement::DocumentPivot::Center => world_wire::DocumentPivot::Center,
    battlement::DocumentPivot::MiddleRight => world_wire::DocumentPivot::MiddleRight,
    battlement::DocumentPivot::BottomLeft => world_wire::DocumentPivot::BottomLeft,
    battlement::DocumentPivot::BottomCenter => world_wire::DocumentPivot::BottomCenter,
    battlement::DocumentPivot::BottomRight => world_wire::DocumentPivot::BottomRight,
  }
}

fn write_geometry_update<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::GeometryObservationUpdate,
) -> WIPOffset<geometry_wire::GeometryObservationUpdate<'a>> {
  let added = value
    .added
    .iter()
    .map(|value| {
      let mut object_id = None;
      let mut display_id = 0;
      let mut camera_kind = geometry_wire::CameraTargetKind::Input;
      let mut camera_object_id = None;
      let mut anchor = None;
      let kind = match &value.target {
        battlement::GeometryObservationTarget::UiElement { object_id: value } => {
          object_id = Some(uuid(value.as_uuid()));
          geometry_wire::GeometryTargetKind::UiElement
        }
        battlement::GeometryObservationTarget::Viewport { display_id: value } => {
          display_id = value.0;
          geometry_wire::GeometryTargetKind::Viewport
        }
        battlement::GeometryObservationTarget::WorldOrigin {
          object_id: value,
          camera,
        } => {
          object_id = Some(uuid(value.as_uuid()));
          (camera_kind, camera_object_id) = geometry_camera(*camera);
          geometry_wire::GeometryTargetKind::WorldOrigin
        }
        battlement::GeometryObservationTarget::WorldAnchor {
          object_id: value,
          anchor: value_anchor,
          camera,
        } => {
          object_id = Some(uuid(value.as_uuid()));
          (camera_kind, camera_object_id) = geometry_camera(*camera);
          anchor = Some(builder.create_string(&value_anchor.0));
          geometry_wire::GeometryTargetKind::WorldAnchor
        }
        battlement::GeometryObservationTarget::WorldRenderedBounds {
          object_id: value,
          camera,
        } => {
          object_id = Some(uuid(value.as_uuid()));
          (camera_kind, camera_object_id) = geometry_camera(*camera);
          geometry_wire::GeometryTargetKind::WorldRenderedBounds
        }
      };
      let target = geometry_wire::GeometryObservationTarget::create(
        builder,
        &geometry_wire::GeometryObservationTargetArgs {
          kind,
          object_id: object_id.as_ref(),
          display_id,
          camera_kind,
          camera_object_id: camera_object_id.as_ref(),
          anchor,
        },
      );
      let observation_id = uuid(value.observation_id.0.as_uuid());
      geometry_wire::GeometryObservation::create(
        builder,
        &geometry_wire::GeometryObservationArgs {
          observation_id: Some(&observation_id),
          target: Some(target),
        },
      )
    })
    .collect::<Vec<_>>();
  let added = builder.create_vector(&added);
  let removed = value
    .removed
    .iter()
    .map(|value| uuid(value.0.as_uuid()))
    .collect::<Vec<_>>();
  let removed = builder.create_vector(&removed);
  geometry_wire::GeometryObservationUpdate::create(
    builder,
    &geometry_wire::GeometryObservationUpdateArgs {
      added: Some(added),
      removed: Some(removed),
    },
  )
}

fn geometry_camera(
  value: battlement::CameraTarget,
) -> (geometry_wire::CameraTargetKind, Option<common::Uuid>) {
  match value {
    battlement::CameraTarget::Input => (geometry_wire::CameraTargetKind::Input, None),
    battlement::CameraTarget::Object(value) => (
      geometry_wire::CameraTargetKind::Object,
      Some(uuid(value.as_uuid())),
    ),
  }
}

fn write_accessibility_update<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::AccessibilityUpdate,
) -> WIPOffset<accessibility_wire::AccessibilityUpdate<'a>> {
  let snapshot = value
    .snapshot
    .as_ref()
    .map(|value| write_accessibility_snapshot(builder, value));
  let announcements = value
    .announcements
    .iter()
    .map(|value| builder.create_string(value))
    .collect::<Vec<_>>();
  let announcements = builder.create_vector(&announcements);
  accessibility_wire::AccessibilityUpdate::create(
    builder,
    &accessibility_wire::AccessibilityUpdateArgs {
      snapshot,
      announcements: Some(announcements),
    },
  )
}

fn write_accessibility_snapshot<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::AccessibilitySnapshot,
) -> WIPOffset<accessibility_wire::AccessibilitySnapshot<'a>> {
  let roots = value
    .roots
    .iter()
    .map(|value| uuid(value.as_uuid()))
    .collect::<Vec<_>>();
  let roots = builder.create_vector(&roots);
  let nodes = value
    .nodes
    .iter()
    .map(|value| write_accessibility_node(builder, value))
    .collect::<Vec<_>>();
  let nodes = builder.create_vector(&nodes);
  accessibility_wire::AccessibilitySnapshot::create(
    builder,
    &accessibility_wire::AccessibilitySnapshotArgs {
      commit_sequence: value.commit_sequence,
      roots: Some(roots),
      nodes: Some(nodes),
    },
  )
}

fn write_accessibility_node<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &battlement::AccessibilityNodeSnapshot,
) -> WIPOffset<accessibility_wire::AccessibilityNodeSnapshot<'a>> {
  let object_id = uuid(value.object_id.as_uuid());
  let parent_id = value.parent_id.as_ref().map(|value| uuid(value.as_uuid()));
  let children = value
    .children
    .iter()
    .map(|value| uuid(value.as_uuid()))
    .collect::<Vec<_>>();
  let children = builder.create_vector(&children);
  let label = value
    .label
    .as_ref()
    .map(|value| builder.create_string(value));
  let hint = value
    .hint
    .as_ref()
    .map(|value| builder.create_string(value));
  let state = accessibility_wire::SemanticState::create(
    builder,
    &accessibility_wire::SemanticStateArgs {
      disabled: value.state.disabled,
      checked: value.state.checked.map(checked_state),
      selected: value.state.selected,
      expanded: value.state.expanded,
      popup: value
        .state
        .popup
        .map(|_| accessibility_wire::PopupKind::ListBox),
      busy: value.state.busy,
      current: value
        .state
        .current
        .map(|_| accessibility_wire::CurrentPage::Page),
    },
  );
  let range = value.value.as_ref().map(|value| {
    let text = value
      .text
      .as_ref()
      .map(|value| builder.create_string(value));
    accessibility_wire::AccessibilityRangeValue::create(
      builder,
      &accessibility_wire::AccessibilityRangeValueArgs {
        current: value.current,
        minimum: value.minimum,
        maximum: value.maximum,
        text,
      },
    )
  });
  let scroll = value
    .actions
    .scroll
    .iter()
    .copied()
    .map(accessibility_scroll_direction)
    .collect::<Vec<_>>();
  let scroll = builder.create_vector(&scroll);
  let actions = accessibility_wire::AccessibilityActionSet::create(
    builder,
    &accessibility_wire::AccessibilityActionSetArgs {
      activate: value.actions.activate,
      increment: value.actions.increment,
      decrement: value.actions.decrement,
      dismiss: value.actions.dismiss,
      scroll: Some(scroll),
    },
  );
  accessibility_wire::AccessibilityNodeSnapshot::create(
    builder,
    &accessibility_wire::AccessibilityNodeSnapshotArgs {
      object_id: Some(&object_id),
      parent_id: parent_id.as_ref(),
      children: Some(children),
      role: semantic_role(value.role),
      label,
      hint,
      state: Some(state),
      value: range,
      actions: Some(actions),
      heading_level: value.heading_level,
      scroll_axis: value.scroll_axis.map(|value| match value {
        battlement::AccessibilityScrollAxis::Horizontal => {
          accessibility_wire::AccessibilityScrollAxis::Horizontal
        }
        battlement::AccessibilityScrollAxis::Vertical => {
          accessibility_wire::AccessibilityScrollAxis::Vertical
        }
      }),
    },
  )
}

fn checked_state(value: battlement::CheckedState) -> accessibility_wire::CheckedState {
  match value {
    battlement::CheckedState::False => accessibility_wire::CheckedState::False,
    battlement::CheckedState::True => accessibility_wire::CheckedState::True,
    battlement::CheckedState::Mixed => accessibility_wire::CheckedState::Mixed,
  }
}

fn accessibility_scroll_direction(
  value: battlement::AccessibilityScrollDirection,
) -> accessibility_wire::AccessibilityScrollDirection {
  match value {
    battlement::AccessibilityScrollDirection::Forward => {
      accessibility_wire::AccessibilityScrollDirection::Forward
    }
    battlement::AccessibilityScrollDirection::Backward => {
      accessibility_wire::AccessibilityScrollDirection::Backward
    }
  }
}

fn semantic_role(value: battlement::SemanticRole) -> accessibility_wire::SemanticRole {
  use accessibility_wire::SemanticRole as Wire;
  use battlement::SemanticRole as Role;
  match value {
    Role::Button => Wire::Button,
    Role::Checkbox => Wire::Checkbox,
    Role::Switch => Wire::Switch,
    Role::Radio => Wire::Radio,
    Role::RadioGroup => Wire::RadioGroup,
    Role::Slider => Wire::Slider,
    Role::Progress => Wire::Progress,
    Role::Disclosure => Wire::Disclosure,
    Role::ScrollArea => Wire::ScrollArea,
    Role::Tab => Wire::Tab,
    Role::TabList => Wire::TabList,
    Role::TabPanel => Wire::TabPanel,
    Role::Dialog => Wire::Dialog,
    Role::Heading => Wire::Heading,
    Role::Image => Wire::Image,
    Role::StaticText => Wire::StaticText,
    Role::Group => Wire::Group,
    Role::ListBox => Wire::ListBox,
    Role::Option => Wire::Option,
    Role::Table => Wire::Table,
    Role::Row => Wire::Row,
    Role::ColumnHeader => Wire::ColumnHeader,
    Role::RowHeader => Wire::RowHeader,
    Role::Cell => Wire::Cell,
    Role::Link => Wire::Link,
    Role::Navigation => Wire::Navigation,
    Role::Region => Wire::Region,
  }
}

#[cfg(any(test, feature = "test-support"))]
fn finish(builder: FlatBufferBuilder<'_>) -> Result<FinishedMessage, ProtocolError> {
  let (storage, start) = builder.collapse();
  let message = FinishedMessage::from_storage(storage, start);
  if message.as_bytes().len() > MAXIMUM_MESSAGE_BYTES {
    return Err(ProtocolError::new("response exceeds 16 MiB"));
  }
  ResponseView::read(message.as_bytes())?;
  Ok(message)
}

fn validate_response(value: wire::Response<'_>) -> Result<(), ProtocolError> {
  require_uuid(value.session_id(), "response session")?;
  for entry in value.messages() {
    match entry.message_type() {
      wire::ResponseMessage::Batch => {
        let batch = entry
          .message_as_batch()
          .ok_or_else(|| ProtocolError::new("response message kind/payload mismatch"))?;
        require_uuid(batch.batch_id(), "batch")?;
        require_uuid(batch.session_id(), "batch session")?;
        if uuid_bytes(batch.session_id()) != uuid_bytes(value.session_id()) {
          return Err(ProtocolError::new("batch response session mismatch"));
        }
        if let Some(action_id) = batch.caused_by_action_id() {
          require_uuid(action_id, "causing action")?;
        }
        if !matches!(
          batch.start(),
          wire::BatchStart::Now
            | wire::BatchStart::AfterEarlierBlockingWork
            | wire::BatchStart::AfterEarlierAssetPreparation
        ) {
          return Err(ProtocolError::new("batch start is unknown"));
        }
        if batch.work_scope() == Some(0) || batch.cancel_scope() == Some(0) {
          return Err(ProtocolError::new("work scope must be nonzero"));
        }
        if batch.cancel_scope().is_some() {
          if batch.work_scope().is_some() || batch.start() != wire::BatchStart::Now {
            return Err(ProtocolError::new(
              "cancellation must be independent unowned work",
            ));
          }
        } else if batch.groups().is_empty() {
          return Err(ProtocolError::new("batch has no parallel groups"));
        }
        for group in batch.groups() {
          if group.commands().is_empty() {
            return Err(ProtocolError::new("parallel group has no commands"));
          }
          for entry in group.commands() {
            if entry.command_type() != wire::CommandEntryPayload::CoreCommand {
              return Err(ProtocolError::new("core command entry kind is unknown"));
            }
            let command = entry
              .command_as_core_command()
              .ok_or_else(|| ProtocolError::new("core command entry payload is absent"))?;
            if batch.cancel_scope().is_some()
              && !matches!(
                command.kind(),
                wire::CoreCommandKind::VisualElementDestroy | wire::CoreCommandKind::ObjectDestroy
              )
            {
              return Err(ProtocolError::new(
                "cancellation cleanup may only destroy objects",
              ));
            }
            validate_command(command)?;
          }
        }
      }
      wire::ResponseMessage::Snapshot => {
        let snapshot = entry
          .message_as_snapshot()
          .ok_or_else(|| ProtocolError::new("response message kind/payload mismatch"))?;
        require_uuid(snapshot.session_id(), "snapshot session")?;
        if uuid_bytes(snapshot.session_id()) != uuid_bytes(value.session_id()) {
          return Err(ProtocolError::new("snapshot response session mismatch"));
        }
        crate::response_validate::validate_snapshot(snapshot)?;
      }
      _ => return Err(ProtocolError::new("response message kind is unknown")),
    }
  }
  Ok(())
}

fn validate_command(value: wire::CoreCommand<'_>) -> Result<(), ProtocolError> {
  require_uuid(value.command_id(), "command")?;
  let expected = match value.kind() {
    wire::CoreCommandKind::ApplicationOpenUrl => wire::CoreCommandPayload::ExternalUrlPayload,
    wire::CoreCommandKind::Diagnostics => wire::CoreCommandPayload::DiagnosticsPayload,
    wire::CoreCommandKind::AssetsReplaceSet => wire::CoreCommandPayload::ReplaceAssetSetPayload,
    wire::CoreCommandKind::SceneLoad => wire::CoreCommandPayload::SceneLoadPayload,
    wire::CoreCommandKind::SceneUnload | wire::CoreCommandKind::SceneSetPrimary => {
      wire::CoreCommandPayload::SceneIdPayload
    }
    wire::CoreCommandKind::ObjectCreate => wire::CoreCommandPayload::ObjectCreatePayload,
    wire::CoreCommandKind::ObjectDestroy | wire::CoreCommandKind::InputSetCamera => {
      wire::CoreCommandPayload::ObjectIdPayload
    }
    wire::CoreCommandKind::ObjectSetActive => wire::CoreCommandPayload::ObjectSetActivePayload,
    wire::CoreCommandKind::ObjectReparent => wire::CoreCommandPayload::ObjectReparentPayload,
    wire::CoreCommandKind::TransformSetLocalPosition
    | wire::CoreCommandKind::TransformSetWorldPosition => wire::CoreCommandPayload::PositionPayload,
    wire::CoreCommandKind::TransformTweenLocalPosition
    | wire::CoreCommandKind::TransformTweenWorldPosition => {
      wire::CoreCommandPayload::TweenPositionPayload
    }
    wire::CoreCommandKind::TransformSetLocalRotation
    | wire::CoreCommandKind::TransformSetWorldRotation => wire::CoreCommandPayload::RotationPayload,
    wire::CoreCommandKind::TransformTweenLocalRotation
    | wire::CoreCommandKind::TransformTweenWorldRotation => {
      wire::CoreCommandPayload::TweenRotationPayload
    }
    wire::CoreCommandKind::TransformSetLocalScale => wire::CoreCommandPayload::ScalePayload,
    wire::CoreCommandKind::TransformTweenLocalScale => wire::CoreCommandPayload::TweenScalePayload,
    wire::CoreCommandKind::RendererSetMaterial => wire::CoreCommandPayload::SetMaterialPayload,
    wire::CoreCommandKind::CameraSetEnabled
    | wire::CoreCommandKind::LightSetEnabled
    | wire::CoreCommandKind::ImageSetFaceCamera
    | wire::CoreCommandKind::TextSetRichText
    | wire::CoreCommandKind::TextSetFaceCamera => wire::CoreCommandPayload::ObjectEnabledPayload,
    wire::CoreCommandKind::CameraSetPerspective => wire::CoreCommandPayload::PerspectivePayload,
    wire::CoreCommandKind::CameraTweenFieldOfView => {
      wire::CoreCommandPayload::TweenFieldOfViewPayload
    }
    wire::CoreCommandKind::CameraSetOrthographic => wire::CoreCommandPayload::OrthographicPayload,
    wire::CoreCommandKind::CameraTweenOrthographicSize => {
      wire::CoreCommandPayload::TweenOrthographicSizePayload
    }
    wire::CoreCommandKind::CameraSetClipping => wire::CoreCommandPayload::CameraClippingPayload,
    wire::CoreCommandKind::CameraSetClear => wire::CoreCommandPayload::CameraClearPayload,
    wire::CoreCommandKind::LightSetType => wire::CoreCommandPayload::LightTypePayload,
    wire::CoreCommandKind::LightSetColor | wire::CoreCommandKind::TextSetColor => {
      wire::CoreCommandPayload::ColorPayload
    }
    wire::CoreCommandKind::LightTweenColor | wire::CoreCommandKind::TextTweenColor => {
      wire::CoreCommandPayload::TweenColorPayload
    }
    wire::CoreCommandKind::LightSetIntensity => wire::CoreCommandPayload::IntensityPayload,
    wire::CoreCommandKind::LightTweenIntensity => wire::CoreCommandPayload::TweenIntensityPayload,
    wire::CoreCommandKind::LightSetRange => wire::CoreCommandPayload::LightRangePayload,
    wire::CoreCommandKind::LightSetSpotAngle => wire::CoreCommandPayload::SpotAnglePayload,
    wire::CoreCommandKind::LightSetShadows => wire::CoreCommandPayload::LightShadowsPayload,
    wire::CoreCommandKind::ImageSetTexture => wire::CoreCommandPayload::SetTexturePayload,
    wire::CoreCommandKind::TextSetFont => wire::CoreCommandPayload::SetFontPayload,
    wire::CoreCommandKind::ImageSetSize => wire::CoreCommandPayload::ImageSizePayload,
    wire::CoreCommandKind::ImageSetFit => wire::CoreCommandPayload::ImageFitPayload,
    wire::CoreCommandKind::ImageSetTint => wire::CoreCommandPayload::TintPayload,
    wire::CoreCommandKind::ImageTweenTint => wire::CoreCommandPayload::TweenTintPayload,
    wire::CoreCommandKind::ImageSetOpacity => wire::CoreCommandPayload::OpacityPayload,
    wire::CoreCommandKind::ImageTweenOpacity => wire::CoreCommandPayload::TweenOpacityPayload,
    wire::CoreCommandKind::TextSetContent => wire::CoreCommandPayload::TextContentPayload,
    wire::CoreCommandKind::TextSetSize => wire::CoreCommandPayload::TextSizePayload,
    wire::CoreCommandKind::TextTweenSize => wire::CoreCommandPayload::TweenTextSizePayload,
    wire::CoreCommandKind::TextSetAlignment => wire::CoreCommandPayload::TextAlignmentPayload,
    wire::CoreCommandKind::TextSetWrapping => wire::CoreCommandPayload::TextWrappingPayload,
    wire::CoreCommandKind::AnimatorPlay => wire::CoreCommandPayload::AnimatorPlayPayload,
    wire::CoreCommandKind::AnimatorCrossFade => wire::CoreCommandPayload::AnimatorCrossFadePayload,
    wire::CoreCommandKind::AnimatorSetBool => wire::CoreCommandPayload::AnimatorBoolPayload,
    wire::CoreCommandKind::AnimatorSetInt => wire::CoreCommandPayload::AnimatorIntPayload,
    wire::CoreCommandKind::AnimatorSetFloat => wire::CoreCommandPayload::AnimatorFloatPayload,
    wire::CoreCommandKind::AnimatorSetTrigger => wire::CoreCommandPayload::AnimatorParameterPayload,
    wire::CoreCommandKind::AnimatorSetSpeed => wire::CoreCommandPayload::AnimatorSpeedPayload,
    wire::CoreCommandKind::ParticlePlay => wire::CoreCommandPayload::ParticlePlayPayload,
    wire::CoreCommandKind::ParticleStop => wire::CoreCommandPayload::ParticleStopPayload,
    wire::CoreCommandKind::ParticleSpawn => wire::CoreCommandPayload::ParticleSpawnPayload,
    wire::CoreCommandKind::AudioPlay => wire::CoreCommandPayload::AudioPlayPayload,
    wire::CoreCommandKind::AudioStop => wire::CoreCommandPayload::AudioStopPayload,
    wire::CoreCommandKind::AudioPause | wire::CoreCommandKind::AudioResume => {
      wire::CoreCommandPayload::AudioPlaybackPayload
    }
    wire::CoreCommandKind::AudioSeek => wire::CoreCommandPayload::AudioSeekPayload,
    wire::CoreCommandKind::AudioSetBuffering => wire::CoreCommandPayload::AudioBufferingPayload,
    wire::CoreCommandKind::AudioReplace => wire::CoreCommandPayload::AudioReplacePayload,
    wire::CoreCommandKind::AudioSetVolume => wire::CoreCommandPayload::AudioVolumePayload,
    wire::CoreCommandKind::AudioTweenVolume => wire::CoreCommandPayload::TweenAudioVolumePayload,
    wire::CoreCommandKind::TimeWait => wire::CoreCommandPayload::WaitPayload,
    wire::CoreCommandKind::OperationCancel => wire::CoreCommandPayload::CancelOperationPayload,
    wire::CoreCommandKind::InputSetEnabled => wire::CoreCommandPayload::SetInputEnabledPayload,
    wire::CoreCommandKind::InputSetPointerEvents => wire::CoreCommandPayload::PointerEventsPayload,
    wire::CoreCommandKind::InputSetGlobalKeys => wire::CoreCommandPayload::GlobalKeysPayload,
    wire::CoreCommandKind::InputSetController => wire::CoreCommandPayload::ControllerInputSettings,
    wire::CoreCommandKind::ControllerVibrate => {
      wire::CoreCommandPayload::ControllerVibrationPayload
    }
    wire::CoreCommandKind::DebugUi => wire::CoreCommandPayload::DebugUiPayload,
    wire::CoreCommandKind::VisualElementCreate => {
      wire::CoreCommandPayload::VisualElementCreatePayload
    }
    wire::CoreCommandKind::VisualElementUpdate => {
      wire::CoreCommandPayload::VisualElementUpdatePayload
    }
    wire::CoreCommandKind::VisualElementDestroy => {
      wire::CoreCommandPayload::VisualElementDestroyPayload
    }
    wire::CoreCommandKind::VisualElementPerformAction => {
      wire::CoreCommandPayload::VisualElementActionPayload
    }
    wire::CoreCommandKind::MotionValue => wire::CoreCommandPayload::MotionValueOperation,
    wire::CoreCommandKind::MotionValuePlayback => {
      wire::CoreCommandPayload::MotionValuePlaybackOperation
    }
    wire::CoreCommandKind::MotionPlayback => wire::CoreCommandPayload::MotionPlaybackOperation,
    wire::CoreCommandKind::MotionControlledClock => {
      wire::CoreCommandPayload::MotionControlledClockOperation
    }
    wire::CoreCommandKind::MotionControl => wire::CoreCommandPayload::MotionControlOperation,
    wire::CoreCommandKind::MotionScope => wire::CoreCommandPayload::MotionScopeOperation,
    wire::CoreCommandKind::MotionDragControl => {
      wire::CoreCommandPayload::MotionDragControlOperation
    }
    wire::CoreCommandKind::GeometryObservationUpdate => {
      wire::CoreCommandPayload::GeometryObservationUpdate
    }
    wire::CoreCommandKind::AccessibilityUpdate => wire::CoreCommandPayload::AccessibilityUpdate,
    kind if kind.variant_name().is_some() => {
      return Err(ProtocolError::new(
        "response command kind is not admitted by this schema revision",
      ));
    }
    _ => return Err(ProtocolError::new("response command kind is unknown")),
  };
  if value.payload_type() != expected {
    return Err(ProtocolError::new("response command kind/payload mismatch"));
  }
  match value.kind() {
    wire::CoreCommandKind::VisualElementCreate => crate::response_validate::validate_create(
      value
        .payload_as_visual_element_create_payload()
        .expect("kind/payload checked"),
    )?,
    wire::CoreCommandKind::VisualElementUpdate => crate::response_validate::validate_update(
      value
        .payload_as_visual_element_update_payload()
        .expect("kind/payload checked"),
    )?,
    wire::CoreCommandKind::VisualElementDestroy => crate::response_validate::validate_destroy(
      value
        .payload_as_visual_element_destroy_payload()
        .expect("kind/payload checked"),
    )?,
    wire::CoreCommandKind::VisualElementPerformAction => crate::response_validate::validate_action(
      value
        .payload_as_visual_element_action_payload()
        .expect("kind/payload checked"),
    )?,
    _ => {}
  }
  Ok(())
}

fn require_uuid(value: &common::Uuid, name: &str) -> Result<(), ProtocolError> {
  if value.bytes().iter().all(|byte| byte == 0) {
    Err(ProtocolError::new(format!("{name} UUID is zero")))
  } else {
    Ok(())
  }
}

fn uuid_bytes(value: &common::Uuid) -> [u8; 16] {
  std::array::from_fn(|index| value.bytes().get(index))
}

fn verifier_options() -> VerifierOptions {
  VerifierOptions {
    max_depth: MAXIMUM_TABLE_DEPTH,
    max_tables: MAXIMUM_TABLE_VISITS,
    max_apparent_size: MAXIMUM_APPARENT_BYTES,
    ignore_missing_null_terminator: false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_response_is_size_prefixed_verified_and_borrowed() {
    let id = [7; 16];
    let bytes = write_empty_response(id).expect("write response");
    let view = ResponseView::read(bytes.as_bytes()).expect("read response");
    assert_eq!(view.session_id(), id);
    assert_eq!(view.message_count(), 0);
  }

  #[test]
  fn response_rejects_zero_uuid_and_broken_prefix() {
    assert!(write_empty_response([0; 16]).is_err());
    let mut bytes = write_empty_response([9; 16])
      .expect("write response")
      .as_bytes()
      .to_vec();
    bytes[0] ^= 1;
    assert!(ResponseView::read(&bytes).is_err());
  }

  #[test]
  fn response_rejects_every_truncated_finished_range_and_hostile_root_offset() {
    let bytes = write_empty_response([0x6d; 16])
      .expect("write response")
      .as_bytes()
      .to_vec();
    for length in 0..bytes.len() {
      assert!(
        ResponseView::read(&bytes[..length]).is_err(),
        "accepted truncated response length {length}"
      );
    }
    let mut hostile = bytes;
    hostile[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(ResponseView::read(&hostile).is_err());
  }

  #[test]
  fn core_response_writes_typed_command() {
    let session_id = battlement::SessionId::new_v4();
    let response = Response::commands(
      session_id,
      [CommandBody::InputSetEnabled(
        battlement::SetInputEnabledPayload { enabled: false },
      )],
    );
    let bytes = write_core_response(&response).expect("write core response");
    let view = ResponseView::read(bytes.as_bytes()).expect("read core response");
    assert_eq!(view.session_id(), *session_id.as_uuid().as_bytes());
    assert_eq!(view.message_count(), 1);
  }

  #[test]
  fn snapshot_flattens_typed_ui_nodes_and_styles() {
    let session_id = battlement::SessionId::new_v4();
    let document_id = battlement::ObjectId::new_v4();
    let root_id = battlement::ObjectId::new_v4();
    let child_id = battlement::ObjectId::new_v4();
    let document =
      battlement::UiDocument::with_root_id(document_id, root_id).child(battlement::UiNode::new(
        child_id,
        battlement::UiLabel::new("ready").style(
          battlement::Style::new()
            .color(battlement::Color::WHITE)
            .padding(12.0),
        ),
      ));
    let scene_id = battlement::SceneId::new_v4();
    let response = Response::snapshot(
      battlement::Snapshot::new_with_main_camera(
        session_id,
        vec![battlement::PreparedAsset::scene("scene/main")],
        vec![battlement::Scene::new(scene_id, "scene/main")],
        Vec::new(),
      )
      .ui_document(document),
    );

    let bytes = write_core_response(&response).expect("write typed UI snapshot");
    let view = ResponseView::read(bytes.as_bytes()).expect("read typed UI snapshot");
    assert_eq!(view.session_id(), *session_id.as_uuid().as_bytes());
    assert_eq!(view.message_count(), 1);
  }

  #[test]
  fn visual_commands_write_closed_typed_payloads() {
    let session_id = battlement::SessionId::new_v4();
    let parent_id = battlement::ObjectId::new_v4();
    let object_id = battlement::ObjectId::new_v4();
    let descendant_id = battlement::ObjectId::new_v4();
    let bodies = vec![
      CommandBody::VisualElementCreate(Box::new(
        battlement::VisualElementCreate::new(
          parent_id,
          battlement::UiNode::new(object_id, battlement::UiLabel::new("created")),
        )
        .child_index(2),
      )),
      CommandBody::VisualElementUpdate(Box::new(battlement::VisualElementUpdate::Properties {
        object_id,
        element: Box::new(battlement::UiLabel::new("updated").into()),
      })),
      CommandBody::VisualElementUpdate(Box::new(battlement::VisualElementUpdate::Parent {
        object_id,
        parent_id,
        child_index: Some(1),
      })),
      CommandBody::VisualElementUpdate(Box::new(battlement::VisualElementUpdate::Index {
        object_id,
        child_index: 3,
      })),
      CommandBody::VisualElementDestroy(battlement::VisualElementDestroy { object_id }),
      CommandBody::VisualElementPerformAction(battlement::VisualElementPerformAction {
        object_id,
        action: battlement::VisualElementAction::ScrollTo { descendant_id },
      }),
      CommandBody::VisualElementPerformAction(battlement::VisualElementPerformAction {
        object_id,
        action: battlement::VisualElementAction::SelectText {
          cursor_index: 4,
          selection_index: 9,
        },
      }),
    ];
    let response = Response::commands(session_id, bodies);

    let bytes = write_core_response(&response).expect("write typed visual commands");
    ResponseView::read(bytes.as_bytes()).expect("verify typed visual commands");
    let value = wire::size_prefixed_root_as_response(bytes.as_bytes()).expect("response root");
    let commands = value
      .messages()
      .get(0)
      .message_as_batch()
      .expect("batch")
      .groups()
      .get(0)
      .commands();
    assert_eq!(commands.len(), 7);
    let create = commands
      .get(0)
      .command_as_core_command()
      .expect("core command")
      .payload_as_visual_element_create_payload()
      .expect("create payload");
    assert_eq!(create.nodes().len(), 1);
    assert_eq!(create.child_index(), Some(2));
    let parent = commands
      .get(2)
      .command_as_core_command()
      .expect("core command")
      .payload_as_visual_element_update_payload()
      .expect("parent payload");
    assert_eq!(parent.kind(), ui_wire::VisualElementUpdateKind::Parent);
    assert_eq!(parent.child_index(), Some(1));
    let action = commands
      .get(6)
      .command_as_core_command()
      .expect("core command")
      .payload_as_visual_element_action_payload()
      .expect("action payload");
    assert_eq!(action.kind(), ui_wire::VisualElementActionKind::SelectText);
    assert_eq!(action.cursor_index(), 4);
    assert_eq!(action.selection_index(), 9);
  }

  #[test]
  fn motion_commands_write_closed_typed_payloads() {
    let session_id = battlement::SessionId::new_v4();
    let value_id = battlement::ObjectId::new_v4();
    let playback_id = battlement::ObjectId::new_v4();
    let descriptor_id = battlement::ObjectId::new_v4();
    let clock_id = battlement::ObjectId::new_v4();
    let control_id = battlement::ObjectId::new_v4();
    let scope_id = battlement::ObjectId::new_v4();
    let target = battlement::MotionTargetDescriptor {
      tracks: vec![battlement::MotionPropertyTrack {
        property: battlement::MotionProperty::Opacity,
        values: vec![battlement::MotionValue::Scalar(0.25)],
        times: None,
        transition: battlement::TransitionDefinition::tween(),
      }],
      transition_end: Vec::new(),
    };
    let bodies = vec![
      CommandBody::MotionValue(battlement::MotionValueOperation {
        value_id,
        command: battlement::MotionValueCommand::Animate {
          playback_id,
          generation: 4,
          target: Box::new(battlement::MotionValue::Scalar(3.5)),
          transition: battlement::TransitionDefinition::spring(),
        },
      }),
      CommandBody::MotionValuePlayback(battlement::MotionValuePlaybackOperation {
        playback_id,
        generation: 4,
        command: battlement::MotionPlaybackCommand::SetSpeed { value: 1.5 },
      }),
      CommandBody::MotionPlayback(battlement::MotionPlaybackOperation {
        descriptor_id,
        slot: battlement::MotionSlotId(7),
        generation: battlement::MotionGeneration(2),
        command: battlement::MotionPlaybackCommand::Seek {
          elapsed_micros: 90_000,
        },
      }),
      CommandBody::MotionControlledClock(battlement::MotionControlledClockOperation {
        clock_id,
        command: battlement::MotionControlledClockCommand::Advance {
          delta_micros: 16_667,
        },
      }),
      CommandBody::MotionControl(battlement::MotionControlOperation {
        control_id,
        command: battlement::MotionControlCommand::Start {
          playback_id,
          generation: 5,
          target: battlement::MotionControlTarget::Target(target.clone()),
        },
      }),
      CommandBody::MotionScope(battlement::MotionScopeOperation {
        scope_id,
        command: battlement::MotionScopeCommand::Set {
          selector: battlement::MotionSelector::Descendants,
          target,
        },
      }),
      CommandBody::MotionDragControl(battlement::MotionDragControlOperation {
        control_id,
        pointer_id: 11,
        device: battlement::MotionPointerDevice::Touch,
        point: battlement::MotionGestureVector { x: 12.0, y: 34.0 },
        snap_to_cursor: true,
      }),
    ];
    let response = Response::commands(session_id, bodies);

    let bytes = write_core_response(&response).expect("write typed motion commands");
    ResponseView::read(bytes.as_bytes()).expect("verify typed motion commands");
    let value = wire::size_prefixed_root_as_response(bytes.as_bytes()).expect("response root");
    let commands = value
      .messages()
      .get(0)
      .message_as_batch()
      .expect("batch")
      .groups()
      .get(0)
      .commands();
    assert_eq!(commands.len(), 7);
    let value = commands
      .get(0)
      .command_as_core_command()
      .expect("core command")
      .payload_as_motion_value_operation()
      .expect("motion value operation");
    assert_eq!(
      value.command(),
      crate::motion_generated::MotionValueCommandKind::Animate
    );
    assert_eq!(value.generation(), 4);
    let drag = commands
      .get(6)
      .command_as_core_command()
      .expect("core command")
      .payload_as_motion_drag_control_operation()
      .expect("motion drag operation");
    assert_eq!(drag.pointer_id(), 11);
    assert!(drag.snap_to_cursor());
  }

  #[test]
  fn ui_motion_descriptor_writes_typed_nested_graph() {
    let session_id = battlement::SessionId::new_v4();
    let document_id = battlement::ObjectId::new_v4();
    let root_id = battlement::ObjectId::new_v4();
    let host_id = battlement::ObjectId::new_v4();
    let value_id = battlement::ObjectId::new_v4();
    let descriptor = battlement::MotionDescriptor {
      descriptor_id: battlement::ObjectId::new_v4(),
      host_id,
      generation: battlement::MotionGeneration(2),
      initial: None,
      initial_disabled: false,
      slots: vec![battlement::MotionSlotDescriptor {
        slot: battlement::MotionSlotId(8),
        generation: battlement::MotionGeneration(3),
        layer: battlement::MotionLayer::Animate,
        target: battlement::MotionTargetDescriptor {
          tracks: vec![battlement::MotionPropertyTrack {
            property: battlement::MotionProperty::Opacity,
            values: vec![battlement::MotionValue::Scalar(0.75)],
            times: None,
            transition: battlement::TransitionDefinition::tween(),
          }],
          transition_end: Vec::new(),
        },
        callbacks: battlement::MotionCallbackSubscriptions::default(),
      }],
      clock: battlement::MotionClockSource::Unscaled,
      reduced_motion: battlement::ReducedMotionPolicy::Never,
      pseudo_styles: Vec::new(),
      style_transition: battlement::StyleTransitionDescriptor::default(),
      animations: Vec::new(),
      decorations: Vec::new(),
      variants: None,
      values: vec![battlement::MotionValueDescriptor {
        value_id,
        initial: battlement::MotionValue::Scalar(1.0),
        source: battlement::MotionValueSource::Mutable,
      }],
      value_bindings: vec![battlement::MotionValueBinding {
        property: battlement::MotionProperty::FlexGrow,
        value_id,
        composition: battlement::MotionBindingComposition::Replace,
      }],
      value_subscriptions: Vec::new(),
      control_id: None,
      scope_id: None,
      scope_root: false,
      motion_name: Some("card".to_owned()),
      named_targets: Vec::new(),
      gestures: None,
      layout: None,
    };
    let document =
      battlement::UiDocument::with_root_id(document_id, root_id).child(battlement::UiNode::new(
        host_id,
        battlement::UiLabel::new("motion").motion_descriptor(descriptor),
      ));
    let scene_id = battlement::SceneId::new_v4();
    let response = Response::snapshot(
      battlement::Snapshot::new_with_main_camera(
        session_id,
        vec![battlement::PreparedAsset::scene("scene/main")],
        vec![battlement::Scene::new(scene_id, "scene/main")],
        Vec::new(),
      )
      .ui_document(document),
    );

    let bytes = write_core_response(&response).expect("write UI motion descriptor");
    ResponseView::read(bytes.as_bytes()).expect("verify UI motion descriptor");
  }
}
