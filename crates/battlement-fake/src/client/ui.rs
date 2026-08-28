//! Typed UI inspection, synthetic gestures, and interaction reconciliation.

mod events;
mod min_max_slider;
mod slider;
mod text_field;

use std::time::Instant;

use battlement::{ActionBody, Command, PointerButton};
use battlement_native::Engine;
use battlement_ui_fake::{UiElementState, UiJournalEntry, UiWorld};

use crate::client::FakeClient;

#[derive(Clone, Copy)]
pub(super) struct ScrollInteraction {
  pub(super) latest: battlement::Vector,
  pub(super) last_changed: Instant,
  pub(super) captured: bool,
  pub(super) armed: bool,
}

#[derive(Clone, Copy)]
pub(super) struct ScrollerInteraction {
  pub(super) committed: f32,
  pub(super) proposed: f32,
}

#[derive(Clone, Copy)]
pub(super) struct SliderIntInteraction {
  pub(super) committed: i32,
  pub(super) proposed: i32,
}

#[derive(Clone, Copy)]
pub(super) struct MinMaxSliderInteraction {
  pub(super) committed: battlement::F32Range,
  pub(super) proposed: battlement::F32Range,
}

#[derive(Clone)]
pub(super) struct TextFieldInteraction {
  pub(super) committed: String,
  pub(super) draft: String,
}

impl TextFieldInteraction {
  fn proposed_is_unchanged(&self) -> bool {
    self.committed == self.draft
  }
}

/// Typed access to fake UI state and synthetic UI gestures.
pub struct UiClient<'a, E>
where
  E: Engine<Command = Command>,
{
  pub(super) client: &'a mut FakeClient<E>,
}

