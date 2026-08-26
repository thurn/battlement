use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
};

/// A controlled exclusive choice presented as native radio options.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct RadioButtonGroup {
    /// Shared visual properties, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Caption associated with the complete field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Ordered display-ready option labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<String>>,
    /// Zero-based Rust-authored option index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_index: Option<u32>,
}

impl RadioButtonGroup {
    /// Creates an empty radio group with no selection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    /// Sets the field caption.
    #[must_use]
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Replaces the ordered option labels.
    #[must_use]
    pub fn choices(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.choices = Some(values.into_iter().map(Into::into).collect());
        self
    }

    /// Selects one option by zero-based index.
    #[must_use]
    pub fn selected_index(mut self, value: u32) -> Self {
        self.selected_index = Some(value);
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.label.is_some() {
            self.label.clone_from(&value.label);
        }
        if value.choices.is_some() {
            self.choices.clone_from(&value.choices);
        }
        if value.selected_index.is_some() {
            self.selected_index = value.selected_index;
        }
    }
}

impl VisualElementProperties for RadioButtonGroup {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
