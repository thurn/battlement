use battlement::*;
use uuid::Uuid as ExternalUuid;

use crate::{
  common_generated::Uuid, ui_event_generated::battlement::flat_buffers::generated as wire,
};

pub(crate) fn decode(
  value: wire::UiEventAction<'_>,
) -> Result<UiEventAction, crate::ProtocolError> {
  Ok(UiEventAction::new(
    ActionId::from_uuid(uuid(value.action_id())).map_err(|failure| error(failure.to_string()))?,
    SessionId::from_uuid(uuid(value.session_id())).map_err(|failure| error(failure.to_string()))?,
    decode_event(value.event())?,
  ))
}

pub(crate) fn decode_event(event: wire::UiEvent<'_>) -> Result<UiEvent, crate::ProtocolError> {
  Ok(UiEvent::new(
    object_id(event.target_id())?,
    event.cancelable(),
    event.default_prevented(),
    body(event)?,
  ))
}

/// Checks nested UI-event semantics without reconstructing the legacy action graph.
pub(crate) fn validate_body(event: wire::UiEvent<'_>) -> Result<(), crate::ProtocolError> {
  match event.kind() {
    0 => drop(accessibility(required(
      event.body_as_accessibility_action_event(),
    )?)?),
    1 | 3 => drop(pointer_button(required(
      event.body_as_pointer_button_event(),
    )?)?),
    2 => drop(pointer_move(required(event.body_as_pointer_move_event())?)?),
    4 => drop(pointer_cancel(required(
      event.body_as_pointer_cancel_event(),
    )?)?),
    5 => drop(click(required(event.body_as_click_event())?)?),
    6 | 7 => drop(boundary(required(event.body_as_pointer_boundary_event())?)?),
    8 | 9 => drop(crossing(required(event.body_as_pointer_crossing_event())?)?),
    10 => drop(wheel(required(event.body_as_wheel_event())?)?),
    11 | 12 => drop(capture(required(event.body_as_pointer_capture_event())?)),
    13 | 14 => validate_key(required(event.body_as_key_event())?)?,
    15 => drop(navigation(required(
      event.body_as_navigation_move_event(),
    )?)?),
    16 | 22 | 23 => {
      required(event.body_as_empty_event())?;
    }
    17..=20 => drop(focus(required(event.body_as_focus_event())?)?),
    21 => drop(geometry(required(event.body_as_geometry_event())?)?),
    24..=26 => validate_transition(required(event.body_as_transition_event())?)?,
    27 => validate_ui_value_table(
      required(event.body_as_value_changing_event())?.proposed_type(),
      required(event.body_as_value_changing_event())?.proposed(),
    )?,
    28 => {
      let value = required(event.body_as_value_commit_event())?;
      validate_ui_value_table(value.previous_type(), value.previous())?;
      validate_ui_value_table(value.proposed_type(), value.proposed())?;
    }
    29 => {
      required(event.body_as_text_input_event())?;
    }
    30 => {
      required(event.body_as_selection_event())?;
    }
    31..=34 => validate_link(required(event.body_as_link_event())?)?,
    35 | 36 => drop(scroll(required(event.body_as_scroll_event())?)?),
    37 => drop(tab_selection(required(
      event.body_as_tab_selection_event(),
    )?)?),
    38 => drop(tab_close(required(event.body_as_tab_close_event())?)?),
    39 => drop(tab_reorder(required(event.body_as_tab_reorder_event())?)?),
    _ => return Err(error("unknown UI event kind")),
  }
  Ok(())
}

fn validate_key(value: wire::KeyEvent<'_>) -> Result<(), crate::ProtocolError> {
  if !value.has_physical_key() && value.physical_key() != wire::PhysicalKey::Escape {
    return Err(error(
      "an absent physical key must use its default scalar slot",
    ));
  }
  if value.has_physical_key() {
    physical_key(value.physical_key())?;
  }
  validate_modifiers(value.modifiers())
}

fn validate_transition(value: wire::TransitionEvent<'_>) -> Result<(), crate::ProtocolError> {
  let properties = value.properties();
  if properties.is_empty() {
    return Err(error("transition properties must be nonempty"));
  }
  let mut unique = std::collections::HashSet::with_capacity(properties.len());
  for property in properties {
    transition_property(property)?;
    if !unique.insert(property.0) {
      return Err(error("transition properties must be unique"));
    }
  }
  finite32(value.elapsed_ms())?;
  Ok(())
}

fn validate_link(value: wire::LinkEvent<'_>) -> Result<(), crate::ProtocolError> {
  point(value.position())?;
  button(value.button())?;
  Ok(())
}

fn validate_ui_value_table(
  kind: wire::UiValue,
  value: flatbuffers::Table<'_>,
) -> Result<(), crate::ProtocolError> {
  // SAFETY: The bounded root verifier checked the union table using the matching tag.
  unsafe {
    match kind {
      wire::UiValue::BoolValue | wire::UiValue::I32Value | wire::UiValue::StringValue => {}
      wire::UiValue::IndexValue => {
        let value = wire::IndexValue::init_from_table(value).value();
        if !value.present() && value.value() != 0 {
          return Err(error("an absent UI index must use its default scalar slot"));
        }
      }
      wire::UiValue::IndicesValue => {
        let values = wire::IndicesValue::init_from_table(value).values();
        let mut previous = None;
        for item in values {
          if previous.is_some_and(|previous| previous >= item) {
            return Err(error("UI indices must be sorted and unique"));
          }
          previous = Some(item);
        }
      }
      wire::UiValue::ChoiceValue => {
        let value = wire::ChoiceValue::init_from_table(value).value();
        if !value.has_index() && value.index() != 0 {
          return Err(error(
            "an absent dropdown index must use its default scalar slot",
          ));
        }
        if value.has_index() != value.value().is_some() {
          return Err(error("dropdown index and value must be present together"));
        }
      }
      wire::UiValue::F32Value => {
        finite32(wire::F32Value::init_from_table(value).value())?;
      }
      wire::UiValue::F32RangeValue => {
        let value = wire::F32RangeValue::init_from_table(value).value();
        let min = finite32(value.min())?;
        let max = finite32(value.max())?;
        if min > max {
          return Err(error("UI range endpoints must be ordered"));
        }
      }
      _ => return Err(error("unknown UI value type")),
    }
  }
  Ok(())
}

