use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// A controlled workspace whose direct children are [`Tab`](crate::Tab) pages.
///
/// Selection and reorder gestures are proposals. Unity restores the latest
/// Rust-authored state until a response updates the selected index or logical
/// child order. Native close gestures are always vetoed; accepting a close
/// requires destroying the requested tab in the response.
///
/// See Unity's [TabView API](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.TabView.html).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TabView {
    /// Shared visual properties, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Zero-based index of the Rust-authored active tab.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_tab_index: Option<u32>,
    /// Whether users may propose a different tab order by dragging headers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reorderable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl TabView {
    /// Creates a tab view using Unity's default selection and ordering behavior.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    parts::part_style_builders!(
        content_viewport_style => TabViewContentViewport,
        header_container_style => TabViewHeaderContainer,
        content_container_style => TabViewContentContainer,
        previous_button_style => TabViewPreviousButton,
        next_button_style => TabViewNextButton,
    );

    /// Sets the zero-based Rust-authored active-tab index.
    #[must_use]
    pub fn selected_tab_index(mut self, value: u32) -> Self {
        self.selected_tab_index = Some(value);
        self
    }

    /// Enables or disables native tab-header dragging.
    #[must_use]
    pub fn reorderable(mut self, value: bool) -> Self {
        self.reorderable = Some(value);
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.selected_tab_index.is_some() {
            self.selected_tab_index = value.selected_tab_index;
        }
        if value.reorderable.is_some() {
            self.reorderable = value.reorderable;
        }
        parts::merge(&mut self.parts, &value.parts);
    }
}

impl VisualElementProperties for TabView {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
