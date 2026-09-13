use std::collections::HashSet;

use battlement::*;
use flatbuffers::{FlatBufferBuilder, UnionWIPOffset, WIPOffset};

use crate::{
  FinishedMessage, ProtocolError, common_generated::Uuid,
  ui_event_generated::battlement::flat_buffers::generated as wire,
};

struct BodyOffset {
  kind: u8,
  body_type: wire::UiEventBody,
  body: WIPOffset<UnionWIPOffset>,
}

pub(crate) fn write(value: &UiEventAction) -> Result<FinishedMessage, ProtocolError> {
  if !value.is_valid() {
    return Err(error("a default-prevented UI event must be cancelable"));
  }
  let mut builder = FlatBufferBuilder::with_capacity(2048);
  let body = write_body(&mut builder, &value.event.body)?;
  let target_id = uuid(value.event.target_id);
  let event = wire::UiEvent::create(
    &mut builder,
    &wire::UiEventArgs {
      target_id: Some(&target_id),
      cancelable: value.event.cancelable,
      default_prevented: value.event.default_prevented,
      kind: body.kind,
      body_type: body.body_type,
      body: Some(body.body),
    },
  );
  let action_id = Uuid(*value.action_id.as_uuid().as_bytes());
  let session_id = Uuid(*value.session_id.as_uuid().as_bytes());
  let root = wire::UiEventAction::create(
    &mut builder,
    &wire::UiEventActionArgs {
      action_id: Some(&action_id),
      session_id: Some(&session_id),
      event: Some(event),
    },
  );
  wire::finish_size_prefixed_ui_event_action_buffer(&mut builder, root);
  let (storage, start) = builder.collapse();
  let message = FinishedMessage::from_storage(storage, start);
  crate::UiEventActionView::read(message.as_bytes())?;
  Ok(message)
}

