use battlement::{AnyCommand, BatchStart, Response, ResponseMessage, Validate};
use battlement_flatbuffers::{
  FinishedMessage, MAXIMUM_APPARENT_BYTES, MAXIMUM_MESSAGE_BYTES, MAXIMUM_TABLE_DEPTH,
  MAXIMUM_TABLE_VISITS, write_composed_core_command, write_composed_snapshot,
};
use battlement_native::EngineError;
use flatbuffers::{FlatBufferBuilder, VerifierOptions};

use crate::{
  FlashPayload,
  fixture_response_generated::battlement::flat_buffers::{fixture_generated as wire, generated},
};

pub(crate) fn validate_fixture_client(bytes: &[u8]) -> Result<[u8; 16], EngineError> {
  if !(12..=MAXIMUM_MESSAGE_BYTES).contains(&bytes.len()) {
    return Err(EngineError::new("fixture client message size is invalid"));
  }
  let declared = u32::from_le_bytes(bytes[..4].try_into().expect("length checked")) as usize;
  if declared != bytes.len() - 4 || !flatbuffers::buffer_has_identifier(bytes, "BTCM", true) {
    return Err(EngineError::new(
      "fixture client message prefix or identifier is invalid",
    ));
  }
  let options = VerifierOptions {
    max_depth: MAXIMUM_TABLE_DEPTH,
    max_tables: MAXIMUM_TABLE_VISITS,
    max_apparent_size: MAXIMUM_APPARENT_BYTES,
    ignore_missing_null_terminator: false,
  };
  let root =
    flatbuffers::size_prefixed_root_with_opts::<wire::FixtureClientMessage>(&options, bytes)
      .map_err(|error| EngineError::new(format!("invalid fixture client message: {error}")))?;
  match root.body_type() {
    wire::FixtureClientBody::FixtureAction => {
      let action = root
        .body_as_fixture_action()
        .ok_or_else(|| EngineError::new("fixture action payload is absent"))?;
      require_fixture_uuid(action.action_id(), "action")?;
      let session = require_fixture_uuid(action.session_id(), "session")?;
      if !action.action_type().starts_with("fixture.")
        || !action.payload().scale().is_finite()
        || action.payload().scale() < 0.0
      {
        return Err(EngineError::new("fixture action is noncanonical"));
      }
      require_fixture_uuid(action.payload().object_id(), "flash object")?;
      Ok(session)
    }
    wire::FixtureClientBody::FixtureBatchFailed => {
      let failure = root
        .body_as_fixture_batch_failed()
        .ok_or_else(|| EngineError::new("fixture batch failure payload is absent"))?;
      let session = require_fixture_uuid(failure.session_id(), "session")?;
      require_fixture_uuid(failure.batch_id(), "batch")?;
      if let Some(command) = failure.command_id() {
        require_fixture_uuid(command, "command")?;
      }
      require_fixture_error(failure.error())?;
      Ok(session)
    }
    wire::FixtureClientBody::FixtureOperationFailed => {
      let failure = root
        .body_as_fixture_operation_failed()
        .ok_or_else(|| EngineError::new("fixture operation failure payload is absent"))?;
      let session = require_fixture_uuid(failure.session_id(), "session")?;
      require_fixture_uuid(failure.batch_id(), "batch")?;
      require_fixture_uuid(failure.command_id(), "command")?;
      require_fixture_error(failure.error())?;
      Ok(session)
    }
    _ => Err(EngineError::new("fixture client message tag is unknown")),
  }
}

fn require_fixture_uuid(value: &generated::Uuid, field: &str) -> Result<[u8; 16], EngineError> {
  let bytes: [u8; 16] = value.bytes().into();
  if bytes == [0; 16] {
    return Err(EngineError::new(format!("{field} UUID is zero")));
  }
  Ok(bytes)
}

fn require_fixture_error(value: wire::FixtureError) -> Result<(), EngineError> {
  if value.variant_name().is_none() {
    return Err(EngineError::new("fixture error code is unknown"));
  }
  Ok(())
}

pub(crate) const WIRE_DIGEST_C: &[u8; 65] =
  b"d036677cfb0e1696942f589875410417b1e0314464cb157d71a6cc31dbd37c3c\0";

