use serde::{Deserialize, Serialize};

use crate::{
  Choice, LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled single-choice field that opens its options in a popup.
///
/// Use a dropdown when choices are mutually exclusive and keeping every option
/// visible would take too much space. Prefer [`UiRadioButtonGroup`] when users
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
/// use battlement_ui::{Choice, UiDropdownField, Prop, UiEventKind};
///
/// let difficulty = UiDropdownField::new()
///     .label("Difficulty")
///     .choices(["Story", "Standard", "Veteran"])
///     .selection(1, "Standard")
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(difficulty.selection, Prop::Set(Choice::selected(1, "Standard")));
/// ```
///
/// [`UiRadioButtonGroup`]: crate::UiRadioButtonGroup
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiDropdownField {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Caption associated with the field.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub label: Prop<String>,
  /// Whether the native field displays its mixed-value state.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub show_mixed_value: Prop<bool>,
  /// Ordered display-ready option labels.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub choices: Prop<Vec<String>>,
  /// Sparse authored selection. An empty choice explicitly clears the field.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub selection: Prop<Choice>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiDropdownField {
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
  pub fn label(mut self, value: impl Into<Prop<String>>) -> Self {
    self.label = value.into();
    self
  }

  /// Enables or disables the native mixed-value presentation.
  #[must_use]
  pub fn show_mixed_value(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.show_mixed_value = value.into();
    self
  }

  /// Replaces the ordered option labels.
  #[must_use]
  pub fn choices(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
    self.choices = Prop::Set(values.into_iter().map(Into::into).collect());
    self
  }

  /// Replaces or resets the ordered option labels.
  #[must_use]
  pub fn choices_value(mut self, value: impl Into<Prop<Vec<String>>>) -> Self {
    self.choices = value.into();
    self
  }

  /// Selects one option using matching index and display value.
  #[must_use]
  pub fn selection(mut self, index: u32, value: impl Into<String>) -> Self {
    self.selection = Prop::Set(Choice::selected(index, value));
    self
  }

  /// Replaces the sparse authored selection with a complete coherent pair.
  #[must_use]
  pub fn selection_value(mut self, value: impl Into<Prop<Choice>>) -> Self {
    self.selection = value.into();
    self
  }

  /// Explicitly clears the selected option in a sparse update.
  #[must_use]
  pub fn clear_selection(mut self) -> Self {
    self.selection = Prop::Set(Choice::none());
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.label, Prop::Unset) {
      self.label.clone_from(&value.label);
    }
    if !matches!(value.show_mixed_value, Prop::Unset) {
      self.show_mixed_value = value.show_mixed_value;
    }
    if !matches!(value.choices, Prop::Unset) {
      self.choices.clone_from(&value.choices);
    }
    if !matches!(value.selection, Prop::Unset) {
      self.selection.clone_from(&value.selection);
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiDropdownField {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