fn write_body<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  body: &UiEventBody,
) -> Result<BodyOffset, ProtocolError> {
  macro_rules! body {
    ($kind:expr, $type:ident, $value:expr) => {{
      let value = $value;
      BodyOffset {
        kind: $kind as u8,
        body_type: wire::UiEventBody::$type,
        body: value.as_union_value(),
      }
    }};
  }
  Ok(match body {
    UiEventBody::AccessibilityAction(value) => {
      let action = match value.action {
        UiAccessibilityAction::Activate => wire::AccessibilityActionKind::Activate,
        UiAccessibilityAction::Increment => wire::AccessibilityActionKind::Increment,
        UiAccessibilityAction::Decrement => wire::AccessibilityActionKind::Decrement,
        UiAccessibilityAction::Dismiss => wire::AccessibilityActionKind::Dismiss,
        UiAccessibilityAction::ScrollForward => wire::AccessibilityActionKind::ScrollForward,
        UiAccessibilityAction::ScrollBackward => wire::AccessibilityActionKind::ScrollBackward,
      };
      body!(
        UiEventKind::AccessibilityAction,
        AccessibilityActionEvent,
        wire::AccessibilityActionEvent::create(
          builder,
          &wire::AccessibilityActionEventArgs {
            backend_generation: value.backend_generation,
            action,
          },
        )
      )
    }
    UiEventBody::PointerDown(value) => body!(
      UiEventKind::PointerDown,
      PointerButtonEvent,
      pointer_button_event(builder, value)?
    ),
    UiEventBody::PointerMove(value) => body!(
      UiEventKind::PointerMove,
      PointerMoveEvent,
      pointer_move_event(builder, value)?
    ),
    UiEventBody::PointerUp(value) => body!(
      UiEventKind::PointerUp,
      PointerButtonEvent,
      pointer_button_event(builder, value)?
    ),
    UiEventBody::PointerCancel(value) => body!(
      UiEventKind::PointerCancel,
      PointerCancelEvent,
      pointer_cancel_event(builder, value)?
    ),
    UiEventBody::Click(value) => body!(UiEventKind::Click, ClickEvent, click(builder, value)?),
    UiEventBody::PointerEnter(value) => body!(
      UiEventKind::PointerEnter,
      PointerBoundaryEvent,
      pointer_boundary(builder, value)?
    ),
    UiEventBody::PointerLeave(value) => body!(
      UiEventKind::PointerLeave,
      PointerBoundaryEvent,
      pointer_boundary(builder, value)?
    ),
    UiEventBody::PointerOver(value) => body!(
      UiEventKind::PointerOver,
      PointerCrossingEvent,
      pointer_crossing(builder, value)?
    ),
    UiEventBody::PointerOut(value) => body!(
      UiEventKind::PointerOut,
      PointerCrossingEvent,
      pointer_crossing(builder, value)?
    ),
    UiEventBody::Wheel(value) => body!(UiEventKind::Wheel, WheelEvent, wheel(builder, value)?),
    UiEventBody::PointerCapture(value) => body!(
      UiEventKind::PointerCapture,
      PointerCaptureEvent,
      wire::PointerCaptureEvent::create(
        builder,
        &wire::PointerCaptureEventArgs {
          pointer_id: value.pointer_id
        },
      )
    ),
    UiEventBody::PointerCaptureOut(value) => body!(
      UiEventKind::PointerCaptureOut,
      PointerCaptureEvent,
      wire::PointerCaptureEvent::create(
        builder,
        &wire::PointerCaptureEventArgs {
          pointer_id: value.pointer_id
        },
      )
    ),
    UiEventBody::KeyDown(value) => body!(UiEventKind::KeyDown, KeyEvent, key(builder, value)),
    UiEventBody::KeyUp(value) => body!(UiEventKind::KeyUp, KeyEvent, key(builder, value)),
    UiEventBody::NavigationMove(value) => body!(
      UiEventKind::NavigationMove,
      NavigationMoveEvent,
      navigation(builder, *value)?
    ),
    UiEventBody::NavigationCancel(_) => body!(
      UiEventKind::NavigationCancel,
      EmptyEvent,
      wire::EmptyEvent::create(builder, &wire::EmptyEventArgs::default())
    ),
    UiEventBody::FocusIn(value) => body!(UiEventKind::FocusIn, FocusEvent, focus(builder, *value)),
    UiEventBody::Focus(value) => body!(UiEventKind::Focus, FocusEvent, focus(builder, *value)),
    UiEventBody::FocusOut(value) => {
      body!(UiEventKind::FocusOut, FocusEvent, focus(builder, *value))
    }
    UiEventBody::Blur(value) => body!(UiEventKind::Blur, FocusEvent, focus(builder, *value)),
    UiEventBody::GeometryChanged(value) => body!(
      UiEventKind::GeometryChanged,
      GeometryEvent,
      geometry(builder, *value)?
    ),
    UiEventBody::AttachToPanel(_) => body!(
      UiEventKind::AttachToPanel,
      EmptyEvent,
      wire::EmptyEvent::create(builder, &wire::EmptyEventArgs::default())
    ),
    UiEventBody::DetachFromPanel(_) => body!(
      UiEventKind::DetachFromPanel,
      EmptyEvent,
      wire::EmptyEvent::create(builder, &wire::EmptyEventArgs::default())
    ),
    UiEventBody::TransitionStart(value) => body!(
      UiEventKind::TransitionStart,
      TransitionEvent,
      transition(builder, value)?
    ),
    UiEventBody::TransitionEnd(value) => body!(
      UiEventKind::TransitionEnd,
      TransitionEvent,
      transition(builder, value)?
    ),
    UiEventBody::TransitionCancel(value) => body!(
      UiEventKind::TransitionCancel,
      TransitionEvent,
      transition(builder, value)?
    ),
    UiEventBody::ValueChanging(value) => {
      let proposed = ui_value(builder, &value.proposed)?;
      body!(
        UiEventKind::ValueChanging,
        ValueChangingEvent,
        wire::ValueChangingEvent::create(
          builder,
          &wire::ValueChangingEventArgs {
            proposed_type: proposed.0,
            proposed: Some(proposed.1),
          },
        )
      )
    }
    UiEventBody::ValueCommitted(value) => {
      let previous = ui_value(builder, &value.previous)?;
      let proposed = ui_value(builder, &value.proposed)?;
      body!(
        UiEventKind::ValueCommitted,
        ValueCommitEvent,
        wire::ValueCommitEvent::create(
          builder,
          &wire::ValueCommitEventArgs {
            previous_type: previous.0,
            previous: Some(previous.1),
            proposed_type: proposed.0,
            proposed: Some(proposed.1),
          },
        )
      )
    }
    UiEventBody::Input(value) => {
      let value = builder.create_string(&value.value);
      body!(
        UiEventKind::Input,
        TextInputEvent,
        wire::TextInputEvent::create(builder, &wire::TextInputEventArgs { value: Some(value) })
      )
    }
    UiEventBody::SelectionChanged(value) => body!(
      UiEventKind::SelectionChanged,
      SelectionEvent,
      wire::SelectionEvent::create(
        builder,
        &wire::SelectionEventArgs {
          cursor_index: value.cursor_index,
          selection_index: value.selection_index,
        },
      )
    ),
    UiEventBody::LinkEnter(value) => {
      body!(UiEventKind::LinkEnter, LinkEvent, link(builder, value)?)
    }
    UiEventBody::LinkLeave(value) => {
      body!(UiEventKind::LinkLeave, LinkEvent, link(builder, value)?)
    }
    UiEventBody::LinkDown(value) => body!(UiEventKind::LinkDown, LinkEvent, link(builder, value)?),
    UiEventBody::LinkUp(value) => body!(UiEventKind::LinkUp, LinkEvent, link(builder, value)?),
    UiEventBody::ScrollSettled(value) => body!(
      UiEventKind::ScrollSettled,
      ScrollEvent,
      scroll(builder, *value)?
    ),
    UiEventBody::ScrollChanged(value) => body!(
      UiEventKind::ScrollChanged,
      ScrollEvent,
      scroll(builder, *value)?
    ),
    UiEventBody::TabSelectionRequested(value) => {
      let id = uuid(value.proposed_tab_id);
      body!(
        UiEventKind::TabSelectionRequested,
        TabSelectionEvent,
        wire::TabSelectionEvent::create(
          builder,
          &wire::TabSelectionEventArgs {
            previous_index: value.previous_index,
            proposed_index: value.proposed_index,
            proposed_tab_id: Some(&id),
          },
        )
      )
    }
    UiEventBody::TabCloseRequested(value) => {
      let id = uuid(value.tab_id);
      body!(
        UiEventKind::TabCloseRequested,
        TabCloseEvent,
        wire::TabCloseEvent::create(
          builder,
          &wire::TabCloseEventArgs {
            tab_id: Some(&id),
            index: value.index,
          },
        )
      )
    }
    UiEventBody::TabReorderRequested(value) => {
      let id = uuid(value.tab_id);
      body!(
        UiEventKind::TabReorderRequested,
        TabReorderEvent,
        wire::TabReorderEvent::create(
          builder,
          &wire::TabReorderEventArgs {
            tab_id: Some(&id),
            previous_index: value.previous_index,
            proposed_index: value.proposed_index,
          },
        )
      )
    }
  })
}