fn body(event: wire::UiEvent<'_>) -> Result<UiEventBody, crate::ProtocolError> {
  Ok(match event.kind() {
    0 => UiEventBody::AccessibilityAction(accessibility(required(
      event.body_as_accessibility_action_event(),
    )?)?),
    1 => UiEventBody::PointerDown(pointer_button(required(
      event.body_as_pointer_button_event(),
    )?)?),
    2 => UiEventBody::PointerMove(pointer_move(required(event.body_as_pointer_move_event())?)?),
    3 => UiEventBody::PointerUp(pointer_button(required(
      event.body_as_pointer_button_event(),
    )?)?),
    4 => UiEventBody::PointerCancel(pointer_cancel(required(
      event.body_as_pointer_cancel_event(),
    )?)?),
    5 => UiEventBody::Click(click(required(event.body_as_click_event())?)?),
    6 => UiEventBody::PointerEnter(boundary(required(event.body_as_pointer_boundary_event())?)?),
    7 => UiEventBody::PointerLeave(boundary(required(event.body_as_pointer_boundary_event())?)?),
    8 => UiEventBody::PointerOver(crossing(required(event.body_as_pointer_crossing_event())?)?),
    9 => UiEventBody::PointerOut(crossing(required(event.body_as_pointer_crossing_event())?)?),
    10 => UiEventBody::Wheel(wheel(required(event.body_as_wheel_event())?)?),
    11 => UiEventBody::PointerCapture(capture(required(event.body_as_pointer_capture_event())?)),
    12 => UiEventBody::PointerCaptureOut(capture(required(event.body_as_pointer_capture_event())?)),
    13 => UiEventBody::KeyDown(key(required(event.body_as_key_event())?)?),
    14 => UiEventBody::KeyUp(key(required(event.body_as_key_event())?)?),
    15 => UiEventBody::NavigationMove(navigation(required(
      event.body_as_navigation_move_event(),
    )?)?),
    16 => UiEventBody::NavigationCancel(NavigationEvent {}),
    17 => UiEventBody::FocusIn(focus(required(event.body_as_focus_event())?)?),
    18 => UiEventBody::Focus(focus(required(event.body_as_focus_event())?)?),
    19 => UiEventBody::FocusOut(focus(required(event.body_as_focus_event())?)?),
    20 => UiEventBody::Blur(focus(required(event.body_as_focus_event())?)?),
    21 => UiEventBody::GeometryChanged(geometry(required(event.body_as_geometry_event())?)?),
    22 => UiEventBody::AttachToPanel(LifecycleEvent {}),
    23 => UiEventBody::DetachFromPanel(LifecycleEvent {}),
    24 => UiEventBody::TransitionStart(transition(required(event.body_as_transition_event())?)?),
    25 => UiEventBody::TransitionEnd(transition(required(event.body_as_transition_event())?)?),
    26 => UiEventBody::TransitionCancel(transition(required(event.body_as_transition_event())?)?),
    27 => UiEventBody::ValueChanging(value_changing(required(
      event.body_as_value_changing_event(),
    )?)?),
    28 => UiEventBody::ValueCommitted(value_committed(required(
      event.body_as_value_commit_event(),
    )?)?),
    29 => UiEventBody::Input(TextInputEvent {
      value: required(event.body_as_text_input_event())?
        .value()
        .to_owned(),
    }),
    30 => {
      let value = required(event.body_as_selection_event())?;
      UiEventBody::SelectionChanged(SelectionEvent {
        cursor_index: value.cursor_index(),
        selection_index: value.selection_index(),
      })
    }
    31 => UiEventBody::LinkEnter(link(required(event.body_as_link_event())?)?),
    32 => UiEventBody::LinkLeave(link(required(event.body_as_link_event())?)?),
    33 => UiEventBody::LinkDown(link(required(event.body_as_link_event())?)?),
    34 => UiEventBody::LinkUp(link(required(event.body_as_link_event())?)?),
    35 => UiEventBody::ScrollSettled(scroll(required(event.body_as_scroll_event())?)?),
    36 => UiEventBody::ScrollChanged(scroll(required(event.body_as_scroll_event())?)?),
    37 => UiEventBody::TabSelectionRequested(tab_selection(required(
      event.body_as_tab_selection_event(),
    )?)?),
    38 => UiEventBody::TabCloseRequested(tab_close(required(event.body_as_tab_close_event())?)?),
    39 => {
      UiEventBody::TabReorderRequested(tab_reorder(required(event.body_as_tab_reorder_event())?)?)
    }
    _ => return Err(error("unknown UI event kind")),
  })
}

