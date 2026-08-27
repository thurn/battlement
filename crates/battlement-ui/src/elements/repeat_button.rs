use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
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
/// need to schedule its own timer. Like [`Button`], this control is a logical
/// leaf.
///
/// See Unity's [RepeatButton manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-RepeatButton.html)
/// for press-and-hold timing behavior.
///
/// # Example
///
/// ```
/// use std::num::NonZeroU32;
/// use battlement_ui::{RepeatButton, UiEventKind};
///
/// let increment = RepeatButton::new(
///     "Increase",
///     400,
///     NonZeroU32::new(75).unwrap(),
/// )
/// .events([UiEventKind::Click]);
///
/// assert_eq!(increment.delay_ms, Some(400));
/// ```
///
/// [`Button`]: crate::Button
/// [`ClickEvent::NavigationSubmit`]: crate::ClickEvent::NavigationSubmit
/// [`ClickEvent::Repeat`]: crate::ClickEvent::Repeat
/// [`UiEventKind::Click`]: crate::UiEventKind::Click
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct RepeatButton {
    /// Name, enabled state, USS classes, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Text rendered inside the button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Delay before held activation starts repeating, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u32>,
    /// Time between held activations, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interval_ms: Option<NonZeroU32>,
    /// Whether supported rich-text tags are parsed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_rich_text: Option<bool>,
    /// Whether emoji prefer the global emoji fallback list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji_fallback_support: Option<bool>,
    /// Whether backslash escape sequences become control characters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_escape_sequences: Option<bool>,
    /// Whether elided text exposes its complete value as a tooltip.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_tooltip_when_elided: Option<bool>,
}

impl RepeatButton {
    /// Creates a repeat button with complete required timing state.
    #[must_use]
    pub fn new(text: impl Into<String>, delay_ms: u32, interval_ms: NonZeroU32) -> Self {
        Self {
            text: Some(text.into()),
            delay_ms: Some(delay_ms),
            interval_ms: Some(interval_ms),
            ..Self::default()
        }
    }

    impl_common_visual_element_methods!();

    /// Replaces both repeat timing values atomically.
    #[must_use]
    pub fn timing(mut self, delay_ms: u32, interval_ms: NonZeroU32) -> Self {
        self.delay_ms = Some(delay_ms);
        self.interval_ms = Some(interval_ms);
        self
    }

    /// Enables or disables supported rich-text tag parsing.
    #[must_use]
    pub fn rich_text(mut self, value: bool) -> Self {
        self.enable_rich_text = Some(value);
        self
    }
    /// Chooses whether emoji use Unity's emoji fallback list first.
    #[must_use]
    pub fn emoji_fallback(mut self, value: bool) -> Self {
        self.emoji_fallback_support = Some(value);
        self
    }
    /// Chooses whether backslash escape sequences are interpreted.
    #[must_use]
    pub fn parse_escape_sequences(mut self, value: bool) -> Self {
        self.parse_escape_sequences = Some(value);
        self
    }
    /// Shows the complete text in a tooltip when layout elides it.
    #[must_use]
    pub fn tooltip_when_elided(mut self, value: bool) -> Self {
        self.display_tooltip_when_elided = Some(value);
        self
    }
    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.text.is_some() {
            self.text.clone_from(&value.text);
        }
        if value.delay_ms.is_some() {
            self.delay_ms = value.delay_ms;
        }
        if value.interval_ms.is_some() {
            self.interval_ms = value.interval_ms;
        }
        if value.enable_rich_text.is_some() {
            self.enable_rich_text = value.enable_rich_text;
        }
        if value.emoji_fallback_support.is_some() {
            self.emoji_fallback_support = value.emoji_fallback_support;
        }
        if value.parse_escape_sequences.is_some() {
            self.parse_escape_sequences = value.parse_escape_sequences;
        }
        if value.display_tooltip_when_elided.is_some() {
            self.display_tooltip_when_elided = value.display_tooltip_when_elided;
        }
    }
}

impl VisualElementProperties for RepeatButton {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }
    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