fn pointer_button_event<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &PointerButtonEvent,
) -> Result<WIPOffset<wire::PointerButtonEvent<'a>>, ProtocolError> {
  let position = point(value.position)?;
  let delta = vector(value.delta)?;
  let button = pointer_button(builder, Some(value.button))?;
  Ok(wire::PointerButtonEvent::create(
    builder,
    &wire::PointerButtonEventArgs {
      position: Some(&position),
      delta: Some(&delta),
      pointer_type: pointer_type(value.pointer_type),
      modifiers: modifiers(&value.modifiers),
      click_count: value.click_count,
      pressure: finite32(value.pressure)?,
      buttons: value.buttons,
      button: Some(button),
      pointer_id: value.pointer_id,
    },
  ))
}

fn pointer_move_event<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &PointerMoveEvent,
) -> Result<WIPOffset<wire::PointerMoveEvent<'a>>, ProtocolError> {
  let position = point(value.position)?;
  let delta = vector(value.delta)?;
  let button = value
    .changed_button
    .map(|button| pointer_button(builder, Some(button)))
    .transpose()?;
  Ok(wire::PointerMoveEvent::create(
    builder,
    &wire::PointerMoveEventArgs {
      position: Some(&position),
      delta: Some(&delta),
      pointer_type: pointer_type(value.pointer_type),
      modifiers: modifiers(&value.modifiers),
      click_count: value.click_count,
      pressure: finite32(value.pressure)?,
      buttons: value.buttons,
      changed_button: button,
      pointer_id: value.pointer_id,
    },
  ))
}