fn accessibility(
  value: wire::AccessibilityActionEvent<'_>,
) -> Result<UiAccessibilityActionEvent, crate::ProtocolError> {
  let action = match value.action() {
    wire::AccessibilityActionKind::Activate => UiAccessibilityAction::Activate,
    wire::AccessibilityActionKind::Increment => UiAccessibilityAction::Increment,
    wire::AccessibilityActionKind::Decrement => UiAccessibilityAction::Decrement,
    wire::AccessibilityActionKind::Dismiss => UiAccessibilityAction::Dismiss,
    wire::AccessibilityActionKind::ScrollForward => UiAccessibilityAction::ScrollForward,
    wire::AccessibilityActionKind::ScrollBackward => UiAccessibilityAction::ScrollBackward,
    _ => return Err(error("unknown accessibility action")),
  };
  Ok(UiAccessibilityActionEvent {
    backend_generation: value.backend_generation(),
    action,
  })
}

fn pointer_button(
  value: wire::PointerButtonEvent<'_>,
) -> Result<PointerButtonEvent, crate::ProtocolError> {
  Ok(PointerButtonEvent {
    pointer_id: value.pointer_id(),
    position: point(value.position())?,
    delta: vector(value.delta())?,
    button: button(value.button())?.unwrap_or_default(),
    buttons: value.buttons(),
    pressure: finite32(value.pressure())?,
    click_count: value.click_count(),
    modifiers: modifiers(value.modifiers())?,
    pointer_type: pointer_type(value.pointer_type())?,
  })
}

fn pointer_move(
  value: wire::PointerMoveEvent<'_>,
) -> Result<PointerMoveEvent, crate::ProtocolError> {
  Ok(PointerMoveEvent {
    pointer_id: value.pointer_id(),
    position: point(value.position())?,
    delta: vector(value.delta())?,
    changed_button: button(value.changed_button())?,
    buttons: value.buttons(),
    pressure: finite32(value.pressure())?,
    click_count: value.click_count(),
    modifiers: modifiers(value.modifiers())?,
    pointer_type: pointer_type(value.pointer_type())?,
  })
}

fn pointer_cancel(
  value: wire::PointerCancelEvent<'_>,
) -> Result<PointerCancelEvent, crate::ProtocolError> {
  Ok(PointerCancelEvent {
    pointer_id: value.pointer_id(),
    position: point(value.position())?,
    delta: vector(value.delta())?,
    buttons: value.buttons(),
    pressure: finite32(value.pressure())?,
    modifiers: modifiers(value.modifiers())?,
    pointer_type: pointer_type(value.pointer_type())?,
  })
}

fn boundary(
  value: wire::PointerBoundaryEvent<'_>,
) -> Result<PointerBoundaryEvent, crate::ProtocolError> {
  Ok(PointerBoundaryEvent {
    pointer_id: value.pointer_id(),
    position: point(value.position())?,
    pointer_type: pointer_type(value.pointer_type())?,
  })
}

fn crossing(
  value: wire::PointerCrossingEvent<'_>,
) -> Result<PointerCrossingEvent, crate::ProtocolError> {
  Ok(PointerCrossingEvent {
    related_target_id: value.related_target_id().map(object_id).transpose()?,
    pointer_id: value.pointer_id(),
    position: point(value.position())?,
    pointer_type: pointer_type(value.pointer_type())?,
  })
}

fn wheel(value: wire::WheelEvent<'_>) -> Result<WheelEvent, crate::ProtocolError> {
  let delta = value.delta();
  Ok(WheelEvent {
    position: point(value.position())?,
    delta: UiVector3 {
      x: finite32(delta.x())?,
      y: finite32(delta.y())?,
      z: finite32(delta.z())?,
    },
    modifiers: modifiers(value.modifiers())?,
  })
}

fn capture(value: wire::PointerCaptureEvent<'_>) -> PointerCaptureEvent {
  PointerCaptureEvent {
    pointer_id: value.pointer_id(),
  }
}

fn key(value: wire::KeyEvent<'_>) -> Result<KeyEvent, crate::ProtocolError> {
  if !value.has_physical_key() && value.physical_key() != wire::PhysicalKey::Escape {
    return Err(error(
      "an absent physical key must use its default scalar slot",
    ));
  }
  Ok(KeyEvent {
    physical_key: value
      .has_physical_key()
      .then(|| physical_key(value.physical_key()))
      .transpose()?,
    text: value.text().to_owned(),
    modifiers: modifiers(value.modifiers())?,
  })
}

fn navigation(
  value: wire::NavigationMoveEvent<'_>,
) -> Result<NavigationMoveEvent, crate::ProtocolError> {
  let direction = navigation_direction(value.direction())?;
  Ok(NavigationMoveEvent {
    direction,
    move_vector: vector(value.move_())?,
  })
}

pub(crate) fn navigation_direction(
  value: wire::NavigationDirection,
) -> Result<NavigationDirection, crate::ProtocolError> {
  match value {
    wire::NavigationDirection::None => Ok(NavigationDirection::None),
    wire::NavigationDirection::Left => Ok(NavigationDirection::Left),
    wire::NavigationDirection::Up => Ok(NavigationDirection::Up),
    wire::NavigationDirection::Right => Ok(NavigationDirection::Right),
    wire::NavigationDirection::Down => Ok(NavigationDirection::Down),
    wire::NavigationDirection::Next => Ok(NavigationDirection::Next),
    wire::NavigationDirection::Previous => Ok(NavigationDirection::Previous),
    _ => Err(error("unknown navigation direction")),
  }
}

fn focus(value: wire::FocusEvent<'_>) -> Result<FocusEvent, crate::ProtocolError> {
  let direction = focus_direction(value.direction(), value.other_direction())?;
  Ok(FocusEvent {
    related_target_id: value.related_target_id().map(object_id).transpose()?,
    direction,
  })
}

