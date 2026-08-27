use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// A controlled single-choice field that keeps every option visible.
///
/// Use a radio group when choices are mutually exclusive and users should be
/// able to compare them without opening a popup. Prefer [`DropdownField`] when
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
/// use battlement_ui::{RadioButtonGroup, UiEventKind};
///
/// let quality = RadioButtonGroup::new()
///     .label("Quality")
///     .choices(["Low", "Medium", "High"])
///     .selected_index(2)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(quality.selected_index, Some(2));
/// ```
///
/// [`DropdownField`]: crate::DropdownField
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
/// [`UiNode`]: crate::UiNode
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl RadioButtonGroup {
    /// Creates an empty radio group with no selection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    parts::part_style_builders!(
        label_style => RadioButtonGroupLabel,
        input_style => RadioButtonGroupInput,
        choices_container_style => RadioButtonGroupChoicesContainer,
        content_container_style => RadioButtonGroupContentContainer,
        all_options_style => RadioButtonGroupAllOptions,
    );

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
            parts::remove_indexed_outside(
                &mut self.parts,
                value.choices.as_ref().map_or(0, Vec::len),
            );
        }
        if value.selected_index.is_some() {
            self.selected_index = value.selected_index;
        }
        parts::merge(&mut self.parts, &value.parts);
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
