use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// A controlled Boolean option rendered with Unity's radio-button appearance.
///
/// Radio buttons are mutually exclusive within their Unity group. The nearest
/// ancestor [`GroupBox`] defines that group; without one, the complete panel is
/// the default group. Prefer [`RadioButtonGroup`] when the options should behave
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
/// use battlement_ui::{RadioButton, UiEventKind};
///
/// let compact = RadioButton::new()
///     .label("Layout")
///     .text("Compact")
///     .value(true)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(compact.value, Some(true));
/// ```
///
/// [`RadioButtonGroup`]: crate::RadioButtonGroup
/// [`GroupBox`]: crate::GroupBox
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct RadioButton {
    /// Shared visual properties, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Caption associated with the complete field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Text displayed beside the native radio mark.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Latest Boolean value authored by Rust.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl RadioButton {
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

impl VisualElementProperties for RadioButton {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
