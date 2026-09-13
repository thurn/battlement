use crate::{
  MAXIMUM_MESSAGE_BYTES, common_generated::Uuid,
  ui_event_generated::battlement::flat_buffers::generated as wire, verifier_options,
};

/// A structurally and semantically verified borrowed UI event action.
#[derive(Clone, Copy)]
pub struct UiEventActionView<'a> {
  value: wire::UiEventAction<'a>,
}

/// A borrowed value carried by a UI change or commit event.
#[derive(Clone, Copy)]
pub enum UiValueView<'a> {
  /// Boolean control value.
  Bool(bool),
  /// Optional selection index.
  Index(Option<u32>),
  /// Sorted, unique selection indices.
  Indices(UiIndicesView<'a>),
  /// Optional dropdown selection and its borrowed display value.
  Choice(UiChoiceView<'a>),
  /// Floating-point control value.
  F32(f32),
  /// Signed integer control value.
  I32(i32),
  /// Ordered floating-point range.
  F32Range {
    /// Inclusive lower endpoint.
    min: f32,
    /// Inclusive upper endpoint.
    max: f32,
  },
  /// Borrowed UTF-8 text tied to the request buffer.
  Text(&'a str),
}

/// A borrowed sorted, unique list of UI selection indices.
#[derive(Clone, Copy)]
pub struct UiIndicesView<'a> {
  values: flatbuffers::Vector<'a, u32>,
}

impl UiIndicesView<'_> {
  /// Returns the number of selected indices.
  #[must_use]
  pub fn len(self) -> usize {
    self.values.len()
  }

  /// Returns whether no indices are selected.
  #[must_use]
  pub fn is_empty(self) -> bool {
    self.values.is_empty()
  }

  /// Iterates the selected indices without allocating.
  pub fn iter(self) -> impl ExactSizeIterator<Item = u32> {
    self.values.iter()
  }
}

/// A borrowed dropdown selection.
#[derive(Clone, Copy)]
pub struct UiChoiceView<'a> {
  index: Option<u32>,
  value: Option<&'a str>,
}

impl<'a> UiChoiceView<'a> {
  /// Returns the selected index, or `None` for no selection.
  #[must_use]
  pub fn index(self) -> Option<u32> {
    self.index
  }

  /// Returns the selected display value, or `None` for no selection.
  #[must_use]
  pub fn value(self) -> Option<&'a str> {
    self.value
  }
}

/// Borrowed previous and proposed values from one committed UI edit.
#[derive(Clone, Copy)]
pub struct UiValueCommitView<'a> {
  value: wire::ValueCommitEvent<'a>,
}

impl<'a> UiValueCommitView<'a> {
  /// Returns the previous control value without materializing it.
  #[must_use]
  pub fn previous(self) -> UiValueView<'a> {
    previous_value(self.value)
  }

  /// Returns the proposed control value without materializing it.
  #[must_use]
  pub fn proposed(self) -> UiValueView<'a> {
    committed_value(self.value)
  }
}

/// Borrowed cursor and selection indices from a text selection event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiSelectionView {
  /// Current cursor index.
  pub cursor_index: u32,
  /// Current selection index.
  pub selection_index: u32,
}

/// Borrowed transition timing and property ordinals.
#[derive(Clone, Copy)]
pub struct UiTransitionView<'a> {
  value: wire::TransitionEvent<'a>,
}

impl UiTransitionView<'_> {
  /// Returns elapsed transition time in milliseconds.
  #[must_use]
  pub fn elapsed_ms(self) -> f32 {
    self.value.elapsed_ms()
  }

  /// Returns the number of transitioned properties.
  #[must_use]
  pub fn property_count(self) -> usize {
    self.value.properties().len()
  }

  /// Iterates typed transition properties without allocating.
  pub fn properties(self) -> impl ExactSizeIterator<Item = battlement::TransitionProperty> {
    self.value.properties().iter().map(|value| {
      crate::ui_event_owned::transition_property(value)
        .expect("transition property was semantically validated")
    })
  }
}

/// Borrowed link callback data.
#[derive(Clone, Copy)]
pub struct UiLinkView<'a> {
  value: wire::LinkEvent<'a>,
}