pub(crate) fn focus_direction(
  kind: wire::FocusDirectionKind,
  other: i32,
) -> Result<FocusDirection, crate::ProtocolError> {
  match kind {
    wire::FocusDirectionKind::Absent | wire::FocusDirectionKind::None => {
      require_default_i32(other, "focus direction")?;
      Ok(FocusDirection::None)
    }
    wire::FocusDirectionKind::Unspecified => {
      require_default_i32(other, "focus direction")?;
      Ok(FocusDirection::Unspecified)
    }
    wire::FocusDirectionKind::Left => {
      require_default_i32(other, "focus direction")?;
      Ok(FocusDirection::Left)
    }
    wire::FocusDirectionKind::Right => {
      require_default_i32(other, "focus direction")?;
      Ok(FocusDirection::Right)
    }
    wire::FocusDirectionKind::Other => Ok(FocusDirection::Other(other)),
    _ => Err(error("unknown focus direction")),
  }
}

fn geometry(value: wire::GeometryEvent<'_>) -> Result<GeometryEvent, crate::ProtocolError> {
  Ok(GeometryEvent {
    previous: rect(value.previous())?,
    current: rect(value.current())?,
  })
}

fn transition(value: wire::TransitionEvent<'_>) -> Result<TransitionEvent, crate::ProtocolError> {
  let properties = value
    .properties()
    .iter()
    .map(transition_property)
    .collect::<Result<Vec<_>, _>>()?;
  if properties.is_empty() {
    return Err(error("transition properties must be nonempty"));
  }
  let unique = properties
    .iter()
    .copied()
    .collect::<std::collections::HashSet<_>>();
  if unique.len() != properties.len() {
    return Err(error("transition properties must be unique"));
  }
  Ok(TransitionEvent {
    properties,
    elapsed_ms: finite32(value.elapsed_ms())?,
  })
}

fn value_changing(
  value: wire::ValueChangingEvent<'_>,
) -> Result<ValueChangingEvent, crate::ProtocolError> {
  Ok(ValueChangingEvent {
    proposed: ui_value(value.proposed_type(), value.proposed())?,
  })
}
fn value_committed(
  value: wire::ValueCommitEvent<'_>,
) -> Result<ValueCommitEvent, crate::ProtocolError> {
  Ok(ValueCommitEvent {
    previous: ui_value(value.previous_type(), value.previous())?,
    proposed: ui_value(value.proposed_type(), value.proposed())?,
  })
}

fn ui_value(
  kind: wire::UiValue,
  value: flatbuffers::Table<'_>,
) -> Result<UiValue, crate::ProtocolError> {
  // SAFETY: The bounded root verifier checked the union table using the matching tag.
  unsafe {
    Ok(match kind {
      wire::UiValue::BoolValue => UiValue::Bool(wire::BoolValue::init_from_table(value).value()),
      wire::UiValue::IndexValue => {
        let value = wire::IndexValue::init_from_table(value).value();
        if !value.present() && value.value() != 0 {
          return Err(error("an absent UI index must use its default scalar slot"));
        }
        UiValue::Index(value.present().then(|| value.value()))
      }
      wire::UiValue::IndicesValue => {
        let values = wire::IndicesValue::init_from_table(value).values();
        let values = values.iter().collect::<Vec<_>>();
        if values.windows(2).any(|pair| pair[0] >= pair[1]) {
          return Err(error("UI indices must be sorted and unique"));
        }
        UiValue::Indices(values)
      }
      wire::UiValue::ChoiceValue => {
        let value = wire::ChoiceValue::init_from_table(value).value();
        let index = value.has_index().then(|| value.index());
        let text = value.value().map(str::to_owned);
        if !value.has_index() && value.index() != 0 {
          return Err(error(
            "an absent dropdown index must use its default scalar slot",
          ));
        }
        if index.is_some() != text.is_some() {
          return Err(error("dropdown index and value must be present together"));
        }
        UiValue::Choice(Choice { index, value: text })
      }
      wire::UiValue::F32Value => {
        UiValue::F32(finite32(wire::F32Value::init_from_table(value).value())?)
      }
      wire::UiValue::I32Value => UiValue::I32(wire::I32Value::init_from_table(value).value()),
      wire::UiValue::F32RangeValue => {
        let value = wire::F32RangeValue::init_from_table(value).value();
        let range = F32Range::new(finite32(value.min())?, finite32(value.max())?);
        if range.min > range.max {
          return Err(error("UI range endpoints must be ordered"));
        }
        UiValue::F32Range(range)
      }
      wire::UiValue::StringValue => {
        UiValue::String(wire::StringValue::init_from_table(value).value().to_owned())
      }
      _ => return Err(error("unknown UI value type")),
    })
  }
}

fn click(value: wire::ClickEvent<'_>) -> Result<ClickEvent, crate::ProtocolError> {
  Ok(match value.kind() {
    wire::ClickKind::Pointer => {
      let pointer = value
        .pointer()
        .ok_or_else(|| error("pointer click payload is absent"))?;
      ClickEvent::pointer(
        pointer.pointer_id(),
        point(pointer.position())?,
        button(pointer.button())?.unwrap_or_default(),
        pointer.click_count(),
        modifiers(pointer.modifiers())?,
      )
    }
    wire::ClickKind::NavigationSubmit => {
      if value.pointer().is_some() {
        return Err(error("navigation click carried pointer data"));
      }
      ClickEvent::NavigationSubmit
    }
    wire::ClickKind::Repeat => {
      if value.pointer().is_some() {
        return Err(error("repeat click carried pointer data"));
      }
      ClickEvent::Repeat
    }
    _ => return Err(error("unknown click kind")),
  })
}

