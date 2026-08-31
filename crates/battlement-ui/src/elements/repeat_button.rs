use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
};

/// A leaf button that repeatedly activates while held.
///
/// Use a repeat button for commands that should continue while pressed, such as
/// incrementing a value or panning a view. Unity invokes the action once after
/// [`Self::delay_ms`], then again every [`Self::interval_ms`] until the pointer
/// or navigation submit is released. The initial delay is nonnegative and the
/// repeat interval is positive by type.
///
/// Subscribe to [`UiEventKind::Click`] to forward activations. Timed hold
/// callbacks are represented as [`ClickEvent::Repeat`]; keyboard or gamepad
/// submit produces one [`ClickEvent::NavigationSubmit`] instead. Rust does not
/// need to schedule its own timer. Like [`UiButton`], this control is a logical
/// leaf.
///
/// See Unity's [RepeatButton manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-RepeatButton.html)
/// for press-and-hold timing behavior.
///
/// # Example
///
/// ```
/// use std::num::NonZeroU32;
/// use battlement_ui::{Prop, UiRepeatButton, UiEventKind};
///
/// let increment = UiRepeatButton::new(
///     "Increase",
///     400,
///     NonZeroU32::new(75).unwrap(),
/// )
/// .events([UiEventKind::Click]);
///
/// assert_eq!(increment.delay_ms, Prop::Set(400));
/// ```
///
/// [`UiButton`]: crate::UiButton
/// [`ClickEvent::NavigationSubmit`]: crate::ClickEvent::NavigationSubmit
/// [`ClickEvent::Repeat`]: crate::ClickEvent::Repeat
/// [`UiEventKind::Click`]: crate::UiEventKind::Click
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiRepeatButton {
  /// Name, enabled state, USS classes, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Text rendered inside the button.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub text: Prop<String>,
  /// Delay before held activation starts repeating, in milliseconds.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub delay_ms: Prop<u32>,
  /// Time between held activations, in milliseconds.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub interval_ms: Prop<NonZeroU32>,
  /// Whether supported rich-text tags are parsed.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub enable_rich_text: Prop<bool>,
  /// Whether emoji prefer the global emoji fallback list.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub emoji_fallback_support: Prop<bool>,
  /// Whether backslash escape sequences become control characters.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub parse_escape_sequences: Prop<bool>,
  /// Whether elided text exposes its complete value as a tooltip.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub display_tooltip_when_elided: Prop<bool>,
}

impl UiRepeatButton {
  /// Creates a repeat button with complete required timing state.
  #[must_use]
  pub fn new(text: impl Into<String>, delay_ms: u32, interval_ms: NonZeroU32) -> Self {
    Self {
      text: Prop::Set(text.into()),
      delay_ms: Prop::Set(delay_ms),
      interval_ms: Prop::Set(interval_ms),
      ..Self::default()
    }
  }

  impl_common_visual_element_methods!();

  /// Replaces both repeat timing values atomically.
  #[must_use]
  pub fn timing(
    mut self,
    delay_ms: impl Into<Prop<u32>>,
    interval_ms: impl Into<Prop<NonZeroU32>>,
  ) -> Self {
    self.delay_ms = delay_ms.into();
    self.interval_ms = interval_ms.into();
    self
  }

  /// Replaces or resets the rendered caption.
  #[must_use]
  pub fn text(mut self, value: impl Into<Prop<String>>) -> Self {
    self.text = value.into();
    self
  }

  /// Enables or disables supported rich-text tag parsing.
  #[must_use]
  pub fn rich_text(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.enable_rich_text = value.into();
    self
  }
  /// Chooses whether emoji use Unity's emoji fallback list first.
  #[must_use]
  pub fn emoji_fallback(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.emoji_fallback_support = value.into();
    self
  }
  /// Chooses whether backslash escape sequences are interpreted.
  #[must_use]
  pub fn parse_escape_sequences(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.parse_escape_sequences = value.into();
    self
  }
  /// Shows the complete text in a tooltip when layout elides it.
  #[must_use]
  pub fn tooltip_when_elided(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.display_tooltip_when_elided = value.into();
    self
  }
  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !value.text.is_unset() {
      self.text.clone_from(&value.text);
    }
    if !value.delay_ms.is_unset() {
      self.delay_ms = value.delay_ms;
    }
    if !value.interval_ms.is_unset() {
      self.interval_ms = value.interval_ms;
    }
    if !value.enable_rich_text.is_unset() {
      self.enable_rich_text = value.enable_rich_text;
    }
    if !value.emoji_fallback_support.is_unset() {
      self.emoji_fallback_support = value.emoji_fallback_support;
    }
    if !value.parse_escape_sequences.is_unset() {
      self.parse_escape_sequences = value.parse_escape_sequences;
    }
    if !value.display_tooltip_when_elided.is_unset() {
      self.display_tooltip_when_elided = value.display_tooltip_when_elided;
    }
  }
}

impl UiVisualElementProperties for UiRepeatButton {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }
  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