fn pointer_cancel_event<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &PointerCancelEvent,
) -> Result<WIPOffset<wire::PointerCancelEvent<'a>>, ProtocolError> {
  let position = point(value.position)?;
  let delta = vector(value.delta)?;
  Ok(wire::PointerCancelEvent::create(
    builder,
    &wire::PointerCancelEventArgs {
      position: Some(&position),
      delta: Some(&delta),
      pointer_type: pointer_type(value.pointer_type),
      modifiers: modifiers(&value.modifiers),
      pressure: finite32(value.pressure)?,
      buttons: value.buttons,
      pointer_id: value.pointer_id,
    },
  ))
}

fn pointer_boundary<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &PointerBoundaryEvent,
) -> Result<WIPOffset<wire::PointerBoundaryEvent<'a>>, ProtocolError> {
  let position = point(value.position)?;
  Ok(wire::PointerBoundaryEvent::create(
    builder,
    &wire::PointerBoundaryEventArgs {
      position: Some(&position),
      pointer_type: pointer_type(value.pointer_type),
      pointer_id: value.pointer_id,
    },
  ))
}

fn pointer_crossing<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &PointerCrossingEvent,
) -> Result<WIPOffset<wire::PointerCrossingEvent<'a>>, ProtocolError> {
  let position = point(value.position)?;
  let related_target_id = value.related_target_id.map(uuid);
  Ok(wire::PointerCrossingEvent::create(
    builder,
    &wire::PointerCrossingEventArgs {
      related_target_id: related_target_id.as_ref(),
      position: Some(&position),
      pointer_type: pointer_type(value.pointer_type),
      pointer_id: value.pointer_id,
    },
  ))
}

fn wheel<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &WheelEvent,
) -> Result<WIPOffset<wire::WheelEvent<'a>>, ProtocolError> {
  let position = point(value.position)?;
  let delta = wire::Vector3::new(
    finite32(value.delta.x)?,
    finite32(value.delta.y)?,
    finite32(value.delta.z)?,
  );
  Ok(wire::WheelEvent::create(
    builder,
    &wire::WheelEventArgs {
      position: Some(&position),
      delta: Some(&delta),
      modifiers: modifiers(&value.modifiers),
    },
  ))
}

fn key<'a>(builder: &mut FlatBufferBuilder<'a>, value: &KeyEvent) -> WIPOffset<wire::KeyEvent<'a>> {
  let text = builder.create_string(&value.text);
  wire::KeyEvent::create(
    builder,
    &wire::KeyEventArgs {
      has_physical_key: value.physical_key.is_some(),
      physical_key: wire::PhysicalKey(value.physical_key.unwrap_or(PhysicalKey::Escape) as u16),
      text: Some(text),
      modifiers: modifiers(&value.modifiers),
    },
  )
}

fn navigation<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: NavigationMoveEvent,
) -> Result<WIPOffset<wire::NavigationMoveEvent<'a>>, ProtocolError> {
  let move_vector = vector(value.move_vector)?;
  Ok(wire::NavigationMoveEvent::create(
    builder,
    &wire::NavigationMoveEventArgs {
      move_: Some(&move_vector),
      direction: wire::NavigationDirection(value.direction as u8),
    },
  ))
}

fn focus<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: FocusEvent,
) -> WIPOffset<wire::FocusEvent<'a>> {
  let related_target_id = value.related_target_id.map(uuid);
  let (direction, other_direction) = match value.direction {
    FocusDirection::None => (wire::FocusDirectionKind::None, 0),
    FocusDirection::Unspecified => (wire::FocusDirectionKind::Unspecified, 0),
    FocusDirection::Left => (wire::FocusDirectionKind::Left, 0),
    FocusDirection::Right => (wire::FocusDirectionKind::Right, 0),
    FocusDirection::Other(value) => (wire::FocusDirectionKind::Other, value),
  };
  wire::FocusEvent::create(
    builder,
    &wire::FocusEventArgs {
      related_target_id: related_target_id.as_ref(),
      direction,
      other_direction,
    },
  )
}

