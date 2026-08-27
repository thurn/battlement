use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// A controlled group that presents logical [`Button`] children as toggles.
///
/// Use this control when each choice benefits from a button-like label or icon.
/// By default the group selects one button and does not allow an empty
/// selection. [`Self::multiple_selection`] permits several selected buttons;
/// [`Self::allow_empty_selection`] permits none. [`Self::selected_indices`]
/// addresses direct children by their zero-based visual order.
///
/// Selection gestures produce [`UiEventKind::ValueCommitted`] proposals. Rust
/// remains authoritative until an update sends the accepted indices. Selected
/// indices must be unique, sorted, and within the direct-child list. Only
/// ordinary [`Button`] nodes are valid logical children.
///
/// See Unity's [ToggleButtonGroup manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-ToggleButtonGroup.html)
/// for single, multiple, and empty-selection behavior.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Button, ToggleButtonGroup, UiEventKind, UiNode};
///
/// let alignment = UiNode::new(
///     ObjectId::new_v4(),
///     ToggleButtonGroup::new()
///         .label("Alignment")
///         .selected_indices([0])
///         .events([UiEventKind::ValueCommitted]),
/// )
/// .children([
///     UiNode::new(ObjectId::new_v4(), Button::new("Left")),
///     UiNode::new(ObjectId::new_v4(), Button::new("Center")),
///     UiNode::new(ObjectId::new_v4(), Button::new("Right")),
/// ]);
///
/// assert_eq!(alignment.children.len(), 3);
/// ```
///
/// [`Button`]: crate::Button
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ToggleButtonGroup {
    /// Shared visual properties, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Caption associated with the complete field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Whether more than one button may be selected.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiple_selection: Option<bool>,
    /// Whether a nonempty group may have no selected button.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_empty_selection: Option<bool>,
    /// Unique sorted zero-based indices authored as selected by Rust.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_indices: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl ToggleButtonGroup {
    /// Creates a single-selection group using its first child by default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    parts::part_style_builders!(
        label_style => ToggleButtonGroupLabel,
        input_style => ToggleButtonGroupInput,
    );

    /// Sets the field caption.
    #[must_use]
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Enables or disables multiple simultaneous selections.
    #[must_use]
    pub fn multiple_selection(mut self, value: bool) -> Self {
        self.multiple_selection = Some(value);
        self
    }

    /// Enables or disables an empty selection in a nonempty group.
    #[must_use]
    pub fn allow_empty_selection(mut self, value: bool) -> Self {
        self.allow_empty_selection = Some(value);
        self
    }

    /// Replaces the unique sorted selected indices.
    #[must_use]
    pub fn selected_indices(mut self, values: impl IntoIterator<Item = u32>) -> Self {
        self.selected_indices = Some(values.into_iter().collect());
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.label.is_some() {
            self.label.clone_from(&value.label);
        }
        if value.multiple_selection.is_some() {
            self.multiple_selection = value.multiple_selection;
        }
        if value.allow_empty_selection.is_some() {
            self.allow_empty_selection = value.allow_empty_selection;
        }
        if value.selected_indices.is_some() {
            self.selected_indices.clone_from(&value.selected_indices);
        }
        parts::merge(&mut self.parts, &value.parts);
    }
}

impl VisualElementProperties for ToggleButtonGroup {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