impl<E> UiClient<'_, E>
where
  E: Engine<Command = Command>,
{
  /// Returns whether an identity belongs to a live logical UI element.
  #[must_use]
  pub fn contains(&self, object_id: battlement::ObjectId) -> bool {
    self.client.ui_world.element(object_id).is_some()
  }

  /// Returns one live logical UI element.
  #[must_use]
  pub fn element(&self, object_id: battlement::ObjectId) -> &UiElementState {
    self
      .client
      .ui_world
      .element(object_id)
      .unwrap_or_else(|| panic!("UI element does not exist: {object_id}"))
  }

  /// Returns successfully executed UI commands.
  #[must_use]
  pub fn journal(&self) -> &[UiJournalEntry] {
    self.client.ui_world.journal()
  }

  /// Returns the logical element that owns keyboard focus.
  #[must_use]
  pub fn focused(&self) -> Option<battlement::ObjectId> {
    self.client.ui_world.focused()
  }

  /// Returns the logical element capturing one pointer.
  #[must_use]
  pub fn pointer_capture(&self, pointer_id: i32) -> Option<battlement::ObjectId> {
    self.client.ui_world.pointer_capture(pointer_id)
  }

  /// Returns the latest action-authored text selection endpoints.
  #[must_use]
  pub fn selection(&self, object_id: battlement::ObjectId) -> Option<(u32, u32)> {
    self.client.ui_world.selection(object_id)
  }

  /// Sends one native-style event when its logical route has a subscription.
  pub fn send_event(&mut self, event: battlement::UiEvent) {
    if !self.client.world.input_enabled() {
      return;
    }
    let _ = self.element(event.target_id);
    if self.client.ui_world.route_event(&event).is_empty() {
      return;
    }
    self.client.submit_action(ActionBody::VisualElement(event));
  }

  /// Sends one pointer-style click when the button is enabled and subscribed.
  pub fn click(&mut self, object_id: battlement::ObjectId) {
    if !self.client.world.input_enabled() {
      return;
    }
    let target = self.element(object_id);
    assert_eq!(
      target.kind(),
      battlement::UiElementKind::Button,
      "UI click target is not a button: {object_id}"
    );
    assert!(
      target.is_enabled().unwrap_or(true),
      "UI click target is disabled: {object_id}"
    );
    let event = battlement::UiEvent::click(
      object_id,
      battlement::ClickEvent::pointer(
        0,
        battlement::PanelPoint::default(),
        PointerButton::Left,
        1,
        battlement::KeyModifiers::default(),
      ),
    );
    if self.client.ui_world.route_event(&event).is_empty() {
      return;
    }
    self.client.submit_action(ActionBody::VisualElement(event));
  }

  /// Activates a Button through keyboard or gamepad submit.
  pub fn navigation_submit(&mut self, object_id: battlement::ObjectId) {
    if !self.client.world.input_enabled() {
      return;
    }
    let target = self.element(object_id);
    assert_eq!(
      target.kind(),
      battlement::UiElementKind::Button,
      "UI navigation submit target is not a button: {object_id}"
    );
    assert!(
      target.is_enabled().unwrap_or(true),
      "UI navigation submit target is disabled: {object_id}"
    );
    let event = battlement::UiEvent::click(object_id, battlement::ClickEvent::NavigationSubmit);
    if self.client.ui_world.route_event(&event).is_empty() {
      return;
    }
    self.client.submit_action(ActionBody::VisualElement(event));
  }

  /// Presses and holds a repeat button for an exact number of milliseconds.
  ///
  /// The returned count includes the immediate press callback and every timer
  /// callback whose deadline is at or before `held_ms`. Release adds nothing.
  pub fn repeat_hold(&mut self, object_id: battlement::ObjectId, held_ms: u64) -> usize {
    if !self.client.world.input_enabled() {
      return 0;
    }
    let target = self.element(object_id);
    assert_eq!(
      target.kind(),
      battlement::UiElementKind::RepeatButton,
      "UI repeat target is not a repeat button: {object_id}"
    );
    assert!(
      target.is_enabled().unwrap_or(true),
      "UI repeat target is disabled: {object_id}"
    );
    let (delay, interval) = self
      .client
      .ui_world
      .repeat_timing(object_id)
      .expect("repeat timing missing");
    let delay = u64::from(delay);
    let interval = u64::from(interval.get());
    let callbacks = 1
      + usize::try_from(
        held_ms
          .checked_sub(delay)
          .map_or(0, |elapsed| elapsed / interval + 1),
      )
      .expect("repeat callback count exceeds usize");
    let event = battlement::UiEvent::click(object_id, battlement::ClickEvent::Repeat);
    if self.client.ui_world.route_event(&event).is_empty() {
      return 0;
    }
    for _ in 0..callbacks {
      self
        .client
        .submit_action(ActionBody::VisualElement(event.clone()));
    }
    callbacks
  }

  /// Begins pointer capture for a scroll gesture without changing the offset.
  pub fn scroll_begin(&mut self, object_id: battlement::ObjectId) {
    self.require_scroll_view(object_id);
    if !self.input_available(object_id) {
      return;
    }
    let now = self.clock_now();
    let latest = self.scroll_offset(object_id);
    self
      .client
      .scroll_interactions
      .entry(object_id)
      .and_modify(|state| state.captured = true)
      .or_insert(ScrollInteraction {
        latest,
        last_changed: now,
        captured: true,
        armed: false,
      });
  }

  /// Applies one user-originated scroll offset and emits an optional live event.
  pub fn scroll_change(&mut self, object_id: battlement::ObjectId, offset: battlement::Vector) {
    self.require_scroll_view(object_id);
    assert!(
      offset.x.is_finite() && offset.y.is_finite(),
      "scroll offset must be finite"
    );
    if !self.input_available(object_id) {
      self.client.scroll_interactions.remove(&object_id);
      return;
    }
    let now = self.clock_now();
    let captured = self
      .client
      .scroll_interactions
      .get(&object_id)
      .is_some_and(|state| state.captured);
    self.client.scroll_interactions.insert(
      object_id,
      ScrollInteraction {
        latest: offset,
        last_changed: now,
        captured,
        armed: true,
      },
    );
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::ScrollChanged)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::ScrollChanged(battlement::ScrollEvent { offset }),
        }));
    }
  }

  /// Releases pointer capture for a scroll gesture without forcing settlement.
  pub fn scroll_end(&mut self, object_id: battlement::ObjectId) {
    if let Some(state) = self.client.scroll_interactions.get_mut(&object_id) {
      state.captured = false;
    }
  }

  /// Emits scroll settlements whose exact 100-millisecond deadline has elapsed.
  pub fn advance(&mut self) {
    let now = self.clock_now();
    let settled = self
      .client
      .scroll_interactions
      .iter()
      .filter_map(|(object_id, state)| {
        let elapsed = now.duration_since(state.last_changed);
        (state.armed && !state.captured && elapsed >= std::time::Duration::from_millis(100))
          .then_some((*object_id, state.latest))
      })
      .collect::<Vec<_>>();
    for (object_id, offset) in settled {
      if let Some(state) = self.client.scroll_interactions.get_mut(&object_id) {
        state.armed = false;
      }
      if self.input_available(object_id)
        && self
          .client
          .ui_world
          .has_subscription(object_id, battlement::UiEventKind::ScrollSettled)
      {
        self
          .client
          .submit_action(ActionBody::VisualElement(battlement::UiEvent {
            target_id: object_id,
            body: battlement::UiEventBody::ScrollSettled(battlement::ScrollEvent { offset }),
          }));
      }
    }
  }

  /// Begins a controlled Scroller drag from its latest Rust-authored value.
  pub fn scroller_begin(&mut self, object_id: battlement::ObjectId) {
    self.require_scroller(object_id);
    if !self.input_available(object_id) {
      return;
    }
    let committed = self.scroller_value(object_id);
    self.client.scroller_interactions.insert(
      object_id,
      ScrollerInteraction {
        committed,
        proposed: committed,
      },
    );
  }

  /// Changes a controlled Scroller's local proposal during pointer capture.
  pub fn scroller_change(&mut self, object_id: battlement::ObjectId, proposed: f32) {
    assert!(proposed.is_finite(), "scroller proposal must be finite");
    let proposed = self.clamp_scroller_value(object_id, proposed);
    self
      .client
      .scroller_interactions
      .get_mut(&object_id)
      .unwrap_or_else(|| panic!("scroller is not captured: {object_id}"))
      .proposed = proposed;
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::ValueChanging)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::ValueChanging(battlement::ValueChangingEvent {
            proposed: battlement::UiValue::F32(proposed),
          }),
        }));
    }
  }

  /// Releases a controlled Scroller and submits one final proposal when subscribed.
  pub fn scroller_commit(&mut self, object_id: battlement::ObjectId) {
    let state = self
      .client
      .scroller_interactions
      .remove(&object_id)
      .unwrap_or_else(|| panic!("scroller is not captured: {object_id}"));
    if !self.input_available(object_id) {
      return;
    }
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::ValueCommitted)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::ValueCommitted(battlement::ValueCommitEvent {
            previous: battlement::UiValue::F32(state.committed),
            proposed: battlement::UiValue::F32(state.proposed),
          }),
        }));
    }
  }

  /// Cancels a controlled Scroller gesture without emitting a final proposal.
  pub fn scroller_cancel(&mut self, object_id: battlement::ObjectId) {
    self.client.scroller_interactions.remove(&object_id);
  }

  /// Activates a toggle and submits its inverted controlled value.
  pub fn toggle_click(&mut self, object_id: battlement::ObjectId) {
    self.require_kind(object_id, battlement::UiElementKind::Toggle);
    let previous = self.boolean_value(object_id);
    self.submit_boolean_proposal(object_id, previous, !previous);
  }

  /// Activates a standalone radio button and proposes selection.
  pub fn radio_click(&mut self, object_id: battlement::ObjectId) {
    self.require_kind(object_id, battlement::UiElementKind::RadioButton);
    let previous = self.boolean_value(object_id);
    self.submit_boolean_proposal(object_id, previous, true);
  }

  /// Proposes one controlled radio-group option without mutating authored state.
  pub fn radio_group_select(&mut self, object_id: battlement::ObjectId, proposed_index: u32) {
    self.require_kind(object_id, battlement::UiElementKind::RadioButtonGroup);
    let battlement::UiElement::RadioButtonGroup(value) = self.element(object_id).element() else {
      unreachable!("validated radio group kind changed")
    };
    assert!(
      (proposed_index as usize)
        < match &value.choices {
          battlement::Prop::Set(choices) => choices.len(),
          battlement::Prop::Unset | battlement::Prop::Reset => 0,
        },
      "radio selection is out of range: {proposed_index}"
    );
    let previous = match value.selected_index {
      battlement::Prop::Set(index) => Some(index),
      battlement::Prop::Unset | battlement::Prop::Reset => None,
    };
    self.submit_choice_proposal(
      object_id,
      battlement::UiValue::Index(previous),
      battlement::UiValue::Index(Some(proposed_index)),
    );
  }

  /// Activates one controlled toggle-group button without mutating authored state.
  pub fn toggle_group_click(&mut self, object_id: battlement::ObjectId, index: u32) {
    self.require_kind(object_id, battlement::UiElementKind::ToggleButtonGroup);
    let target = self.element(object_id);
    assert!(
      (index as usize) < target.children().len(),
      "toggle button index is out of range: {index}"
    );
    let child_id = target.children()[index as usize];
    if !self.input_available(child_id) {
      return;
    }
    let battlement::UiElement::ToggleButtonGroup(value) = target.element() else {
      unreachable!("validated toggle group kind changed")
    };
    let mut previous = match &value.selected_indices {
      battlement::Prop::Set(indices) => indices.clone(),
      battlement::Prop::Unset | battlement::Prop::Reset
        if target.children().is_empty()
          || matches!(value.allow_empty_selection, battlement::Prop::Set(true)) =>
      {
        Vec::new()
      }
      battlement::Prop::Unset | battlement::Prop::Reset => vec![0],
    };
    let mut proposed = previous.clone();
    if matches!(value.multiple_selection, battlement::Prop::Set(true)) {
      if let Ok(position) = proposed.binary_search(&index) {
        proposed.remove(position);
      } else {
        proposed.push(index);
        proposed.sort_unstable();
      }
    } else if proposed == [index]
      && matches!(value.allow_empty_selection, battlement::Prop::Set(true))
    {
      proposed.clear();
    } else {
      proposed.clear();
      proposed.push(index);
    }
    if proposed.is_empty() && !matches!(value.allow_empty_selection, battlement::Prop::Set(true)) {
      return;
    }
    self.submit_choice_proposal(
      object_id,
      battlement::UiValue::Indices(std::mem::take(&mut previous)),
      battlement::UiValue::Indices(proposed),
    );
  }

  /// Proposes one controlled dropdown option without mutating authored state.
  pub fn dropdown_select(&mut self, object_id: battlement::ObjectId, index: u32) {
    self.require_kind(object_id, battlement::UiElementKind::DropdownField);
    let battlement::UiElement::DropdownField(value) = self.element(object_id).element() else {
      unreachable!("validated dropdown kind changed")
    };
    let battlement::Prop::Set(choices) = &value.choices else {
      panic!("dropdown selection is out of range: {index}");
    };
    let choice = choices
      .get(index as usize)
      .unwrap_or_else(|| panic!("dropdown selection is out of range: {index}"));
    let previous = match &value.selection {
      battlement::Prop::Set(selection) => selection.clone(),
      battlement::Prop::Unset | battlement::Prop::Reset => battlement::Choice::none(),
    };
    self.submit_choice_proposal(
      object_id,
      battlement::UiValue::Choice(previous),
      battlement::UiValue::Choice(battlement::Choice::selected(index, choice)),
    );
  }

  /// Proposes clearing a controlled dropdown without mutating authored state.
  pub fn dropdown_clear(&mut self, object_id: battlement::ObjectId) {
    self.require_kind(object_id, battlement::UiElementKind::DropdownField);
    let battlement::UiElement::DropdownField(value) = self.element(object_id).element() else {
      unreachable!("validated dropdown kind changed")
    };
    self.submit_choice_proposal(
      object_id,
      battlement::UiValue::Choice(match &value.selection {
        battlement::Prop::Set(selection) => selection.clone(),
        battlement::Prop::Unset | battlement::Prop::Reset => battlement::Choice::none(),
      }),
      battlement::UiValue::Choice(battlement::Choice::none()),
    );
  }

  /// Proposes a controlled active-tab change without mutating authored state.
  pub fn tab_select(&mut self, object_id: battlement::ObjectId, proposed_index: u32) {
    self.require_tab_view(object_id);
    if !self.input_available(object_id) {
      return;
    }
    let children = self.element(object_id).children();
    let proposed_tab_id = *children
      .get(proposed_index as usize)
      .unwrap_or_else(|| panic!("tab selection is out of range: {proposed_index}"));
    let previous_index = self.tab_selected_index(object_id);
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::TabSelectionRequested)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::TabSelectionRequested(battlement::TabSelectionEvent {
            previous_index,
            proposed_index,
            proposed_tab_id,
          }),
        }));
    }
  }

  /// Proposes closing one tab while preserving it until Rust destroys it.
  pub fn tab_close(&mut self, object_id: battlement::ObjectId, index: u32) {
    self.require_tab_view(object_id);
    if !self.input_available(object_id) {
      return;
    }
    let tab_id = *self
      .element(object_id)
      .children()
      .get(index as usize)
      .unwrap_or_else(|| panic!("tab close index is out of range: {index}"));
    let battlement::UiElement::Tab(tab) = self.element(tab_id).element() else {
      panic!("TabView child is not a Tab: {tab_id}");
    };
    if tab.closeable != battlement::Prop::Set(true) {
      return;
    }
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::TabCloseRequested)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::TabCloseRequested(battlement::TabCloseEvent {
            tab_id,
            index,
          }),
        }));
    }
  }

  /// Proposes moving a tab header while preserving authored logical order.
  pub fn tab_reorder(
    &mut self,
    object_id: battlement::ObjectId,
    previous_index: u32,
    proposed_index: u32,
  ) {
    self.require_tab_view(object_id);
    if !self.input_available(object_id) {
      return;
    }
    let battlement::UiElement::TabView(tab_view) = self.element(object_id).element() else {
      unreachable!("tab view kind changed after validation");
    };
    if tab_view.reorderable != battlement::Prop::Set(true) {
      return;
    }
    let children = self.element(object_id).children();
    let tab_id = *children
      .get(previous_index as usize)
      .unwrap_or_else(|| panic!("tab source index is out of range: {previous_index}"));
    assert!(
      (proposed_index as usize) < children.len(),
      "tab destination index is out of range: {proposed_index}"
    );
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::TabReorderRequested)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::TabReorderRequested(battlement::TabReorderEvent {
            tab_id,
            previous_index,
            proposed_index,
          }),
        }));
    }
  }

  fn require_scroll_view(&self, object_id: battlement::ObjectId) {
    assert_eq!(
      self.element(object_id).kind(),
      battlement::UiElementKind::ScrollView
    );
  }

  fn require_scroller(&self, object_id: battlement::ObjectId) {
    assert_eq!(
      self.element(object_id).kind(),
      battlement::UiElementKind::Scroller
    );
  }

  fn require_tab_view(&self, object_id: battlement::ObjectId) {
    assert_eq!(
      self.element(object_id).kind(),
      battlement::UiElementKind::TabView
    );
  }

  fn require_text_field(&self, object_id: battlement::ObjectId) {
    assert_eq!(
      self.element(object_id).kind(),
      battlement::UiElementKind::TextField
    );
  }

  fn require_kind(&self, object_id: battlement::ObjectId, expected: battlement::UiElementKind) {
    assert_eq!(self.element(object_id).kind(), expected);
  }

  fn submit_boolean_proposal(
    &mut self,
    object_id: battlement::ObjectId,
    previous: bool,
    proposed: bool,
  ) {
    if !self.input_available(object_id) || previous == proposed {
      return;
    }
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::ValueCommitted)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::ValueCommitted(battlement::ValueCommitEvent {
            previous: battlement::UiValue::Bool(previous),
            proposed: battlement::UiValue::Bool(proposed),
          }),
        }));
    }
  }

  fn submit_choice_proposal(
    &mut self,
    object_id: battlement::ObjectId,
    previous: battlement::UiValue,
    proposed: battlement::UiValue,
  ) {
    if !self.input_available(object_id) || previous == proposed {
      return;
    }
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::ValueCommitted)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::ValueCommitted(battlement::ValueCommitEvent {
            previous,
            proposed,
          }),
        }));
    }
  }

  fn boolean_value(&self, object_id: battlement::ObjectId) -> bool {
    match self.element(object_id).element() {
      battlement::UiElement::Toggle(value) => prop_bool(value.value),
      battlement::UiElement::RadioButton(value) => prop_bool(value.value),
      _ => unreachable!("validated Boolean control kind changed"),
    }
  }

  pub(super) fn input_available(&self, object_id: battlement::ObjectId) -> bool {
    self.client.world.input_enabled() && self.element_enabled_in_hierarchy(object_id)
  }

  fn text_edit_available(&self, object_id: battlement::ObjectId) -> bool {
    if !self.input_available(object_id) {
      return false;
    }
    let battlement::UiElement::TextField(value) = self.element(object_id).element() else {
      unreachable!("validated text field kind changed")
    };
    !matches!(value.read_only, battlement::Prop::Set(true))
  }

  fn text_value(&self, object_id: battlement::ObjectId) -> &str {
    let battlement::UiElement::TextField(value) = self.element(object_id).element() else {
      unreachable!("validated text field kind changed")
    };
    match &value.value {
      battlement::Prop::Set(value) => value,
      battlement::Prop::Unset | battlement::Prop::Reset => "",
    }
  }

  fn element_enabled_in_hierarchy(&self, object_id: battlement::ObjectId) -> bool {
    let mut current = Some(object_id);
    while let Some(value) = current {
      let element = self.element(value);
      if element.is_enabled() == Some(false) {
        return false;
      }
      current = element.parent_id();
    }
    true
  }

  fn clock_now(&self) -> Instant {
    self
      .client
      .clock
      .as_ref()
      .expect("scroll settlement requires FakeClient::connect_clocked")
      .now()
  }

  fn scroll_offset(&self, object_id: battlement::ObjectId) -> battlement::Vector {
    let battlement::UiElement::ScrollView(value) = self.element(object_id).element() else {
      unreachable!("validated scroll view kind changed")
    };
    match value.scroll_offset {
      battlement::Prop::Set(offset) => offset,
      battlement::Prop::Unset | battlement::Prop::Reset => battlement::Vector::default(),
    }
  }

  fn scroller_value(&self, object_id: battlement::ObjectId) -> f32 {
    let battlement::UiElement::Scroller(value) = self.element(object_id).element() else {
      unreachable!("validated scroller kind changed")
    };
    prop_float(value.value)
  }

  fn clamp_scroller_value(&self, object_id: battlement::ObjectId, proposed: f32) -> f32 {
    let battlement::UiElement::Scroller(value) = self.element(object_id).element() else {
      unreachable!("validated scroller kind changed")
    };
    let low = prop_float(value.low_value);
    let high = prop_float(value.high_value);
    proposed.clamp(low.min(high), low.max(high))
  }

  fn tab_selected_index(&self, object_id: battlement::ObjectId) -> u32 {
    let battlement::UiElement::TabView(value) = self.element(object_id).element() else {
      unreachable!("validated tab view kind changed")
    };
    match value.selected_tab_index {
      battlement::Prop::Set(index) => index,
      battlement::Prop::Unset | battlement::Prop::Reset => 0,
    }
  }
}