pub(crate) fn write_response(
  response: &Response<AnyCommand<FlashPayload>>,
) -> Result<FinishedMessage, EngineError> {
  validate_owned_response(response)?;
  let mut builder = FlatBufferBuilder::with_capacity(4 * 1024);
  let mut messages = Vec::with_capacity(response.messages.len());
  for message in &response.messages {
    let (message_type, message) = match message {
      ResponseMessage::Snapshot(snapshot) => (
        wire::ResponseMessage::Battlement_FlatBuffers_Generated_Snapshot,
        write_composed_snapshot(&mut builder, snapshot).map_err(protocol_error)?,
      ),
      ResponseMessage::Batch(batch) => {
        let mut groups = Vec::with_capacity(batch.groups.len());
        for group in &batch.groups {
          let mut commands = Vec::with_capacity(group.commands.len());
          for command in &group.commands {
            let (command_type, command) = match command {
              AnyCommand::Core(command) => (
                wire::FixtureCommand::Battlement_FlatBuffers_Generated_CoreCommand,
                write_composed_core_command(&mut builder, command).map_err(protocol_error)?,
              ),
              AnyCommand::Custom(command) => {
                let command_type = builder.create_string(&command.command_type);
                let object_id = uuid(command.payload.object_id().as_uuid().as_bytes());
                let payload = wire::FlashPayload::create(
                  &mut builder,
                  &wire::FlashPayloadArgs {
                    object_id: Some(&object_id),
                    scale: command.payload.scale(),
                  },
                );
                let command_id = uuid(command.command_id.as_uuid().as_bytes());
                let command = wire::FlashCommand::create(
                  &mut builder,
                  &wire::FlashCommandArgs {
                    command_id: Some(&command_id),
                    blocking: command.blocking,
                    command_type: Some(command_type),
                    payload: Some(payload),
                  },
                );
                (wire::FixtureCommand::FlashCommand, command.as_union_value())
              }
            };
            commands.push(wire::CommandEntry::create(
              &mut builder,
              &wire::CommandEntryArgs {
                command_type,
                command: Some(command),
              },
            ));
          }
          let commands = builder.create_vector(&commands);
          groups.push(wire::ParallelCommandGroup::create(
            &mut builder,
            &wire::ParallelCommandGroupArgs {
              commands: Some(commands),
            },
          ));
        }
        let groups = builder.create_vector(&groups);
        let batch_id = uuid(batch.batch_id.as_uuid().as_bytes());
        let session_id = uuid(batch.session_id.as_uuid().as_bytes());
        let caused_by_action_id = batch
          .caused_by_action_id
          .as_ref()
          .map(|value| uuid(value.as_uuid().as_bytes()));
        let presentation_control = batch.presentation_control.as_ref().map(|control| {
          let owner_id = uuid(control.owner_id.as_uuid().as_bytes());
          generated::PresentationControl::create(
            &mut builder,
            &generated::PresentationControlArgs {
              work_scope: control.work_scope,
              owner_id: Some(&owner_id),
              paused: control.paused,
            },
          )
        });
        let batch = wire::Batch::create(
            &mut builder,
            &wire::BatchArgs {
              batch_id: Some(&batch_id),
              session_id: Some(&session_id),
              caused_by_action_id: caused_by_action_id.as_ref(),
              work_scope: batch.work_scope,
              cancel_scope: batch.cancel_scope,
              start: match batch.start {
                BatchStart::Now => battlement_flatbuffers::schema_generated::response_generated::battlement::flat_buffers::generated::BatchStart::Now,
                BatchStart::AfterEarlierBlockingWork => battlement_flatbuffers::schema_generated::response_generated::battlement::flat_buffers::generated::BatchStart::AfterEarlierBlockingWork,
                BatchStart::AfterEarlierAssetPreparation => battlement_flatbuffers::schema_generated::response_generated::battlement::flat_buffers::generated::BatchStart::AfterEarlierAssetPreparation,
              },
              groups: Some(groups),
              presentation_control,
            },
          );
        (wire::ResponseMessage::Batch, batch.as_union_value())
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
  let session_id = uuid(response.session_id.as_uuid().as_bytes());
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
  verify_fixture_response(message.as_bytes())?;
  Ok(message)
}

fn validate_owned_response(
  response: &Response<AnyCommand<FlashPayload>>,
) -> Result<(), EngineError> {
  if response.session_id.as_uuid().is_nil() {
    return Err(EngineError::new("response session UUID is zero"));
  }
  for message in &response.messages {
    match message {
      ResponseMessage::Snapshot(snapshot) => {
        if snapshot.session_id != response.session_id {
          return Err(EngineError::new(
            "snapshot session does not match response session",
          ));
        }
        snapshot
          .validate()
          .map_err(|error| EngineError::new(format!("invalid fixture snapshot: {error}")))?;
      }
      ResponseMessage::Batch(batch) if batch.session_id != response.session_id => {
        return Err(EngineError::new(
          "batch session does not match response session",
        ));
      }
      ResponseMessage::Batch(batch) => {
        for group in &batch.groups {
          for command in &group.commands {
            match command {
              AnyCommand::Core(command) => command
                .validate()
                .map_err(|error| EngineError::new(format!("invalid core command: {error}")))?,
              AnyCommand::Custom(command) => {
                if command.command_id.as_uuid().is_nil() {
                  return Err(EngineError::new("custom command UUID is zero"));
                }
                if command.command_type != "fixture.character.flash" {
                  return Err(EngineError::new("unknown fixture custom command type"));
                }
                if !command.payload.scale().is_finite() || command.payload.scale() < 0.0 {
                  return Err(EngineError::new("fixture flash scale is invalid"));
                }
              }
            }
          }
        }
      }
    }
  }
  Ok(())
}

pub(crate) fn verify_fixture_response(bytes: &[u8]) -> Result<[u8; 16], EngineError> {
  if bytes.len() > MAXIMUM_MESSAGE_BYTES {
    return Err(EngineError::new("response exceeds 16 MiB"));
  }
  if bytes.len() < 12 || !wire::response_size_prefixed_buffer_has_identifier(bytes) {
    return Err(EngineError::new("response has the wrong file identifier"));
  }
  let options = VerifierOptions {
    max_depth: MAXIMUM_TABLE_DEPTH,
    max_tables: MAXIMUM_TABLE_VISITS,
    max_apparent_size: MAXIMUM_APPARENT_BYTES,
    ignore_missing_null_terminator: false,
  };
  let response = wire::size_prefixed_root_as_response_with_opts(&options, bytes)
    .map_err(|error| EngineError::new(format!("invalid response FlatBuffer: {error}")))?;
  require_fixture_uuid(response.session_id(), "response session")
}

fn uuid(value: &[u8; 16]) -> battlement_flatbuffers::schema_generated::common_generated::battlement::flat_buffers::generated::Uuid{
  battlement_flatbuffers::schema_generated::common_generated::battlement::flat_buffers::generated::Uuid::new(value)
}

fn protocol_error(error: battlement_flatbuffers::ProtocolError) -> EngineError {
  EngineError::new(error.to_string())
}