impl<'a> UiLinkView<'a> {
  /// Returns the link identifier.
  #[must_use]
  pub fn link_id(self) -> &'a str {
    self.value.link_id()
  }

  /// Returns the visible link text.
  #[must_use]
  pub fn link_text(self) -> &'a str {
    self.value.link_text()
  }

  /// Returns the pointer identifier.
  #[must_use]
  pub fn pointer_id(self) -> i32 {
    self.value.pointer_id()
  }

  /// Returns the pointer position in panel coordinates.
  #[must_use]
  pub fn position(self) -> (f64, f64) {
    let value = self.value.position();
    (value.x(), value.y())
  }
}

/// Borrowed focus callback data.
#[derive(Clone, Copy)]
pub struct UiFocusView<'a> {
  value: wire::FocusEvent<'a>,
}

impl UiFocusView<'_> {
  /// Returns the related target UUID, when present.
  #[must_use]
  pub fn related_target_id(self) -> Option<[u8; 16]> {
    self.value.related_target_id().map(uuid_bytes)
  }

  /// Returns the typed focus direction.
  #[must_use]
  pub fn direction(self) -> battlement::FocusDirection {
    crate::ui_event_owned::focus_direction(self.value.direction(), self.value.other_direction())
      .expect("focus direction was semantically validated")
  }
}

/// Borrowed keyboard callback data.
#[derive(Clone, Copy)]
pub struct UiKeyView<'a> {
  value: wire::KeyEvent<'a>,
}

impl<'a> UiKeyView<'a> {
  /// Returns the typed physical key, when present.
  #[must_use]
  pub fn physical_key(self) -> Option<battlement::PhysicalKey> {
    self.value.has_physical_key().then(|| {
      crate::ui_event_owned::physical_key(self.value.physical_key())
        .expect("physical key was semantically validated")
    })
  }

  /// Returns the borrowed text produced by the key event.
  #[must_use]
  pub fn text(self) -> &'a str {
    self.value.text()
  }

  /// Returns the validated modifier bit mask.
  #[must_use]
  pub fn modifiers(self) -> u32 {
    self.value.modifiers()
  }
}

/// Borrowed navigation callback data.
#[derive(Clone, Copy)]
pub struct UiNavigationView<'a> {
  value: wire::NavigationMoveEvent<'a>,
}

impl UiNavigationView<'_> {
  /// Returns the typed navigation direction.
  #[must_use]
  pub fn direction(self) -> battlement::NavigationDirection {
    crate::ui_event_owned::navigation_direction(self.value.direction())
      .expect("navigation direction was semantically validated")
  }

  /// Returns the navigation vector.
  #[must_use]
  pub fn movement(self) -> (f32, f32) {
    let value = self.value.move_();
    (value.x(), value.y())
  }
}

/// Borrowed tab-selection request data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiTabSelectionView {
  /// Previously selected tab index.
  pub previous_index: u32,
  /// Proposed tab index.
  pub proposed_index: u32,
  /// Proposed tab UUID.
  pub proposed_tab_id: [u8; 16],
}

/// Borrowed tab-close request data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiTabCloseView {
  /// Tab UUID to close.
  pub tab_id: [u8; 16],
  /// Current tab index.
  pub index: u32,
}

/// Borrowed tab-reorder request data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiTabReorderView {
  /// Tab UUID to move.
  pub tab_id: [u8; 16],
  /// Previous tab index.
  pub previous_index: u32,
  /// Proposed tab index.
  pub proposed_index: u32,
}

/// Borrowed pointer-button callback data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiPointerButtonView {
  /// Pointer identifier.
  pub pointer_id: i32,
  /// Position in panel coordinates.
  pub position: (f64, f64),
  /// Movement since the preceding event.
  pub delta: (f32, f32),
  /// Button associated with this event.
  pub button: battlement::PointerButton,
  /// Native pressed-button bit mask.
  pub buttons: u32,
  /// Normalized pointer pressure.
  pub pressure: f32,
  /// Consecutive click count.
  pub click_count: u32,
  /// Validated key-modifier bit mask.
  pub modifiers: u32,
  /// Pointer device category.
  pub pointer_type: battlement::PointerType,
}

/// Borrowed pointer-move callback data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiPointerMoveView {
  /// Pointer identifier.
  pub pointer_id: i32,
  /// Position in panel coordinates.
  pub position: (f64, f64),
  /// Movement since the preceding event.
  pub delta: (f32, f32),
  /// Button whose state changed, when any.
  pub changed_button: Option<battlement::PointerButton>,
  /// Native pressed-button bit mask.
  pub buttons: u32,
  /// Normalized pointer pressure.
  pub pressure: f32,
  /// Consecutive click count.
  pub click_count: u32,
  /// Validated key-modifier bit mask.
  pub modifiers: u32,
  /// Pointer device category.
  pub pointer_type: battlement::PointerType,
}

