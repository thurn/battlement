use battlement::{Command, CommandBody, ObjectId, WorldPointerPayload, WorldPointerSettings};
use battlement_flatbuffers::schema_generated::response_generated::battlement::flat_buffers::generated as wire;
use battlement_flatbuffers::{self, MessageWriter, NativeBatchStart, ResponseView};
use uuid::Uuid;

#[test]
fn owned_and_direct_pointer_settings_are_verified_and_clearable() {
  for settings in [
    Some(WorldPointerSettings {
      interaction_layer: i32::MIN,
      order: u32::MAX,
      capture_on_press: true,
      focusable: true,
    }),
    None,
  ] {
    for direct in [true, false] {
      let mut writer = MessageWriter::default();
      let command = if direct {
        writer
          .set_world_pointer([1; 16], false, [2; 16], settings)
          .unwrap()
      } else {
        writer
          .command(&Command::new_v4(CommandBody::InputSetWorldPointer(
            WorldPointerPayload {
              object_id: ObjectId::from_uuid(Uuid::from_bytes([2; 16])).unwrap(),
              settings,
            },
          )))
          .unwrap()
      };
      let group = writer.parallel_group(&[command]).unwrap();
      let batch = writer
        .batch([3; 16], [4; 16], None, NativeBatchStart::Now, &[group])
        .unwrap();
      let bytes = writer.finish([4; 16], &[batch]).unwrap();
      assert_eq!(
        ResponseView::read(bytes.as_bytes())
          .unwrap()
          .message_count(),
        1
      );
      let response = wire::size_prefixed_root_as_response(bytes.as_bytes()).unwrap();
      let command = response
        .messages()
        .get(0)
        .message_as_batch()
        .unwrap()
        .groups()
        .get(0)
        .commands()
        .get(0)
        .command_as_core_command()
        .unwrap();
      let payload = command.payload_as_world_pointer_payload().unwrap();
      assert_eq!(
        payload.object_id().bytes().iter().collect::<Vec<_>>(),
        [2; 16]
      );
      assert_eq!(
        payload.settings().map(|s| WorldPointerSettings {
          interaction_layer: s.interaction_layer(),
          order: s.order(),
          capture_on_press: s.capture_on_press(),
          focusable: s.focusable(),
        }),
        settings
      );
      assert!(ResponseView::read(&bytes.as_bytes()[..bytes.as_bytes().len() - 1]).is_err());
    }
  }
}

#[test]
fn zero_pointer_target_is_rejected_before_transport() {
  let mut writer = MessageWriter::default();
  let command = writer
    .set_world_pointer(
      [1; 16],
      false,
      [0; 16],
      Some(WorldPointerSettings::default()),
    )
    .unwrap();
  let group = writer.parallel_group(&[command]).unwrap();
  let batch = writer
    .batch([3; 16], [4; 16], None, NativeBatchStart::Now, &[group])
    .unwrap();
  assert!(writer.finish([4; 16], &[batch]).is_err());
}