pub(crate) fn button(
  value: Option<wire::PointerButton<'_>>,
) -> Result<Option<PointerButton>, crate::ProtocolError> {
  let Some(value) = value else {
    return Ok(None);
  };
  Ok(Some(match value.kind() {
    wire::PointerButtonKind::None => {
      require_default_i32(value.other(), "pointer button")?;
      return Ok(None);
    }
    wire::PointerButtonKind::Left => {
      require_default_i32(value.other(), "pointer button")?;
      PointerButton::Left
    }
    wire::PointerButtonKind::Middle => {
      require_default_i32(value.other(), "pointer button")?;
      PointerButton::Middle
    }
    wire::PointerButtonKind::Right => {
      require_default_i32(value.other(), "pointer button")?;
      PointerButton::Right
    }
    wire::PointerButtonKind::Other if value.other() > 2 => PointerButton::Other(value.other()),
    wire::PointerButtonKind::Other => return Err(error("other pointer button must exceed two")),
    _ => return Err(error("unknown pointer button kind")),
  }))
}

fn modifiers(mask: u32) -> Result<KeyModifiers, crate::ProtocolError> {
  validate_modifiers(mask)?;
  let variants = [
    KeyModifier::Alt,
    KeyModifier::Control,
    KeyModifier::Command,
    KeyModifier::Shift,
    KeyModifier::CapsLock,
    KeyModifier::Numeric,
    KeyModifier::FunctionKey,
  ];
  KeyModifiers::new(
    variants
      .into_iter()
      .enumerate()
      .filter_map(|(index, value)| (mask & (1 << index) != 0).then_some(value))
      .collect(),
  )
  .map_err(error)
}

fn validate_modifiers(mask: u32) -> Result<(), crate::ProtocolError> {
  (mask & !0x7f == 0)
    .then_some(())
    .ok_or_else(|| error("unknown key modifier bit"))
}

pub(crate) fn pointer_type(value: wire::PointerType) -> Result<PointerType, crate::ProtocolError> {
  match value {
    wire::PointerType::Mouse => Ok(PointerType::Mouse),
    wire::PointerType::Touch => Ok(PointerType::Touch),
    wire::PointerType::Pen => Ok(PointerType::Pen),
    wire::PointerType::Unknown => Ok(PointerType::Unknown),
    _ => Err(error("unknown pointer type")),
  }
}