/// Borrowed wheel callback data.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiWheelView {
  /// Position in panel coordinates.
  pub position: (f64, f64),
  /// Three-axis wheel displacement.
  pub delta: (f32, f32, f32),
  /// Validated key-modifier bit mask.
  pub modifiers: u32,
}

/// Borrowed geometry-change rectangles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiGeometryView {
  /// Previous rectangle `(x, y, width, height)`.
  pub previous: (f64, f64, f64, f64),
  /// Current rectangle `(x, y, width, height)`.
  pub current: (f64, f64, f64, f64),
}

impl<'a> UiEventActionView<'a> {
  /// Reads one complete size-prefixed UI event action.
  pub fn read(bytes: &'a [u8]) -> Result<Self, crate::ProtocolError> {
    if bytes.len() > MAXIMUM_MESSAGE_BYTES {
      return Err(error("UI event action exceeds 16 MiB"));
    }
    let payload = bytes
      .get(4..)
      .filter(|_| size_prefix_matches(bytes))
      .ok_or_else(|| error("UI event action size prefix does not match its finished range"))?;
    if !wire::ui_event_action_buffer_has_identifier(payload) {
      return Err(error("UI event action has the wrong file identifier"));
    }
    let value =
      wire::size_prefixed_root_as_ui_event_action_with_opts(&verifier_options(), bytes)
        .map_err(|failure| error(format!("invalid UI event action FlatBuffer: {failure}")))?;
    validate(value)?;
    crate::ui_event_owned::validate_body(value.event())?;
    Ok(Self { value })
  }

  /// Returns the canonical action UUID bytes.
  #[must_use]
  pub fn action_id(self) -> [u8; 16] {
    uuid_bytes(self.value.action_id())
  }

  /// Returns the canonical session UUID bytes.
  #[must_use]
  pub fn session_id(self) -> [u8; 16] {
    uuid_bytes(self.value.session_id())
  }

  /// Returns the canonical target UUID bytes.
  #[must_use]
  pub fn target_id(self) -> [u8; 16] {
    uuid_bytes(self.value.event().target_id())
  }

  /// Returns the stable UI event kind ordinal.
  #[must_use]
  pub fn kind(self) -> u8 {
    self.value.event().kind()
  }

  /// Returns the typed UI event kind.
  #[must_use]
  pub fn event_kind(self) -> battlement::UiEventKind {
    const KINDS: [battlement::UiEventKind; 40] = [
      battlement::UiEventKind::AccessibilityAction,
      battlement::UiEventKind::PointerDown,
      battlement::UiEventKind::PointerMove,
      battlement::UiEventKind::PointerUp,
      battlement::UiEventKind::PointerCancel,
      battlement::UiEventKind::Click,
      battlement::UiEventKind::PointerEnter,
      battlement::UiEventKind::PointerLeave,
      battlement::UiEventKind::PointerOver,
      battlement::UiEventKind::PointerOut,
      battlement::UiEventKind::Wheel,
      battlement::UiEventKind::PointerCapture,
      battlement::UiEventKind::PointerCaptureOut,
      battlement::UiEventKind::KeyDown,
      battlement::UiEventKind::KeyUp,
      battlement::UiEventKind::NavigationMove,
      battlement::UiEventKind::NavigationCancel,
      battlement::UiEventKind::FocusIn,
      battlement::UiEventKind::Focus,
      battlement::UiEventKind::FocusOut,
      battlement::UiEventKind::Blur,
      battlement::UiEventKind::GeometryChanged,
      battlement::UiEventKind::AttachToPanel,
      battlement::UiEventKind::DetachFromPanel,
      battlement::UiEventKind::TransitionStart,
      battlement::UiEventKind::TransitionEnd,
      battlement::UiEventKind::TransitionCancel,
      battlement::UiEventKind::ValueChanging,
      battlement::UiEventKind::ValueCommitted,
      battlement::UiEventKind::Input,
      battlement::UiEventKind::SelectionChanged,
      battlement::UiEventKind::LinkEnter,
      battlement::UiEventKind::LinkLeave,
      battlement::UiEventKind::LinkDown,
      battlement::UiEventKind::LinkUp,
      battlement::UiEventKind::ScrollSettled,
      battlement::UiEventKind::ScrollChanged,
      battlement::UiEventKind::TabSelectionRequested,
      battlement::UiEventKind::TabCloseRequested,
      battlement::UiEventKind::TabReorderRequested,
    ];
    KINDS[usize::from(self.kind())]
  }