impl<E> FakeClient<E>
where
  E: Engine<Command = Command>,
{
  pub(crate) fn reconcile_ui_interactions(&mut self, body: &battlement::CommandBody) {
    if matches!(body, battlement::CommandBody::InputSetEnabled(value) if !value.enabled) {
      self.scroll_interactions.clear();
      self.scroller_interactions.clear();
      self.slider_interactions.clear();
      self.slider_int_interactions.clear();
      self.min_max_slider_interactions.clear();
      self.text_field_interactions.clear();
      self.ui_link_identities.clear();
      return;
    }
    if let battlement::CommandBody::VisualElementUpdate(value) = body
      && let battlement::VisualElementUpdate::Properties { object_id, element } = value.as_ref()
    {
      if matches!(&**element, battlement::UiElement::ScrollView(value) if !matches!(value.scroll_offset, battlement::Prop::Unset))
      {
        self.scroll_interactions.remove(object_id);
      }
      if let battlement::UiElement::Scroller(value) = &**element
        && !matches!(value.value, battlement::Prop::Unset)
        && let Some(state) = self.scroller_interactions.get_mut(object_id)
      {
        state.committed = prop_float(value.value);
      }
      if let battlement::UiElement::Slider(value) = &**element
        && !matches!(value.value, battlement::Prop::Unset)
        && let Some(state) = self.slider_interactions.get_mut(object_id)
      {
        state.committed = prop_float(value.value);
      }
      if let battlement::UiElement::SliderInt(value) = &**element
        && !matches!(value.value, battlement::Prop::Unset)
        && let Some(state) = self.slider_int_interactions.get_mut(object_id)
      {
        state.committed = match value.value {
          battlement::Prop::Set(value) => value,
          battlement::Prop::Unset | battlement::Prop::Reset => 0,
        };
      }
      if let battlement::UiElement::MinMaxSlider(value) = &**element
        && (!matches!(value.min_value, battlement::Prop::Unset)
          || !matches!(value.max_value, battlement::Prop::Unset))
      {
        self.min_max_slider_interactions.remove(object_id);
      }
      if matches!(&**element, battlement::UiElement::TextField(value) if !matches!(value.value, battlement::Prop::Unset))
      {
        self.text_field_interactions.remove(object_id);
      }
    }
    let world = &self.ui_world;
    self
      .scroll_interactions
      .retain(|object_id, _| ui_element_enabled(world, *object_id));
    self
      .scroller_interactions
      .retain(|object_id, _| ui_element_enabled(world, *object_id));
    self
      .slider_interactions
      .retain(|object_id, _| ui_element_enabled(world, *object_id));
    self
      .slider_int_interactions
      .retain(|object_id, _| ui_element_enabled(world, *object_id));
    self
      .min_max_slider_interactions
      .retain(|object_id, _| ui_element_enabled(world, *object_id));
    self
      .text_field_interactions
      .retain(|object_id, _| ui_element_enabled(world, *object_id));
    self
      .ui_link_identities
      .retain(|(object_id, _), _| ui_element_enabled(world, *object_id));
  }
}

fn prop_float(value: battlement::Prop<f32>) -> f32 {
  match value {
    battlement::Prop::Set(value) => value,
    battlement::Prop::Unset | battlement::Prop::Reset => 0.0,
  }
}

fn prop_bool(value: battlement::Prop<bool>) -> bool {
  match value {
    battlement::Prop::Set(value) => value,
    battlement::Prop::Unset | battlement::Prop::Reset => false,
  }
}

fn ui_element_enabled(world: &UiWorld, object_id: battlement::ObjectId) -> bool {
  let mut current = Some(object_id);
  while let Some(value) = current {
    let Some(element) = world.element(value) else {
      return false;
    };
    if element.is_enabled() == Some(false) {
      return false;
    }
    current = element.parent_id();
  }
  true
}
