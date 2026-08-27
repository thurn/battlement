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
/// Only [`Tab`] nodes are valid direct children. Each tab's logical children are
/// its page content. When headers overflow the available width, Unity exposes
/// previous and next controls; their appearance can be changed with the named
/// part-style builders.
///
/// Subscribe to [`UiEventKind::TabSelectionRequested`],
/// [`UiEventKind::TabCloseRequested`], or
/// [`UiEventKind::TabReorderRequested`] for the corresponding proposals.
/// Accept selection by updating [`Self::selected_tab_index`], close by destroying
/// the requested tab, and reorder by changing the logical child order.
///
/// See Unity's [TabView manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-TabView.html)
/// and [scripting API](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.TabView.html).
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Tab, TabView, UiEventKind, UiNode};
///
/// let workspace = UiNode::new(
///     ObjectId::new_v4(),
///     TabView::new()
///         .selected_tab_index(0)
///         .reorderable(true)
///         .events([
///             UiEventKind::TabSelectionRequested,
///             UiEventKind::TabReorderRequested,
///         ]),
/// )
/// .children([
///     UiNode::new(ObjectId::new_v4(), Tab::new("Map")),
///     UiNode::new(ObjectId::new_v4(), Tab::new("Journal")),
/// ]);
///
/// assert_eq!(workspace.children.len(), 2);
/// ```
///
/// [`Tab`]: crate::Tab
/// [`UiEventKind::TabSelectionRequested`]: crate::UiEventKind::TabSelectionRequested
/// [`UiEventKind::TabCloseRequested`]: crate::UiEventKind::TabCloseRequested
/// [`UiEventKind::TabReorderRequested`]: crate::UiEventKind::TabReorderRequested
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