  /// Whether the event's default action can be prevented.
  #[must_use]
  pub fn cancelable(self) -> bool {
    self.value.event().cancelable()
  }

  /// Whether the incoming event has already prevented its native default.
  #[must_use]
  pub fn default_prevented(self) -> bool {
    self.value.event().default_prevented()
  }

  /// Returns the proposed value for a value-changing event.
  #[must_use]
  pub fn value_changing(self) -> Option<UiValueView<'a>> {
    self
      .value
      .event()
      .body_as_value_changing_event()
      .map(changing_value)
  }

  /// Returns the previous and proposed values for a value-commit event.
  #[must_use]
  pub fn value_commit(self) -> Option<UiValueCommitView<'a>> {
    self
      .value
      .event()
      .body_as_value_commit_event()
      .map(|value| UiValueCommitView { value })
  }

  /// Returns the borrowed text for an input event.
  #[must_use]
  pub fn input_text(self) -> Option<&'a str> {
    self
      .value
      .event()
      .body_as_text_input_event()
      .map(|value| value.value())
  }

  /// Returns the scalar indices for a text selection event.
  #[must_use]
  pub fn selection(self) -> Option<UiSelectionView> {
    self
      .value
      .event()
      .body_as_selection_event()
      .map(|value| UiSelectionView {
        cursor_index: value.cursor_index(),
        selection_index: value.selection_index(),
      })
  }

  /// Returns transition callback data for transition start, end, or cancel.
  #[must_use]
  pub fn transition(self) -> Option<UiTransitionView<'a>> {
    self
      .value
      .event()
      .body_as_transition_event()
      .map(|value| UiTransitionView { value })
  }

  /// Returns link callback data for link enter, leave, down, or up.
  #[must_use]
  pub fn link(self) -> Option<UiLinkView<'a>> {
    self
      .value
      .event()
      .body_as_link_event()
      .map(|value| UiLinkView { value })
  }

  /// Returns focus callback data for focus-in, focus, focus-out, or blur.
  #[must_use]
  pub fn focus(self) -> Option<UiFocusView<'a>> {
    self
      .value
      .event()
      .body_as_focus_event()
      .map(|value| UiFocusView { value })
  }

  /// Returns keyboard callback data for key-down or key-up.
  #[must_use]
  pub fn key(self) -> Option<UiKeyView<'a>> {
    self
      .value
      .event()
      .body_as_key_event()
      .map(|value| UiKeyView { value })
  }

  /// Returns directional navigation callback data.
  #[must_use]
  pub fn navigation(self) -> Option<UiNavigationView<'a>> {
    self
      .value
      .event()
      .body_as_navigation_move_event()
      .map(|value| UiNavigationView { value })
  }

  /// Returns the stable click-kind ordinal for a click event.
  #[must_use]
  pub fn click_kind(self) -> Option<u8> {
    self
      .value
      .event()
      .body_as_click_event()
      .map(|value| value.kind().0)
  }

  /// Returns the scroll offset for a settled or changed scroll event.
  #[must_use]
  pub fn scroll_offset(self) -> Option<(f32, f32)> {
    self.value.event().body_as_scroll_event().map(|value| {
      let offset = value.offset();
      (offset.x(), offset.y())
    })
  }

  /// Returns tab-selection request data.
  #[must_use]
  pub fn tab_selection(self) -> Option<UiTabSelectionView> {
    self
      .value
      .event()
      .body_as_tab_selection_event()
      .map(|value| UiTabSelectionView {
        previous_index: value.previous_index(),
        proposed_index: value.proposed_index(),
        proposed_tab_id: uuid_bytes(value.proposed_tab_id()),
      })
  }

  /// Returns tab-close request data.
  #[must_use]
  pub fn tab_close(self) -> Option<UiTabCloseView> {
    self
      .value
      .event()
      .body_as_tab_close_event()
      .map(|value| UiTabCloseView {
        tab_id: uuid_bytes(value.tab_id()),
        index: value.index(),
      })
  }

  /// Returns tab-reorder request data.
  #[must_use]
  pub fn tab_reorder(self) -> Option<UiTabReorderView> {
    self
      .value
      .event()
      .body_as_tab_reorder_event()
      .map(|value| UiTabReorderView {
        tab_id: uuid_bytes(value.tab_id()),
        previous_index: value.previous_index(),
        proposed_index: value.proposed_index(),
      })
  }

  /// Returns pointer-button data for pointer-down or pointer-up.
  #[must_use]
  pub fn pointer_button(self) -> Option<UiPointerButtonView> {
    self
      .value
      .event()
      .body_as_pointer_button_event()
      .map(|value| {
        let position = value.position();
        let delta = value.delta();
        UiPointerButtonView {
          pointer_id: value.pointer_id(),
          position: (position.x(), position.y()),
          delta: (delta.x(), delta.y()),
          button: crate::ui_event_owned::button(value.button())
            .expect("pointer button was semantically validated")
            .unwrap_or_default(),
          buttons: value.buttons(),
          pressure: value.pressure(),
          click_count: value.click_count(),
          modifiers: value.modifiers(),
          pointer_type: crate::ui_event_owned::pointer_type(value.pointer_type())
            .expect("pointer type was semantically validated"),
        }
      })
  }

  /// Returns pointer-move data.
  #[must_use]
  pub fn pointer_move(self) -> Option<UiPointerMoveView> {
    self
      .value
      .event()
      .body_as_pointer_move_event()
      .map(|value| {
        let position = value.position();
        let delta = value.delta();
        UiPointerMoveView {
          pointer_id: value.pointer_id(),
          position: (position.x(), position.y()),
          delta: (delta.x(), delta.y()),
          changed_button: crate::ui_event_owned::button(value.changed_button())
            .expect("changed pointer button was semantically validated"),
          buttons: value.buttons(),
          pressure: value.pressure(),
          click_count: value.click_count(),
          modifiers: value.modifiers(),
          pointer_type: crate::ui_event_owned::pointer_type(value.pointer_type())
            .expect("pointer type was semantically validated"),
        }
      })
  }

  /// Returns wheel callback data.
  #[must_use]
  pub fn wheel(self) -> Option<UiWheelView> {
    self.value.event().body_as_wheel_event().map(|value| {
      let position = value.position();
      let delta = value.delta();
      UiWheelView {
        position: (position.x(), position.y()),
        delta: (delta.x(), delta.y(), delta.z()),
        modifiers: value.modifiers(),
      }
    })
  }

  /// Returns the pointer identifier for pointer-capture lifecycle events.
  #[must_use]
  pub fn pointer_capture_id(self) -> Option<i32> {
    self
      .value
      .event()
      .body_as_pointer_capture_event()
      .map(|value| value.pointer_id())
  }

  /// Returns previous and current rectangles for a geometry-change event.
  #[must_use]
  pub fn geometry(self) -> Option<UiGeometryView> {
    self.value.event().body_as_geometry_event().map(|value| {
      let previous = value.previous();
      let current = value.current();
      UiGeometryView {
        previous: (
          previous.x(),
          previous.y(),
          previous.width(),
          previous.height(),
        ),
        current: (current.x(), current.y(), current.width(), current.height()),
      }
    })
  }

  /// Materializes the legacy public model for in-process engines during migration.
  pub fn to_owned(self) -> Result<battlement::UiEventAction, crate::ProtocolError> {
    crate::ui_event_owned::decode(self.value)
  }

  /// Copies only the callback payload required by the legacy synchronous dispatcher.
  ///
  /// Action and session identities remain borrowed scalars and are not reconstructed.
  pub fn to_owned_event(self) -> Result<battlement::UiEvent, crate::ProtocolError> {
    crate::ui_event_owned::decode_event(self.value.event())
  }
}