fn geometry<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: GeometryEvent,
) -> Result<WIPOffset<wire::GeometryEvent<'a>>, ProtocolError> {
  let previous = rect(value.previous)?;
  let current = rect(value.current)?;
  Ok(wire::GeometryEvent::create(
    builder,
    &wire::GeometryEventArgs {
      previous: Some(&previous),
      current: Some(&current),
    },
  ))
}

fn transition<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &TransitionEvent,
) -> Result<WIPOffset<wire::TransitionEvent<'a>>, ProtocolError> {
  if value.properties.is_empty() {
    return Err(error("transition properties must be nonempty"));
  }
  let mut unique = HashSet::with_capacity(value.properties.len());
  let properties = value
    .properties
    .iter()
    .map(|property| {
      let ordinal = *property as u16;
      if !unique.insert(ordinal) {
        return Err(error("transition properties must be unique"));
      }
      Ok(wire::TransitionProperty(ordinal))
    })
    .collect::<Result<Vec<_>, ProtocolError>>()?;
  let properties = builder.create_vector(&properties);
  Ok(wire::TransitionEvent::create(
    builder,
    &wire::TransitionEventArgs {
      properties: Some(properties),
      elapsed_ms: finite32(value.elapsed_ms)?,
    },
  ))
}

fn ui_value<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &UiValue,
) -> Result<(wire::UiValue, WIPOffset<UnionWIPOffset>), ProtocolError> {
  Ok(match value {
    UiValue::Bool(value) => {
      let value = wire::BoolValue::create(builder, &wire::BoolValueArgs { value: *value });
      (wire::UiValue::BoolValue, value.as_union_value())
    }
    UiValue::Index(value) => {
      let optional = wire::OptionalIndex::create(
        builder,
        &wire::OptionalIndexArgs {
          present: value.is_some(),
          value: value.unwrap_or(0),
        },
      );
      let value = wire::IndexValue::create(
        builder,
        &wire::IndexValueArgs {
          value: Some(optional),
        },
      );
      (wire::UiValue::IndexValue, value.as_union_value())
    }
    UiValue::Indices(values) => {
      if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(error("UI indices must be sorted and unique"));
      }
      let values = builder.create_vector(values);
      let value = wire::IndicesValue::create(
        builder,
        &wire::IndicesValueArgs {
          values: Some(values),
        },
      );
      (wire::UiValue::IndicesValue, value.as_union_value())
    }
    UiValue::Choice(choice) => {
      if choice.index.is_some() != choice.value.is_some() {
        return Err(error("dropdown index and value must be present together"));
      }
      let text = choice
        .value
        .as_ref()
        .map(|value| builder.create_string(value));
      let choice = wire::DropdownChoice::create(
        builder,
        &wire::DropdownChoiceArgs {
          has_index: choice.index.is_some(),
          index: choice.index.unwrap_or(0),
          value: text,
        },
      );
      let value = wire::ChoiceValue::create(
        builder,
        &wire::ChoiceValueArgs {
          value: Some(choice),
        },
      );
      (wire::UiValue::ChoiceValue, value.as_union_value())
    }
    UiValue::F32(item) => {
      let value = wire::F32Value::create(
        builder,
        &wire::F32ValueArgs {
          value: finite32(*item)?,
        },
      );
      (wire::UiValue::F32Value, value.as_union_value())
    }
    UiValue::I32(item) => {
      let value = wire::I32Value::create(builder, &wire::I32ValueArgs { value: *item });
      (wire::UiValue::I32Value, value.as_union_value())
    }
    UiValue::F32Range(range) => {
      if range.min > range.max {
        return Err(error("UI range endpoints must be ordered"));
      }
      let range = wire::FloatRange::new(finite32(range.min)?, finite32(range.max)?);
      let value = wire::F32RangeValue::create(
        builder,
        &wire::F32RangeValueArgs {
          value: Some(&range),
        },
      );
      (wire::UiValue::F32RangeValue, value.as_union_value())
    }
    UiValue::String(item) => {
      let text = builder.create_string(item);
      let value = wire::StringValue::create(builder, &wire::StringValueArgs { value: Some(text) });
      (wire::UiValue::StringValue, value.as_union_value())
    }
  })
}

