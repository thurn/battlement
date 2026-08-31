use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled Boolean field rendered as a native checkbox-style toggle.
///
/// Use a toggle for an independent on/off setting. [`Self::label`] captions the
/// complete field, while [`Self::text`] appears beside the checkmark as the
/// option's visible text. For mutually exclusive choices use
/// [`UiRadioButtonGroup`]; for a row of selectable action buttons use
/// [`UiToggleButtonGroup`].
///
/// Pointer, keyboard, and navigation-submit interaction proposes a new Boolean
/// through [`UiEventKind::ValueCommitted`]. Rust remains authoritative, so the
/// native value returns to [`Self::value`] until an update accepts the proposal.
/// The control is a logical leaf and exposes named builders for styling its
/// label, input, checkmark, and text.
///
/// See Unity's [Toggle manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Toggle.html)
/// for native interaction and styling behavior.
///
/// # Example
///
/// ```
/// use battlement_ui::{Prop, UiToggle, UiEventKind};
///
/// let subtitles = UiToggle::new()
///     .label("Accessibility")
///     .text("Show subtitles")
///     .value(true)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(subtitles.value, Prop::Set(true));
/// ```
///
/// [`UiRadioButtonGroup`]: crate::UiRadioButtonGroup
/// [`UiToggleButtonGroup`]: crate::UiToggleButtonGroup
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiToggle {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Caption associated with the complete field.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub label: Prop<String>,
  /// Text displayed beside the native checkmark.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub text: Prop<String>,
  /// Latest Boolean value authored by Rust.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub value: Prop<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiToggle {
  /// Creates an empty controlled toggle.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `ToggleLabel` part.
  #[must_use]
  pub fn label_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ToggleLabel, value);
    self
  }

  /// Applies sparse inline declarations to the native `ToggleInput` part.
  #[must_use]
  pub fn input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ToggleInput, value);
    self
  }

  /// Applies sparse inline declarations to the native `ToggleCheckmark` part.
  #[must_use]
  pub fn checkmark_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ToggleCheckmark, value);
    self
  }

  /// Applies sparse inline declarations to the native `ToggleText` part.
  #[must_use]
  pub fn text_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ToggleText, value);
    self
  }

  /// Sets the field caption.
  #[must_use]
  pub fn label(mut self, value: impl Into<Prop<String>>) -> Self {
    self.label = value.into();
    self
  }

  /// Sets the option text.
  #[must_use]
  pub fn text(mut self, value: impl Into<Prop<String>>) -> Self {
    self.text = value.into();
    self
  }

  /// Sets the Rust-authored value.
  #[must_use]
  pub fn value(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.value = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.label, Prop::Unset) {
      self.label.clone_from(&value.label);
    }
    if !matches!(value.text, Prop::Unset) {
      self.text.clone_from(&value.text);
    }
    if !matches!(value.value, Prop::Unset) {
      self.value = value.value;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiToggle {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