pub(crate) fn physical_key(value: wire::PhysicalKey) -> Result<PhysicalKey, crate::ProtocolError> {
  Ok(match value {
    wire::PhysicalKey::Escape => PhysicalKey::Escape,
    wire::PhysicalKey::F1 => PhysicalKey::F1,
    wire::PhysicalKey::F2 => PhysicalKey::F2,
    wire::PhysicalKey::F3 => PhysicalKey::F3,
    wire::PhysicalKey::F4 => PhysicalKey::F4,
    wire::PhysicalKey::F5 => PhysicalKey::F5,
    wire::PhysicalKey::F6 => PhysicalKey::F6,
    wire::PhysicalKey::F7 => PhysicalKey::F7,
    wire::PhysicalKey::F8 => PhysicalKey::F8,
    wire::PhysicalKey::F9 => PhysicalKey::F9,
    wire::PhysicalKey::F10 => PhysicalKey::F10,
    wire::PhysicalKey::F11 => PhysicalKey::F11,
    wire::PhysicalKey::F12 => PhysicalKey::F12,
    wire::PhysicalKey::Backquote => PhysicalKey::Backquote,
    wire::PhysicalKey::Digit0 => PhysicalKey::Digit0,
    wire::PhysicalKey::Digit1 => PhysicalKey::Digit1,
    wire::PhysicalKey::Digit2 => PhysicalKey::Digit2,
    wire::PhysicalKey::Digit3 => PhysicalKey::Digit3,
    wire::PhysicalKey::Digit4 => PhysicalKey::Digit4,
    wire::PhysicalKey::Digit5 => PhysicalKey::Digit5,
    wire::PhysicalKey::Digit6 => PhysicalKey::Digit6,
    wire::PhysicalKey::Digit7 => PhysicalKey::Digit7,
    wire::PhysicalKey::Digit8 => PhysicalKey::Digit8,
    wire::PhysicalKey::Digit9 => PhysicalKey::Digit9,
    wire::PhysicalKey::Minus => PhysicalKey::Minus,
    wire::PhysicalKey::Equal => PhysicalKey::Equal,
    wire::PhysicalKey::Backspace => PhysicalKey::Backspace,
    wire::PhysicalKey::Tab => PhysicalKey::Tab,
    wire::PhysicalKey::KeyA => PhysicalKey::KeyA,
    wire::PhysicalKey::KeyB => PhysicalKey::KeyB,
    wire::PhysicalKey::KeyC => PhysicalKey::KeyC,
    wire::PhysicalKey::KeyD => PhysicalKey::KeyD,
    wire::PhysicalKey::KeyE => PhysicalKey::KeyE,
    wire::PhysicalKey::KeyF => PhysicalKey::KeyF,
    wire::PhysicalKey::KeyG => PhysicalKey::KeyG,
    wire::PhysicalKey::KeyH => PhysicalKey::KeyH,
    wire::PhysicalKey::KeyI => PhysicalKey::KeyI,
    wire::PhysicalKey::KeyJ => PhysicalKey::KeyJ,
    wire::PhysicalKey::KeyK => PhysicalKey::KeyK,
    wire::PhysicalKey::KeyL => PhysicalKey::KeyL,
    wire::PhysicalKey::KeyM => PhysicalKey::KeyM,
    wire::PhysicalKey::KeyN => PhysicalKey::KeyN,
    wire::PhysicalKey::KeyO => PhysicalKey::KeyO,
    wire::PhysicalKey::KeyP => PhysicalKey::KeyP,
    wire::PhysicalKey::KeyQ => PhysicalKey::KeyQ,
    wire::PhysicalKey::KeyR => PhysicalKey::KeyR,
    wire::PhysicalKey::KeyS => PhysicalKey::KeyS,
    wire::PhysicalKey::KeyT => PhysicalKey::KeyT,
    wire::PhysicalKey::KeyU => PhysicalKey::KeyU,
    wire::PhysicalKey::KeyV => PhysicalKey::KeyV,
    wire::PhysicalKey::KeyW => PhysicalKey::KeyW,
    wire::PhysicalKey::KeyX => PhysicalKey::KeyX,
    wire::PhysicalKey::KeyY => PhysicalKey::KeyY,
    wire::PhysicalKey::KeyZ => PhysicalKey::KeyZ,
    wire::PhysicalKey::BracketLeft => PhysicalKey::BracketLeft,
    wire::PhysicalKey::BracketRight => PhysicalKey::BracketRight,
    wire::PhysicalKey::Backslash => PhysicalKey::Backslash,
    wire::PhysicalKey::CapsLock => PhysicalKey::CapsLock,
    wire::PhysicalKey::Semicolon => PhysicalKey::Semicolon,
    wire::PhysicalKey::Quote => PhysicalKey::Quote,
    wire::PhysicalKey::Enter => PhysicalKey::Enter,
    wire::PhysicalKey::ShiftLeft => PhysicalKey::ShiftLeft,
    wire::PhysicalKey::ShiftRight => PhysicalKey::ShiftRight,
    wire::PhysicalKey::ControlLeft => PhysicalKey::ControlLeft,
    wire::PhysicalKey::ControlRight => PhysicalKey::ControlRight,
    wire::PhysicalKey::AltLeft => PhysicalKey::AltLeft,
    wire::PhysicalKey::AltRight => PhysicalKey::AltRight,
    wire::PhysicalKey::MetaLeft => PhysicalKey::MetaLeft,
    wire::PhysicalKey::MetaRight => PhysicalKey::MetaRight,
    wire::PhysicalKey::Comma => PhysicalKey::Comma,
    wire::PhysicalKey::Period => PhysicalKey::Period,
    wire::PhysicalKey::Slash => PhysicalKey::Slash,
    wire::PhysicalKey::Space => PhysicalKey::Space,
    wire::PhysicalKey::ContextMenu => PhysicalKey::ContextMenu,
    wire::PhysicalKey::Insert => PhysicalKey::Insert,
    wire::PhysicalKey::Delete => PhysicalKey::Delete,
    wire::PhysicalKey::Home => PhysicalKey::Home,
    wire::PhysicalKey::End => PhysicalKey::End,
    wire::PhysicalKey::PageUp => PhysicalKey::PageUp,
    wire::PhysicalKey::PageDown => PhysicalKey::PageDown,
    wire::PhysicalKey::ArrowLeft => PhysicalKey::ArrowLeft,
    wire::PhysicalKey::ArrowRight => PhysicalKey::ArrowRight,
    wire::PhysicalKey::ArrowUp => PhysicalKey::ArrowUp,
    wire::PhysicalKey::ArrowDown => PhysicalKey::ArrowDown,
    wire::PhysicalKey::PrintScreen => PhysicalKey::PrintScreen,
    wire::PhysicalKey::ScrollLock => PhysicalKey::ScrollLock,
    wire::PhysicalKey::Pause => PhysicalKey::Pause,
    wire::PhysicalKey::NumLock => PhysicalKey::NumLock,
    wire::PhysicalKey::Numpad0 => PhysicalKey::Numpad0,
    wire::PhysicalKey::Numpad1 => PhysicalKey::Numpad1,
    wire::PhysicalKey::Numpad2 => PhysicalKey::Numpad2,
    wire::PhysicalKey::Numpad3 => PhysicalKey::Numpad3,
    wire::PhysicalKey::Numpad4 => PhysicalKey::Numpad4,
    wire::PhysicalKey::Numpad5 => PhysicalKey::Numpad5,
    wire::PhysicalKey::Numpad6 => PhysicalKey::Numpad6,
    wire::PhysicalKey::Numpad7 => PhysicalKey::Numpad7,
    wire::PhysicalKey::Numpad8 => PhysicalKey::Numpad8,
    wire::PhysicalKey::Numpad9 => PhysicalKey::Numpad9,
    wire::PhysicalKey::NumpadDecimal => PhysicalKey::NumpadDecimal,
    wire::PhysicalKey::NumpadAdd => PhysicalKey::NumpadAdd,
    wire::PhysicalKey::NumpadSubtract => PhysicalKey::NumpadSubtract,
    wire::PhysicalKey::NumpadMultiply => PhysicalKey::NumpadMultiply,
    wire::PhysicalKey::NumpadDivide => PhysicalKey::NumpadDivide,
    wire::PhysicalKey::NumpadEnter => PhysicalKey::NumpadEnter,
    _ => return Err(error("unknown physical key")),
  })
}

