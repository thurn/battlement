//! Direct fixture writers for downstream transport tests.

use crate::{
  client_message_generated::battlement::flat_buffers::generated as client_wire,
  common_generated::{Uuid, Vector3d},
  ui_event_generated::PanelPoint,
  ui_event_generated::battlement::flat_buffers::generated as wire,
};

/// Writes a verified pointer-enter core action without an owned protocol graph.
#[must_use]
pub fn pointer_enter_core_action(
  action_id: [u8; 16],
  session_id: [u8; 16],
  object_id: [u8; 16],
) -> Vec<u8> {
  pointer_core_action(
    action_id,
    session_id,
    object_id,
    client_wire::CoreActionKind::PointerEnter,
  )
}

/// Writes a verified pointer-click core action without an owned protocol graph.
#[must_use]
pub fn pointer_click_core_action(
  action_id: [u8; 16],
  session_id: [u8; 16],
  object_id: [u8; 16],
) -> Vec<u8> {
  pointer_core_action(
    action_id,
    session_id,
    object_id,
    client_wire::CoreActionKind::PointerClick,
  )
}

fn pointer_core_action(
  action_id: [u8; 16],
  session_id: [u8; 16],
  object_id: [u8; 16],
  kind: client_wire::CoreActionKind,
) -> Vec<u8> {
  let mut builder = flatbuffers::FlatBufferBuilder::new();
  let screen = PanelPoint::new(1.0, 2.0);
  let world = Vector3d::new(0.0, 0.0, 0.0);
  let (body_type, body) = if kind == client_wire::CoreActionKind::PointerClick {
    let button = crate::ui_event_generated::PointerButton::create(
      &mut builder,
      &crate::ui_event_generated::PointerButtonArgs {
        kind: crate::ui_event_generated::PointerButtonKind::Left,
        other: 0,
      },
    );
    let body = client_wire::PointerButtonAction::create(
      &mut builder,
      &client_wire::PointerButtonActionArgs {
        object_id: Some(&Uuid(object_id)),
        pointer_id: 0,
        screen_position: Some(&screen),
        world_hit: Some(&world),
        button: Some(button),
      },
    );
    (
      client_wire::CoreActionBody::PointerButtonAction,
      body.as_union_value(),
    )
  } else {
    let body = client_wire::PointerAction::create(
      &mut builder,
      &client_wire::PointerActionArgs {
        object_id: Some(&Uuid(object_id)),
        pointer_id: 0,
        screen_position: Some(&screen),
        world_hit: Some(&world),
      },
    );
    (
      client_wire::CoreActionBody::PointerAction,
      body.as_union_value(),
    )
  };
  let action = client_wire::CoreAction::create(
    &mut builder,
    &client_wire::CoreActionArgs {
      action_id: Some(&Uuid(action_id)),
      session_id: Some(&Uuid(session_id)),
      kind,
      body_type,
      body: Some(body),
    },
  );
  let root = client_wire::CoreClientMessage::create(
    &mut builder,
    &client_wire::CoreClientMessageArgs {
      body_type: client_wire::CoreClientMessageBody::CoreAction,
      body: Some(action.as_union_value()),
    },
  );
  client_wire::finish_size_prefixed_core_client_message_buffer(&mut builder, root);
  builder.finished_data().to_vec()
}

/// Writes a verified navigation-submit UI event without constructing an owned protocol graph.
#[must_use]
pub fn navigation_submit_ui_event(
  action_id: [u8; 16],
  session_id: [u8; 16],
  target_id: [u8; 16],
  cancelable: bool,
  default_prevented: bool,
) -> Vec<u8> {
  let mut builder = flatbuffers::FlatBufferBuilder::new();
  let body = wire::ClickEvent::create(
    &mut builder,
    &wire::ClickEventArgs {
      kind: wire::ClickKind::NavigationSubmit,
      pointer: None,
    },
  );
  let event = wire::UiEvent::create(
    &mut builder,
    &wire::UiEventArgs {
      target_id: Some(&Uuid(target_id)),
      cancelable,
      default_prevented,
      kind: 5,
      body_type: wire::UiEventBody::ClickEvent,
      body: Some(body.as_union_value()),
    },
  );
  let root = wire::UiEventAction::create(
    &mut builder,
    &wire::UiEventActionArgs {
      action_id: Some(&Uuid(action_id)),
      session_id: Some(&Uuid(session_id)),
      event: Some(event),
    },
  );
  wire::finish_size_prefixed_ui_event_action_buffer(&mut builder, root);
  builder.finished_data().to_vec()
}

/// Writes a verified text-input UI event directly from one borrowed string.
#[must_use]
pub fn text_input_ui_event(
  action_id: [u8; 16],
  session_id: [u8; 16],
  target_id: [u8; 16],
  text: &str,
) -> Vec<u8> {
  let mut builder = flatbuffers::FlatBufferBuilder::new();
  let text = builder.create_string(text);
  let body = wire::TextInputEvent::create(
    &mut builder,
    &wire::TextInputEventArgs { value: Some(text) },
  );
  let event = wire::UiEvent::create(
    &mut builder,
    &wire::UiEventArgs {
      target_id: Some(&Uuid(target_id)),
      cancelable: false,
      default_prevented: false,
      kind: battlement::UiEventKind::Input as u8,
      body_type: wire::UiEventBody::TextInputEvent,
      body: Some(body.as_union_value()),
    },
  );
  let root = wire::UiEventAction::create(
    &mut builder,
    &wire::UiEventActionArgs {
      action_id: Some(&Uuid(action_id)),
      session_id: Some(&Uuid(session_id)),
      event: Some(event),
    },
  );
  wire::finish_size_prefixed_ui_event_action_buffer(&mut builder, root);
  builder.finished_data().to_vec()
}
