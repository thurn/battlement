use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// A controlled standalone Boolean radio option.
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

    parts::part_style_builders!(
        label_style => RadioButtonLabel,
        input_style => RadioButtonInput,
        checkmark_background_style => RadioButtonCheckmarkBackground,
        checkmark_style => RadioButtonCheckmark,
        text_style => RadioButtonText,
    );

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
