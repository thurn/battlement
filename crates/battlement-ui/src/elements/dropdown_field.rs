use serde::{Deserialize, Serialize};

use crate::{
  Choice, LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled single-choice field that opens its options in a popup.
///
/// Use a dropdown when choices are mutually exclusive and keeping every option
/// visible would take too much space. Prefer [`RadioButtonGroup`] when users
/// benefit from scanning all choices at once. [`Self::choices`] defines display
/// order; [`Self::selection`] carries both the zero-based index and matching
/// display value so Unity and Rust agree on the selected option.
///
/// User selection is provisional. Subscribe to
/// [`UiEventKind::ValueCommitted`] to receive the proposed [`Choice`], then send
/// an update with the accepted selection. Until then, the latest Rust-authored
/// selection remains authoritative. [`Self::clear_selection`] explicitly
/// removes a selection during a sparse update.
///
/// The control is a logical leaf. Use the named part-style builders to customize
/// its label, input, displayed text, or arrow.
///
/// See Unity's [DropdownField manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-DropdownField.html)
/// for popup behavior and guidance on choosing between dropdowns and radio groups.
///
/// # Example
///
/// ```
/// use battlement_ui::{DropdownField, UiEventKind};
///
/// let difficulty = DropdownField::new()
///     .label("Difficulty")
///     .choices(["Story", "Standard", "Veteran"])
///     .selection(1, "Standard")
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(difficulty.selection.unwrap().index, Some(1));
/// ```
///
/// [`RadioButtonGroup`]: crate::RadioButtonGroup
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct DropdownField {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: VisualElement,
  /// Caption associated with the field.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  /// Whether the native field displays its mixed-value state.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub show_mixed_value: Option<bool>,
  /// Ordered display-ready option labels.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub choices: Option<Vec<String>>,
  /// Sparse authored selection. An empty choice explicitly clears the field.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub selection: Option<Choice>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl DropdownField {
  /// Creates an empty dropdown with no selection.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `DropdownFieldLabel` part.
  #[must_use]
  pub fn label_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::DropdownFieldLabel, value);
    self
  }

  /// Applies sparse inline declarations to the native `DropdownFieldInput` part.
  #[must_use]
  pub fn input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::DropdownFieldInput, value);
    self
  }

  /// Applies sparse inline declarations to the native `DropdownFieldText` part.
  #[must_use]
  pub fn text_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::DropdownFieldText, value);
    self
  }

  /// Applies sparse inline declarations to the native `DropdownFieldArrow` part.
  #[must_use]
  pub fn arrow_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::DropdownFieldArrow, value);
    self
  }

  /// Sets the field caption.
  #[must_use]
  pub fn label(mut self, value: impl Into<String>) -> Self {
    self.label = Some(value.into());
    self
  }

  /// Enables or disables the native mixed-value presentation.
  #[must_use]
  pub fn show_mixed_value(mut self, value: bool) -> Self {
    self.show_mixed_value = Some(value);
    self
  }

  /// Replaces the ordered option labels.
  #[must_use]
  pub fn choices(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
    self.choices = Some(values.into_iter().map(Into::into).collect());
    self
  }

  /// Selects one option using matching index and display value.
  #[must_use]
  pub fn selection(mut self, index: u32, value: impl Into<String>) -> Self {
    self.selection = Some(Choice::selected(index, value));
    self
  }

  /// Replaces the sparse authored selection with a complete coherent pair.
  #[must_use]
  pub fn selection_value(mut self, value: Choice) -> Self {
    self.selection = Some(value);
    self
  }

  /// Explicitly clears the selected option in a sparse update.
  #[must_use]
  pub fn clear_selection(mut self) -> Self {
    self.selection = Some(Choice::none());
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if value.label.is_some() {
      self.label.clone_from(&value.label);
    }
    if value.show_mixed_value.is_some() {
      self.show_mixed_value = value.show_mixed_value;
    }
    if value.choices.is_some() {
      self.choices.clone_from(&value.choices);
    }
    if value.selection.is_some() {
      self.selection.clone_from(&value.selection);
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl VisualElementProperties for DropdownField {
  fn visual_element(&self) -> &VisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut VisualElement {
    &mut self.element
  }
}