fn changing_value(value: wire::ValueChangingEvent<'_>) -> UiValueView<'_> {
  ui_value(
    value.proposed_type(),
    value.proposed_as_bool_value().map(|value| value.value()),
    value.proposed_as_index_value().map(|value| value.value()),
    value
      .proposed_as_indices_value()
      .map(|value| value.values()),
    value.proposed_as_choice_value().map(|value| value.value()),
    value.proposed_as_f32_value().map(|value| value.value()),
    value.proposed_as_i32_value().map(|value| value.value()),
    value
      .proposed_as_f32_range_value()
      .map(|value| value.value()),
    value.proposed_as_string_value().map(|value| value.value()),
  )
}

fn previous_value(value: wire::ValueCommitEvent<'_>) -> UiValueView<'_> {
  ui_value(
    value.previous_type(),
    value.previous_as_bool_value().map(|value| value.value()),
    value.previous_as_index_value().map(|value| value.value()),
    value
      .previous_as_indices_value()
      .map(|value| value.values()),
    value.previous_as_choice_value().map(|value| value.value()),
    value.previous_as_f32_value().map(|value| value.value()),
    value.previous_as_i32_value().map(|value| value.value()),
    value
      .previous_as_f32_range_value()
      .map(|value| value.value()),
    value.previous_as_string_value().map(|value| value.value()),
  )
}

fn committed_value(value: wire::ValueCommitEvent<'_>) -> UiValueView<'_> {
  ui_value(
    value.proposed_type(),
    value.proposed_as_bool_value().map(|value| value.value()),
    value.proposed_as_index_value().map(|value| value.value()),
    value
      .proposed_as_indices_value()
      .map(|value| value.values()),
    value.proposed_as_choice_value().map(|value| value.value()),
    value.proposed_as_f32_value().map(|value| value.value()),
    value.proposed_as_i32_value().map(|value| value.value()),
    value
      .proposed_as_f32_range_value()
      .map(|value| value.value()),
    value.proposed_as_string_value().map(|value| value.value()),
  )
}

#[allow(clippy::too_many_arguments)]
fn ui_value<'a>(
  kind: wire::UiValue,
  boolean: Option<bool>,
  index: Option<wire::OptionalIndex<'a>>,
  indices: Option<flatbuffers::Vector<'a, u32>>,
  choice: Option<wire::DropdownChoice<'a>>,
  float: Option<f32>,
  integer: Option<i32>,
  range: Option<&wire::FloatRange>,
  text: Option<&'a str>,
) -> UiValueView<'a> {
  match kind {
    wire::UiValue::BoolValue => UiValueView::Bool(boolean.expect("verified UI bool union")),
    wire::UiValue::IndexValue => {
      let value = index.expect("verified UI index union");
      UiValueView::Index(value.present().then(|| value.value()))
    }
    wire::UiValue::IndicesValue => UiValueView::Indices(UiIndicesView {
      values: indices.expect("verified UI indices union"),
    }),
    wire::UiValue::ChoiceValue => {
      let value = choice.expect("verified UI choice union");
      UiValueView::Choice(UiChoiceView {
        index: value.has_index().then(|| value.index()),
        value: value.value(),
      })
    }
    wire::UiValue::F32Value => UiValueView::F32(float.expect("verified UI float union")),
    wire::UiValue::I32Value => UiValueView::I32(integer.expect("verified UI integer union")),
    wire::UiValue::F32RangeValue => {
      let value = range.expect("verified UI range union");
      UiValueView::F32Range {
        min: value.min(),
        max: value.max(),
      }
    }
    wire::UiValue::StringValue => UiValueView::Text(text.expect("verified UI text union")),
    _ => unreachable!("UI value union was semantically validated"),
  }
}

fn validate(value: wire::UiEventAction<'_>) -> Result<(), crate::ProtocolError> {
  let event = value.event();
  if is_nil(value.action_id()) || is_nil(value.session_id()) || is_nil(event.target_id()) {
    return Err(error("UI event UUIDs must be nonzero"));
  }
  if event.default_prevented() && !event.cancelable() {
    return Err(error("a default-prevented UI event must be cancelable"));
  }
  let expected = expected_body(event.kind()).ok_or_else(|| error("unknown UI event kind"))?;
  if event.body_type() != expected {
    return Err(error("UI event kind and payload type do not match"));
  }
  Ok(())
}

fn expected_body(kind: u8) -> Option<wire::UiEventBody> {
  use wire::UiEventBody as Body;
  Some(match kind {
    0 => Body::AccessibilityActionEvent,
    1 | 3 => Body::PointerButtonEvent,
    2 => Body::PointerMoveEvent,
    4 => Body::PointerCancelEvent,
    5 => Body::ClickEvent,
    6 | 7 => Body::PointerBoundaryEvent,
    8 | 9 => Body::PointerCrossingEvent,
    10 => Body::WheelEvent,
    11 | 12 => Body::PointerCaptureEvent,
    13 | 14 => Body::KeyEvent,
    15 => Body::NavigationMoveEvent,
    16 | 22 | 23 => Body::EmptyEvent,
    17..=20 => Body::FocusEvent,
    21 => Body::GeometryEvent,
    24..=26 => Body::TransitionEvent,
    27 => Body::ValueChangingEvent,
    28 => Body::ValueCommitEvent,
    29 => Body::TextInputEvent,
    30 => Body::SelectionEvent,
    31..=34 => Body::LinkEvent,
    35 | 36 => Body::ScrollEvent,
    37 => Body::TabSelectionEvent,
    38 => Body::TabCloseEvent,
    39 => Body::TabReorderEvent,
    _ => return None,
  })
}

fn size_prefix_matches(bytes: &[u8]) -> bool {
  bytes.len() >= 4
    && usize::try_from(u32::from_le_bytes(
      bytes[..4].try_into().expect("length checked"),
    ))
    .ok()
      == bytes.len().checked_sub(4)
}

fn is_nil(value: &Uuid) -> bool {
  value.bytes().iter().all(|byte| byte == 0)
}

pub(crate) fn uuid_bytes(value: &Uuid) -> [u8; 16] {
  let mut bytes = [0; 16];
  for (index, byte) in value.bytes().iter().enumerate() {
    bytes[index] = byte;
  }
  bytes
}

fn error(message: impl Into<String>) -> crate::ProtocolError {
  crate::ProtocolError::new(message)
}

#[cfg(test)]
mod tests {
  use battlement::{ClickEvent, UiEventBody};

  use super::*;

  const ACTION_ID: Uuid = Uuid([1; 16]);
  const SESSION_ID: Uuid = Uuid([2; 16]);
  const TARGET_ID: Uuid = Uuid([3; 16]);

  #[test]
  fn reads_navigation_submit_and_materializes_validated_legacy_value() {
    let bytes = event_bytes(5, wire::UiEventBody::ClickEvent, |builder| {
      wire::ClickEvent::create(
        builder,
        &wire::ClickEventArgs {
          kind: wire::ClickKind::NavigationSubmit,
          pointer: None,
        },
      )
      .as_union_value()
    });
    let view = UiEventActionView::read(&bytes).unwrap();
    assert_eq!(view.action_id(), [1; 16]);
    assert_eq!(view.session_id(), [2; 16]);
    assert_eq!(view.target_id(), [3; 16]);
    assert_eq!(view.kind(), 5);
    assert!(matches!(
      view.to_owned().unwrap().event.body,
      UiEventBody::Click(ClickEvent::NavigationSubmit)
    ));
  }

  #[test]
  fn borrows_text_commit_values_without_reconstructing_the_legacy_event() {
    let bytes = event_bytes(28, wire::UiEventBody::ValueCommitEvent, |builder| {
      let previous_text = builder.create_string("before");
      let previous = wire::StringValue::create(
        builder,
        &wire::StringValueArgs {
          value: Some(previous_text),
        },
      );
      let proposed_text = builder.create_string("after");
      let proposed = wire::StringValue::create(
        builder,
        &wire::StringValueArgs {
          value: Some(proposed_text),
        },
      );
      wire::ValueCommitEvent::create(
        builder,
        &wire::ValueCommitEventArgs {
          previous_type: wire::UiValue::StringValue,
          previous: Some(previous.as_union_value()),
          proposed_type: wire::UiValue::StringValue,
          proposed: Some(proposed.as_union_value()),
        },
      )
      .as_union_value()
    });
    let view = UiEventActionView::read(&bytes).unwrap();
    let commit = view.value_commit().expect("value commit");
    assert!(matches!(commit.previous(), UiValueView::Text("before")));
    assert!(matches!(commit.proposed(), UiValueView::Text("after")));
    assert!(view.value_changing().is_none());
  }

  #[test]
  fn borrows_input_text_and_reads_selection_scalars() {
    let input = event_bytes(29, wire::UiEventBody::TextInputEvent, |builder| {
      let text = builder.create_string("draft");
      wire::TextInputEvent::create(builder, &wire::TextInputEventArgs { value: Some(text) })
        .as_union_value()
    });
    assert_eq!(
      UiEventActionView::read(&input).unwrap().input_text(),
      Some("draft")
    );

    let selection = event_bytes(30, wire::UiEventBody::SelectionEvent, |builder| {
      wire::SelectionEvent::create(
        builder,
        &wire::SelectionEventArgs {
          cursor_index: 7,
          selection_index: 3,
        },
      )
      .as_union_value()
    });
    assert_eq!(
      UiEventActionView::read(&selection).unwrap().selection(),
      Some(UiSelectionView {
        cursor_index: 7,
        selection_index: 3,
      })
    );
  }

  #[test]
  fn rejects_kind_union_mismatch_and_unknown_scalar_enum() {
    let mismatched = event_bytes(5, wire::UiEventBody::EmptyEvent, |builder| {
      wire::EmptyEvent::create(builder, &wire::EmptyEventArgs {}).as_union_value()
    });
    assert!(
      UiEventActionView::read(&mismatched)
        .err()
        .unwrap()
        .to_string()
        .contains("kind and payload")
    );

    let unknown_key = event_bytes(13, wire::UiEventBody::KeyEvent, |builder| {
      let text = builder.create_string("a");
      wire::KeyEvent::create(
        builder,
        &wire::KeyEventArgs {
          has_physical_key: true,
          physical_key: wire::PhysicalKey(u16::MAX),
          text: Some(text),
          modifiers: 0,
        },
      )
      .as_union_value()
    });
    assert!(
      UiEventActionView::read(&unknown_key)
        .err()
        .unwrap()
        .to_string()
        .contains("unknown physical key")
    );
  }

  #[test]
  fn rejects_nonfinite_coordinates_and_inactive_union_scalars() {
    let nonfinite = event_bytes(6, wire::UiEventBody::PointerBoundaryEvent, |builder| {
      let position = wire::PanelPoint::new(f64::NAN, 0.0);
      wire::PointerBoundaryEvent::create(
        builder,
        &wire::PointerBoundaryEventArgs {
          position: Some(&position),
          pointer_id: 1,
          pointer_type: wire::PointerType::Mouse,
        },
      )
      .as_union_value()
    });
    assert!(
      UiEventActionView::read(&nonfinite)
        .err()
        .unwrap()
        .to_string()
        .contains("finite")
    );

    let inactive_scalar = event_bytes(17, wire::UiEventBody::FocusEvent, |builder| {
      wire::FocusEvent::create(
        builder,
        &wire::FocusEventArgs {
          related_target_id: None,
          direction: wire::FocusDirectionKind::Left,
          other_direction: 9,
        },
      )
      .as_union_value()
    });
    assert!(
      UiEventActionView::read(&inactive_scalar)
        .err()
        .unwrap()
        .to_string()
        .contains("outside its selected variant")
    );
  }

  #[test]
  fn rejects_truncation_prefix_mismatch_and_wrong_identifier() {
    let mut bytes = event_bytes(5, wire::UiEventBody::ClickEvent, |builder| {
      wire::ClickEvent::create(
        builder,
        &wire::ClickEventArgs {
          kind: wire::ClickKind::NavigationSubmit,
          pointer: None,
        },
      )
      .as_union_value()
    });
    let truncated = &bytes[..bytes.len() - 1];
    assert!(UiEventActionView::read(truncated).is_err());
    bytes[0] ^= 1;
    assert!(UiEventActionView::read(&bytes).is_err());
    bytes[0] ^= 1;
    bytes[8] ^= 1;
    assert!(UiEventActionView::read(&bytes).is_err());
  }

  #[test]
  fn rejects_invalid_utf8_before_exposing_a_borrowed_string() {
    let mut bytes = event_bytes(29, wire::UiEventBody::TextInputEvent, |builder| {
      let value = builder.create_string("valid-text");
      wire::TextInputEvent::create(builder, &wire::TextInputEventArgs { value: Some(value) })
        .as_union_value()
    });
    let start = bytes
      .windows(b"valid-text".len())
      .position(|window| window == b"valid-text")
      .expect("fixture contains its UTF-8 payload");
    bytes[start + 2] = 0xff;
    assert!(UiEventActionView::read(&bytes).is_err());
  }

  fn event_bytes(
    kind: u8,
    body_type: wire::UiEventBody,
    body: impl for<'a> FnOnce(
      &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>,
  ) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let body = body(&mut builder);
    let event = wire::UiEvent::create(
      &mut builder,
      &wire::UiEventArgs {
        target_id: Some(&TARGET_ID),
        cancelable: true,
        default_prevented: false,
        kind,
        body_type,
        body: Some(body),
      },
    );
    let root = wire::UiEventAction::create(
      &mut builder,
      &wire::UiEventActionArgs {
        action_id: Some(&ACTION_ID),
        session_id: Some(&SESSION_ID),
        event: Some(event),
      },
    );
    wire::finish_size_prefixed_ui_event_action_buffer(&mut builder, root);
    builder.finished_data().to_vec()
  }
}