fn click<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &ClickEvent,
) -> Result<WIPOffset<wire::ClickEvent<'a>>, ProtocolError> {
  let (kind, pointer) = match value {
    ClickEvent::Pointer {
      pointer_id,
      position,
      button,
      click_count,
      modifiers: keys,
    } => {
      let position = point(*position)?;
      let button = pointer_button(builder, Some(*button))?;
      let pointer = wire::PointerClickEvent::create(
        builder,
        &wire::PointerClickEventArgs {
          position: Some(&position),
          modifiers: modifiers(keys),
          button: Some(button),
          pointer_id: *pointer_id,
          click_count: *click_count,
        },
      );
      (wire::ClickKind::Pointer, Some(pointer))
    }
    ClickEvent::NavigationSubmit => (wire::ClickKind::NavigationSubmit, None),
    ClickEvent::Repeat => (wire::ClickKind::Repeat, None),
  };
  Ok(wire::ClickEvent::create(
    builder,
    &wire::ClickEventArgs { kind, pointer },
  ))
}

fn link<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: &LinkEvent,
) -> Result<WIPOffset<wire::LinkEvent<'a>>, ProtocolError> {
  let link_id = builder.create_string(&value.link_id);
  let link_text = builder.create_string(&value.link_text);
  let position = point(value.position)?;
  let button = value
    .button
    .map(|button| pointer_button(builder, Some(button)))
    .transpose()?;
  Ok(wire::LinkEvent::create(
    builder,
    &wire::LinkEventArgs {
      link_id: Some(link_id),
      link_text: Some(link_text),
      pointer_id: value.pointer_id,
      position: Some(&position),
      button,
    },
  ))
}

fn scroll<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: ScrollEvent,
) -> Result<WIPOffset<wire::ScrollEvent<'a>>, ProtocolError> {
  let offset = vector(value.offset)?;
  Ok(wire::ScrollEvent::create(
    builder,
    &wire::ScrollEventArgs {
      offset: Some(&offset),
    },
  ))
}

fn pointer_button<'a>(
  builder: &mut FlatBufferBuilder<'a>,
  value: Option<PointerButton>,
) -> Result<WIPOffset<wire::PointerButton<'a>>, ProtocolError> {
  let (kind, other) = match value {
    None => (wire::PointerButtonKind::None, 0),
    Some(PointerButton::Left) => (wire::PointerButtonKind::Left, 0),
    Some(PointerButton::Middle) => (wire::PointerButtonKind::Middle, 0),
    Some(PointerButton::Right) => (wire::PointerButtonKind::Right, 0),
    Some(PointerButton::Other(value)) if value > 2 => (wire::PointerButtonKind::Other, value),
    Some(PointerButton::Other(_)) => return Err(error("other pointer button must exceed two")),
  };
  Ok(wire::PointerButton::create(
    builder,
    &wire::PointerButtonArgs { kind, other },
  ))
}

fn modifiers(value: &KeyModifiers) -> u32 {
  value
    .as_slice()
    .iter()
    .fold(0, |mask, item| mask | (1 << (*item as u32)))
}
fn pointer_type(value: PointerType) -> wire::PointerType {
  wire::PointerType(value as u8)
}
fn uuid(value: ObjectId) -> Uuid {
  Uuid(*value.as_uuid().as_bytes())
}
fn point(value: PanelPoint) -> Result<wire::PanelPoint, ProtocolError> {
  Ok(wire::PanelPoint::new(
    finite64(value.x)?,
    finite64(value.y)?,
  ))
}
fn vector(value: Vector) -> Result<wire::Vector2, ProtocolError> {
  Ok(wire::Vector2::new(finite32(value.x)?, finite32(value.y)?))
}
fn rect(value: Rect) -> Result<wire::Rect, ProtocolError> {
  Ok(wire::Rect::new(
    finite64(value.x)?,
    finite64(value.y)?,
    finite64(value.width)?,
    finite64(value.height)?,
  ))
}
fn finite32(value: f32) -> Result<f32, ProtocolError> {
  value
    .is_finite()
    .then_some(value)
    .ok_or_else(|| error("UI event numbers must be finite"))
}
fn finite64(value: f64) -> Result<f64, ProtocolError> {
  value
    .is_finite()
    .then_some(value)
    .ok_or_else(|| error("UI event numbers must be finite"))
}
fn error(message: impl Into<String>) -> ProtocolError {
  ProtocolError::new(message)
}