pub(crate) fn transition_property(
  value: wire::TransitionProperty,
) -> Result<TransitionProperty, crate::ProtocolError> {
  Ok(match value {
    wire::TransitionProperty::All => TransitionProperty::All,
    wire::TransitionProperty::AlignContent => TransitionProperty::AlignContent,
    wire::TransitionProperty::AlignItems => TransitionProperty::AlignItems,
    wire::TransitionProperty::AlignSelf => TransitionProperty::AlignSelf,
    wire::TransitionProperty::AspectRatio => TransitionProperty::AspectRatio,
    wire::TransitionProperty::BackgroundColor => TransitionProperty::BackgroundColor,
    wire::TransitionProperty::BackgroundImage => TransitionProperty::BackgroundImage,
    wire::TransitionProperty::BackgroundPositionX => TransitionProperty::BackgroundPositionX,
    wire::TransitionProperty::BackgroundPositionY => TransitionProperty::BackgroundPositionY,
    wire::TransitionProperty::BackgroundRepeat => TransitionProperty::BackgroundRepeat,
    wire::TransitionProperty::BackgroundSize => TransitionProperty::BackgroundSize,
    wire::TransitionProperty::BorderBottomColor => TransitionProperty::BorderBottomColor,
    wire::TransitionProperty::BorderBottomLeftRadius => TransitionProperty::BorderBottomLeftRadius,
    wire::TransitionProperty::BorderBottomRightRadius => {
      TransitionProperty::BorderBottomRightRadius
    }
    wire::TransitionProperty::BorderBottomWidth => TransitionProperty::BorderBottomWidth,
    wire::TransitionProperty::BorderLeftColor => TransitionProperty::BorderLeftColor,
    wire::TransitionProperty::BorderLeftWidth => TransitionProperty::BorderLeftWidth,
    wire::TransitionProperty::BorderRightColor => TransitionProperty::BorderRightColor,
    wire::TransitionProperty::BorderRightWidth => TransitionProperty::BorderRightWidth,
    wire::TransitionProperty::BorderTopColor => TransitionProperty::BorderTopColor,
    wire::TransitionProperty::BorderTopLeftRadius => TransitionProperty::BorderTopLeftRadius,
    wire::TransitionProperty::BorderTopRightRadius => TransitionProperty::BorderTopRightRadius,
    wire::TransitionProperty::BorderTopWidth => TransitionProperty::BorderTopWidth,
    wire::TransitionProperty::Bottom => TransitionProperty::Bottom,
    wire::TransitionProperty::Color => TransitionProperty::Color,
    wire::TransitionProperty::Cursor => TransitionProperty::Cursor,
    wire::TransitionProperty::Display => TransitionProperty::Display,
    wire::TransitionProperty::FlexBasis => TransitionProperty::FlexBasis,
    wire::TransitionProperty::FlexDirection => TransitionProperty::FlexDirection,
    wire::TransitionProperty::FlexGrow => TransitionProperty::FlexGrow,
    wire::TransitionProperty::FlexShrink => TransitionProperty::FlexShrink,
    wire::TransitionProperty::FlexWrap => TransitionProperty::FlexWrap,
    wire::TransitionProperty::FontSize => TransitionProperty::FontSize,
    wire::TransitionProperty::Height => TransitionProperty::Height,
    wire::TransitionProperty::JustifyContent => TransitionProperty::JustifyContent,
    wire::TransitionProperty::Left => TransitionProperty::Left,
    wire::TransitionProperty::LetterSpacing => TransitionProperty::LetterSpacing,
    wire::TransitionProperty::MarginBottom => TransitionProperty::MarginBottom,
    wire::TransitionProperty::MarginLeft => TransitionProperty::MarginLeft,
    wire::TransitionProperty::MarginRight => TransitionProperty::MarginRight,
    wire::TransitionProperty::MarginTop => TransitionProperty::MarginTop,
    wire::TransitionProperty::MaxHeight => TransitionProperty::MaxHeight,
    wire::TransitionProperty::MaxWidth => TransitionProperty::MaxWidth,
    wire::TransitionProperty::MinHeight => TransitionProperty::MinHeight,
    wire::TransitionProperty::MinWidth => TransitionProperty::MinWidth,
    wire::TransitionProperty::Opacity => TransitionProperty::Opacity,
    wire::TransitionProperty::Overflow => TransitionProperty::Overflow,
    wire::TransitionProperty::PaddingBottom => TransitionProperty::PaddingBottom,
    wire::TransitionProperty::PaddingLeft => TransitionProperty::PaddingLeft,
    wire::TransitionProperty::PaddingRight => TransitionProperty::PaddingRight,
    wire::TransitionProperty::PaddingTop => TransitionProperty::PaddingTop,
    wire::TransitionProperty::Position => TransitionProperty::Position,
    wire::TransitionProperty::Right => TransitionProperty::Right,
    wire::TransitionProperty::Rotate => TransitionProperty::Rotate,
    wire::TransitionProperty::Scale => TransitionProperty::Scale,
    wire::TransitionProperty::TextOverflow => TransitionProperty::TextOverflow,
    wire::TransitionProperty::TextShadow => TransitionProperty::TextShadow,
    wire::TransitionProperty::Top => TransitionProperty::Top,
    wire::TransitionProperty::TransformOrigin => TransitionProperty::TransformOrigin,
    wire::TransitionProperty::TransitionDelay => TransitionProperty::TransitionDelay,
    wire::TransitionProperty::TransitionDuration => TransitionProperty::TransitionDuration,
    wire::TransitionProperty::TransitionProperty => TransitionProperty::TransitionProperty,
    wire::TransitionProperty::TransitionTimingFunction => {
      TransitionProperty::TransitionTimingFunction
    }
    wire::TransitionProperty::Translate => TransitionProperty::Translate,
    wire::TransitionProperty::UnityBackgroundImageTintColor => {
      TransitionProperty::UnityBackgroundImageTintColor
    }
    wire::TransitionProperty::UnityEditorTextRenderingMode => {
      TransitionProperty::UnityEditorTextRenderingMode
    }
    wire::TransitionProperty::UnityFontDefinition => TransitionProperty::UnityFontDefinition,
    wire::TransitionProperty::UnityFontStyleAndWeight => {
      TransitionProperty::UnityFontStyleAndWeight
    }
    wire::TransitionProperty::UnityMaterial => TransitionProperty::UnityMaterial,
    wire::TransitionProperty::UnityOverflowClipBox => TransitionProperty::UnityOverflowClipBox,
    wire::TransitionProperty::UnityParagraphSpacing => TransitionProperty::UnityParagraphSpacing,
    wire::TransitionProperty::UnitySliceBottom => TransitionProperty::UnitySliceBottom,
    wire::TransitionProperty::UnitySliceLeft => TransitionProperty::UnitySliceLeft,
    wire::TransitionProperty::UnitySliceRight => TransitionProperty::UnitySliceRight,
    wire::TransitionProperty::UnitySliceScale => TransitionProperty::UnitySliceScale,
    wire::TransitionProperty::UnitySliceTop => TransitionProperty::UnitySliceTop,
    wire::TransitionProperty::UnitySliceType => TransitionProperty::UnitySliceType,
    wire::TransitionProperty::UnityTextAlign => TransitionProperty::UnityTextAlign,
    wire::TransitionProperty::UnityTextAutoSize => TransitionProperty::UnityTextAutoSize,
    wire::TransitionProperty::UnityTextGenerator => TransitionProperty::UnityTextGenerator,
    wire::TransitionProperty::UnityTextOutlineColor => TransitionProperty::UnityTextOutlineColor,
    wire::TransitionProperty::UnityTextOutlineWidth => TransitionProperty::UnityTextOutlineWidth,
    wire::TransitionProperty::UnityTextOverflowPosition => {
      TransitionProperty::UnityTextOverflowPosition
    }
    wire::TransitionProperty::Visibility => TransitionProperty::Visibility,
    wire::TransitionProperty::WhiteSpace => TransitionProperty::WhiteSpace,
    wire::TransitionProperty::Width => TransitionProperty::Width,
    wire::TransitionProperty::WordSpacing => TransitionProperty::WordSpacing,
    _ => return Err(error("unknown transition property")),
  })
}

