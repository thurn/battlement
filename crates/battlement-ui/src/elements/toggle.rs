use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled Boolean field rendered as a native checkbox-style toggle.
///
/// Use a toggle for an independent on/off setting. [`Self::label`] captions the
/// complete field, while [`Self::text`] appears beside the checkmark as the
/// option's visible text. For mutually exclusive choices use
/// [`RadioButtonGroup`]; for a row of selectable action buttons use
/// [`ToggleButtonGroup`].
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
/// use battlement_ui::{Toggle, UiEventKind};
///
/// let subtitles = Toggle::new()
///     .label("Accessibility")
///     .text("Show subtitles")
///     .value(true)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(subtitles.value, Some(true));
/// ```
///
/// [`RadioButtonGroup`]: crate::RadioButtonGroup
/// [`ToggleButtonGroup`]: crate::ToggleButtonGroup
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Toggle {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: VisualElement,
  /// Caption associated with the complete field.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  /// Text displayed beside the native checkmark.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
  /// Latest Boolean value authored by Rust.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub value: Option<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl Toggle {
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
  pub fn label(mut self, value: impl Into<String>) -> Self {
    self.label = Some(value.into());
    self
  }

  /// Sets the option text.
  #[must_use]
  pub fn text(mut self, value: impl Into<String>) -> Self {
    self.text = Some(value.into());
    self
  }

  /// Sets the Rust-authored value.
  #[must_use]
  pub fn value(mut self, value: bool) -> Self {
    self.value = Some(value);
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if value.label.is_some() {
      self.label.clone_from(&value.label);
    }
    if value.text.is_some() {
      self.text.clone_from(&value.text);
    }
    if value.value.is_some() {
      self.value = value.value;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl VisualElementProperties for Toggle {
  fn visual_element(&self) -> &VisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut VisualElement {
    &mut self.element
  }
}
