use battlement::ActionBody;
use battlement_native::Engine;

use crate::client::ui::{TextFieldInteraction, UiClient};

impl<E> UiClient<'_, E>
where
  E: Engine<Command = battlement::Command>,
{
  /// Applies one native text edit and optionally forwards the complete local draft.
  pub fn text_input(&mut self, object_id: battlement::ObjectId, draft: impl Into<String>) {
    self.require_text_field(object_id);
    if !self.text_edit_available(object_id) {
      self.client.text_field_interactions.remove(&object_id);
      return;
    }
    let draft = draft.into();
    let committed = self.text_value(object_id).to_owned();
    self
      .client
      .text_field_interactions
      .entry(object_id)
      .and_modify(|state| state.draft.clone_from(&draft))
      .or_insert(TextFieldInteraction {
        committed,
        draft: draft.clone(),
      });
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::Input)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::Input(battlement::TextInputEvent { value: draft }),
        }));
    }
  }

  /// Returns the native draft, or the committed value when no edit is active.
  #[must_use]
  pub fn text_draft(&self, object_id: battlement::ObjectId) -> &str {
    self.require_text_field(object_id);
    self
      .client
      .text_field_interactions
      .get(&object_id)
      .map_or_else(|| self.text_value(object_id), |state| state.draft.as_str())
  }

  /// Commits one local text draft and immediately restores authored fake state.
  pub fn text_commit(&mut self, object_id: battlement::ObjectId) {
    self.require_text_field(object_id);
    let Some(state) = self.client.text_field_interactions.remove(&object_id) else {
      return;
    };
    if !self.text_edit_available(object_id) || state.proposed_is_unchanged() {
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
            previous: battlement::UiValue::String(state.committed),
            proposed: battlement::UiValue::String(state.draft),
          }),
        }));
    }
  }

  /// Cancels one local text draft without emitting an action.
  pub fn text_escape(&mut self, object_id: battlement::ObjectId) {
    self.require_text_field(object_id);
    self.client.text_field_interactions.remove(&object_id);
  }

  /// Emits one logical native caret or selection mutation when subscribed.
  pub fn text_selection(
    &mut self,
    object_id: battlement::ObjectId,
    cursor_index: u32,
    select_index: u32,
  ) {
    let length = match self.element(object_id).element() {
      battlement::UiElement::TextField(_) => self.text_draft(object_id).encode_utf16().count(),
      battlement::UiElement::TextElement(value) => value
        .text
        .as_deref()
        .unwrap_or_default()
        .encode_utf16()
        .count(),
      _ => panic!("text selection requires a TextField or TextElement"),
    };
    assert!(
      (cursor_index as usize) <= length,
      "text cursor index is out of range"
    );
    assert!(
      (select_index as usize) <= length,
      "text select index is out of range"
    );
    if !self.input_available(object_id) {
      return;
    }
    if self
      .client
      .ui_world
      .has_subscription(object_id, battlement::UiEventKind::SelectionChanged)
    {
      self
        .client
        .submit_action(ActionBody::VisualElement(battlement::UiEvent {
          target_id: object_id,
          body: battlement::UiEventBody::SelectionChanged(battlement::SelectionEvent {
            cursor_index,
            selection_index: select_index,
          }),
        }));
    }
  }
}
