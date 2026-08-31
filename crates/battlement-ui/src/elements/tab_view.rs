use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled workspace whose direct children are [`UiTab`](crate::UiTab) pages.
///
/// Selection and reorder gestures are proposals. Unity restores the latest
/// Rust-authored state until a response updates the selected index or logical
/// child order. Native close gestures are always vetoed; accepting a close
/// requires destroying the requested tab in the response.
///
/// Only [`UiTab`] nodes are valid direct children. Each tab's logical children are
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
/// use battlement_ui::{UiTab, UiTabView, UiEventKind, UiNode};
///
/// let workspace = UiNode::new(
///     ObjectId::new_v4(),
///     UiTabView::new()
///         .selected_tab_index(0)
///         .reorderable(true)
///         .events([
///             UiEventKind::TabSelectionRequested,
///             UiEventKind::TabReorderRequested,
///         ]),
/// )
/// .children([
///     UiNode::new(ObjectId::new_v4(), UiTab::new("Map")),
///     UiNode::new(ObjectId::new_v4(), UiTab::new("Journal")),
/// ]);
///
/// assert_eq!(workspace.children.len(), 2);
/// ```
///
/// [`UiTab`]: crate::UiTab
/// [`UiEventKind::TabSelectionRequested`]: crate::UiEventKind::TabSelectionRequested
/// [`UiEventKind::TabCloseRequested`]: crate::UiEventKind::TabCloseRequested
/// [`UiEventKind::TabReorderRequested`]: crate::UiEventKind::TabReorderRequested
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiTabView {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Zero-based index of the Rust-authored active tab.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub selected_tab_index: Prop<u32>,
  /// Whether users may propose a different tab order by dragging headers.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub reorderable: Prop<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiTabView {
  /// Creates a tab view using Unity's default selection and ordering behavior.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `TabViewContentViewport` part.
  #[must_use]
  pub fn content_viewport_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabViewContentViewport, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabViewHeaderContainer` part.
  #[must_use]
  pub fn header_container_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabViewHeaderContainer, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabViewContentContainer` part.
  #[must_use]
  pub fn content_container_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabViewContentContainer, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabViewPreviousButton` part.
  #[must_use]
  pub fn previous_button_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabViewPreviousButton, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabViewNextButton` part.
  #[must_use]
  pub fn next_button_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabViewNextButton, value);
    self
  }

  /// Sets the zero-based Rust-authored active-tab index.
  #[must_use]
  pub fn selected_tab_index(mut self, value: impl Into<Prop<u32>>) -> Self {
    self.selected_tab_index = value.into();
    self
  }

  /// Enables or disables native tab-header dragging.
  #[must_use]
  pub fn reorderable(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.reorderable = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.selected_tab_index, Prop::Unset) {
      self.selected_tab_index = value.selected_tab_index;
    }
    if !matches!(value.reorderable, Prop::Unset) {
      self.reorderable = value.reorderable;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiTabView {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
