use battlement_types::ObjectId;
use battlement_ui::{UiElement, UiElementKind, VisualElementAction};

use crate::{UiJournalEntry, UiWorld, UiWorldError};

impl UiWorld {
  /// Returns the element that owns keyboard focus.
  #[must_use]
  pub const fn focused(&self) -> Option<ObjectId> {
    self.focused
  }

  /// Returns the logical element capturing one pointer.
  #[must_use]
  pub fn pointer_capture(&self, pointer_id: i32) -> Option<ObjectId> {
    self.pointer_captures.get(&pointer_id).copied()
  }

  /// Returns the latest requested UTF-16 cursor and selection endpoints.
  #[must_use]
  pub fn selection(&self, object_id: ObjectId) -> Option<(u32, u32)> {
    self.selections.get(&object_id).copied()
  }

  /// Clears focus and pointer capture after input is disabled.
  pub fn clear_interaction_state(&mut self) {
    self.focused = None;
    self.pointer_captures.clear();
  }

  /// Validates and records a transient native-style UI action.
  pub fn perform_action(
    &mut self,
    object_id: ObjectId,
    action: &VisualElementAction,
  ) -> Result<(), UiWorldError> {
    let target = self
      .elements
      .get(&object_id)
      .ok_or(UiWorldError::UnknownObject)?;
    match action {
      VisualElementAction::Focus => {
        if !self.enabled_in_hierarchy(object_id) || !focusable(target) {
          return Err(UiWorldError::InvalidProperty);
        }
        self.focused = Some(object_id);
      }
      VisualElementAction::Blur => {
        if self.focused != Some(object_id) {
          return Err(UiWorldError::InvalidProperty);
        }
        self.focused = None;
      }
      VisualElementAction::CapturePointer { pointer_id } => {
        if !self.enabled_in_hierarchy(object_id) {
          return Err(UiWorldError::InvalidProperty);
        }
        self.pointer_captures.insert(*pointer_id, object_id);
      }
      VisualElementAction::ReleasePointer { pointer_id } => {
        if self.pointer_captures.get(pointer_id) != Some(&object_id) {
          return Err(UiWorldError::InvalidProperty);
        }
        self.pointer_captures.remove(pointer_id);
      }
      VisualElementAction::ScrollTo { descendant_id } => {
        if target.kind() != UiElementKind::ScrollView {
          return Err(UiWorldError::InvalidProperty);
        }
        if !self.elements.contains_key(descendant_id) {
          return Err(UiWorldError::UnknownObject);
        }
        if *descendant_id == object_id || !self.is_descendant(*descendant_id, object_id) {
          return Err(UiWorldError::InvalidHierarchy);
        }
      }
      VisualElementAction::SelectText {
        cursor_index,
        selection_index,
      } => {
        let selectable = match target.element() {
          UiElement::TextField(_) => true,
          UiElement::TextElement(value) => value.selectable == Some(true),
          _ => return Err(UiWorldError::InvalidProperty),
        };
        if !selectable {
          return Err(UiWorldError::InvalidProperty);
        }
        let length = target.text().unwrap_or_default().encode_utf16().count();
        if *cursor_index as usize > length || *selection_index as usize > length {
          return Err(UiWorldError::InvalidProperty);
        }
        self
          .selections
          .insert(object_id, (*cursor_index, *selection_index));
      }
    }
    self
      .journal
      .push(UiJournalEntry::Action(object_id, action.clone()));
    Ok(())
  }
}

fn focusable(target: &crate::UiElementState) -> bool {
  target.is_focusable().unwrap_or(matches!(
    target.kind(),
    UiElementKind::TextField
      | UiElementKind::Toggle
      | UiElementKind::RadioButton
      | UiElementKind::RadioButtonGroup
      | UiElementKind::ToggleButtonGroup
      | UiElementKind::DropdownField
      | UiElementKind::Button
      | UiElementKind::RepeatButton
      | UiElementKind::Scroller
      | UiElementKind::Slider
      | UiElementKind::SliderInt
      | UiElementKind::MinMaxSlider
      | UiElementKind::Tab
  ))
}
