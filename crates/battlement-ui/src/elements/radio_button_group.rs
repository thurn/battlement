use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled single-choice field that keeps every option visible.
///
/// Use a radio group when choices are mutually exclusive and users should be
/// able to compare them without opening a popup. Prefer [`UiDropdownField`] when
/// space is limited or the list is long. [`Self::choices`] defines the visible
/// options in order, and [`Self::selected_index`] selects one by its zero-based
/// position.
///
/// User activation proposes a new index through
/// [`UiEventKind::ValueCommitted`]. Rust remains authoritative until an update
/// changes [`Self::selected_index`]. Choices are native radio controls, not
/// logical [`UiNode`] children; use the indexed part-style builders to customize
/// an individual option.
///
/// See Unity's [RadioButtonGroup manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-RadioButtonGroup.html)
/// for selection behavior and the choice-list attributes.
///
/// # Example
///
/// ```
/// use battlement_ui::{Prop, UiRadioButtonGroup, UiEventKind};
///
/// let quality = UiRadioButtonGroup::new()
///     .label("Quality")
///     .choices(["Low", "Medium", "High"])
///     .selected_index(2)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(quality.selected_index, Prop::Set(2));
/// ```
///
/// [`UiDropdownField`]: crate::UiDropdownField
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
/// [`UiNode`]: crate::UiNode
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiRadioButtonGroup {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Caption associated with the complete field.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub label: Prop<String>,
  /// Ordered display-ready option labels.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub choices: Prop<Vec<String>>,
  /// Zero-based Rust-authored option index.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub selected_index: Prop<u32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiRadioButtonGroup {
  /// Creates an empty radio group with no selection.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `RadioButtonGroupLabel` part.
  #[must_use]
  pub fn label_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::RadioButtonGroupLabel, value);
    self
  }

  /// Applies sparse inline declarations to the native `RadioButtonGroupInput` part.
  #[must_use]
  pub fn input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::RadioButtonGroupInput, value);
    self
  }

  /// Applies sparse inline declarations to the native `RadioButtonGroupChoicesContainer` part.
  #[must_use]
  pub fn choices_container_style(mut self, value: Style) -> Self {
    parts::append(
      &mut self.parts,
      Part::RadioButtonGroupChoicesContainer,
      value,
    );
    self
  }

  /// Applies sparse inline declarations to the native `RadioButtonGroupContentContainer` part.
  #[must_use]
  pub fn content_container_style(mut self, value: Style) -> Self {
    parts::append(
      &mut self.parts,
      Part::RadioButtonGroupContentContainer,
      value,
    );
    self
  }

  /// Applies sparse inline declarations to the native `RadioButtonGroupAllOptions` part.
  #[must_use]
  pub fn all_options_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::RadioButtonGroupAllOptions, value);
    self
  }

  /// Styles one native radio option by zero-based choice index.
  #[must_use]
  pub fn option_style(mut self, index: u32, value: Style) -> Self {
    parts::append_indexed(&mut self.parts, Part::RadioButtonGroupOption, index, value);
    self
  }

  /// Styles one option's checkmark background by zero-based choice index.
  #[must_use]
  pub fn option_checkmark_background_style(mut self, index: u32, value: Style) -> Self {
    parts::append_indexed(
      &mut self.parts,
      Part::RadioButtonGroupOptionCheckmarkBackground,
      index,
      value,
    );
    self
  }

  /// Styles one option's checkmark by zero-based choice index.
  #[must_use]
  pub fn option_checkmark_style(mut self, index: u32, value: Style) -> Self {
    parts::append_indexed(
      &mut self.parts,
      Part::RadioButtonGroupOptionCheckmark,
      index,
      value,
    );
    self
  }

  /// Styles one option's text by zero-based choice index.
  #[must_use]
  pub fn option_text_style(mut self, index: u32, value: Style) -> Self {
    parts::append_indexed(
      &mut self.parts,
      Part::RadioButtonGroupOptionText,
      index,
      value,
    );
    self
  }

  /// Sets the field caption.
  #[must_use]
  pub fn label(mut self, value: impl Into<Prop<String>>) -> Self {
    self.label = value.into();
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

  /// Selects one option by zero-based index.
  #[must_use]
  pub fn selected_index(mut self, value: impl Into<Prop<u32>>) -> Self {
    self.selected_index = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.label, Prop::Unset) {
      self.label.clone_from(&value.label);
    }
    if !matches!(value.choices, Prop::Unset) {
      self.choices.clone_from(&value.choices);
      parts::remove_indexed_outside(
        &mut self.parts,
        value.choices.set_value().map_or(0, Vec::len),
      );
    }
    if !matches!(value.selected_index, Prop::Unset) {
      self.selected_index = value.selected_index;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiRadioButtonGroup {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
