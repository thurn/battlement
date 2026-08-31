use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled Boolean option rendered with Unity's radio-button appearance.
///
/// Radio buttons are mutually exclusive within their Unity group. The nearest
/// ancestor [`UiGroupBox`] defines that group; without one, the complete panel is
/// the default group. Prefer [`UiRadioButtonGroup`] when the options should behave
/// as one indexed field. Use separate radio buttons inside a group box when the
/// group also needs other kinds of visual content. [`Self::label`] captions the
/// complete field, while [`Self::text`] appears beside the radio mark.
///
/// User activation proposes a value through
/// [`UiEventKind::ValueCommitted`]. Rust remains authoritative: the native
/// control returns to the latest [`Self::value`] until an update accepts the
/// proposal. This control is a logical leaf.
///
/// See Unity's [RadioButton manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-RadioButton.html)
/// for native focus, input, and styling behavior.
///
/// # Example
///
/// ```
/// use battlement_ui::{Prop, UiRadioButton, UiEventKind};
///
/// let compact = UiRadioButton::new()
///     .label("Layout")
///     .text("Compact")
///     .value(true)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(compact.value, Prop::Set(true));
/// ```
///
/// [`UiRadioButtonGroup`]: crate::UiRadioButtonGroup
/// [`UiGroupBox`]: crate::UiGroupBox
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiRadioButton {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Caption associated with the complete field.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub label: Prop<String>,
  /// Text displayed beside the native radio mark.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub text: Prop<String>,
  /// Latest Boolean value authored by Rust.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub value: Prop<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiRadioButton {
  /// Creates an empty controlled standalone radio button.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `RadioButtonLabel` part.
  #[must_use]
  pub fn label_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::RadioButtonLabel, value);
    self
  }

  /// Applies sparse inline declarations to the native `RadioButtonInput` part.
  #[must_use]
  pub fn input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::RadioButtonInput, value);
    self
  }

  /// Applies sparse inline declarations to the native `RadioButtonCheckmarkBackground` part.
  #[must_use]
  pub fn checkmark_background_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::RadioButtonCheckmarkBackground, value);
    self
  }

  /// Applies sparse inline declarations to the native `RadioButtonCheckmark` part.
  #[must_use]
  pub fn checkmark_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::RadioButtonCheckmark, value);
    self
  }

  /// Applies sparse inline declarations to the native `RadioButtonText` part.
  #[must_use]
  pub fn text_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::RadioButtonText, value);
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

impl UiVisualElementProperties for UiRadioButton {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