fn link(value: wire::LinkEvent<'_>) -> Result<LinkEvent, crate::ProtocolError> {
  Ok(LinkEvent {
    link_id: value.link_id().to_owned(),
    link_text: value.link_text().to_owned(),
    pointer_id: value.pointer_id(),
    position: point(value.position())?,
    button: button(value.button())?,
  })
}
fn scroll(value: wire::ScrollEvent<'_>) -> Result<ScrollEvent, crate::ProtocolError> {
  Ok(ScrollEvent {
    offset: vector(value.offset())?,
  })
}
fn tab_selection(
  value: wire::TabSelectionEvent<'_>,
) -> Result<TabSelectionEvent, crate::ProtocolError> {
  Ok(TabSelectionEvent {
    previous_index: value.previous_index(),
    proposed_index: value.proposed_index(),
    proposed_tab_id: object_id(value.proposed_tab_id())?,
  })
}
fn tab_close(value: wire::TabCloseEvent<'_>) -> Result<TabCloseEvent, crate::ProtocolError> {
  Ok(TabCloseEvent {
    tab_id: object_id(value.tab_id())?,
    index: value.index(),
  })
}
fn tab_reorder(value: wire::TabReorderEvent<'_>) -> Result<TabReorderEvent, crate::ProtocolError> {
  Ok(TabReorderEvent {
    tab_id: object_id(value.tab_id())?,
    previous_index: value.previous_index(),
    proposed_index: value.proposed_index(),
  })
}

fn point(value: &wire::PanelPoint) -> Result<PanelPoint, crate::ProtocolError> {
  Ok(PanelPoint::new(finite64(value.x())?, finite64(value.y())?))
}
fn vector(value: &wire::Vector2) -> Result<Vector, crate::ProtocolError> {
  Ok(Vector::new(finite32(value.x())?, finite32(value.y())?))
}
fn rect(value: &wire::Rect) -> Result<Rect, crate::ProtocolError> {
  Ok(Rect::new(
    finite64(value.x())?,
    finite64(value.y())?,
    finite64(value.width())?,
    finite64(value.height())?,
  ))
}
fn object_id(value: &Uuid) -> Result<ObjectId, crate::ProtocolError> {
  ObjectId::from_uuid(uuid(value)).map_err(|failure| error(failure.to_string()))
}
fn uuid(value: &Uuid) -> ExternalUuid {
  ExternalUuid::from_bytes(crate::ui_event::uuid_bytes(value))
}
fn required<T>(value: Option<T>) -> Result<T, crate::ProtocolError> {
  value.ok_or_else(|| error("required union payload is absent"))
}
fn finite32(value: f32) -> Result<f32, crate::ProtocolError> {
  value
    .is_finite()
    .then_some(value)
    .ok_or_else(|| error("UI event number must be finite"))
}
fn finite64(value: f64) -> Result<f64, crate::ProtocolError> {
  value
    .is_finite()
    .then_some(value)
    .ok_or_else(|| error("UI event number must be finite"))
}
fn require_default_i32(value: i32, field: &str) -> Result<(), crate::ProtocolError> {
  (value == 0)
    .then_some(())
    .ok_or_else(|| error(format!("{field} carried data outside its selected variant")))
}
fn error(message: impl Into<String>) -> crate::ProtocolError {
  crate::ProtocolError::new(message)
}
