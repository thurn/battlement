use serde::{Deserialize, Serialize};

use crate::{
    Choice, LanguageDirection, PickingMode, Style, UsageHint, VisualElement,
    VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// A controlled single-choice popup selector.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct DropdownField {
    /// Shared visual properties, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Caption associated with the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Whether the native field displays its mixed-value state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_mixed_value: Option<bool>,
    /// Ordered display-ready option labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<String>>,
    /// Sparse authored selection. An empty choice explicitly clears the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<Choice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl DropdownField {
    /// Creates an empty dropdown with no selection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    parts::part_style_builders!(
        label_style => DropdownFieldLabel,
        input_style => DropdownFieldInput,
        text_style => DropdownFieldText,
        arrow_style => DropdownFieldArrow,
    );

    /// Sets the field caption.
    #[must_use]
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Enables or disables the native mixed-value presentation.
    #[must_use]
    pub fn show_mixed_value(mut self, value: bool) -> Self {
        self.show_mixed_value = Some(value);
        self
    }

    /// Replaces the ordered option labels.
    #[must_use]
    pub fn choices(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.choices = Some(values.into_iter().map(Into::into).collect());
        self
    }

    /// Selects one option using matching index and display value.
    #[must_use]
    pub fn selection(mut self, index: u32, value: impl Into<String>) -> Self {
        self.selection = Some(Choice::selected(index, value));
        self
    }

    /// Replaces the sparse authored selection with a complete coherent pair.
    #[must_use]
    pub fn selection_value(mut self, value: Choice) -> Self {
        self.selection = Some(value);
        self
    }

    /// Explicitly clears the selected option in a sparse update.
    #[must_use]
    pub fn clear_selection(mut self) -> Self {
        self.selection = Some(Choice::none());
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.label.is_some() {
            self.label.clone_from(&value.label);
        }
        if value.show_mixed_value.is_some() {
            self.show_mixed_value = value.show_mixed_value;
        }
        if value.choices.is_some() {
            self.choices.clone_from(&value.choices);
        }
        if value.selection.is_some() {
            self.selection.clone_from(&value.selection);
        }
        parts::merge(&mut self.parts, &value.parts);
    }
}

impl VisualElementProperties for DropdownField {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
