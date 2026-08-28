use serde::{Deserialize, Serialize};

use crate::{
  IconSource, LanguageDirection, PickingMode, Prop, Style, UsageHint, VisualElement,
  VisualElementProperties,
  elements::parts::{self, Part, PartStyle},
};

/// One labeled, optionally icon-bearing page inside a [`TabView`](crate::TabView).
///
/// A tab is a logical container for its page content. It may only be placed
/// directly beneath a tab view; other elements cannot be direct tab-view
/// children. [`Self::text`] and [`Self::icon`] form the header, while logical
/// children form the page shown when the tab is selected.
///
/// A closeable tab displays Unity's native close control, but Battlement treats
/// the gesture as a proposal. Subscribe on the parent [`TabView`] to
/// [`UiEventKind::TabCloseRequested`] and destroy this tab to accept it. Merely
/// clicking the close control never removes the tab automatically.
///
/// See Unity's [Tab manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Tab.html).
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Label, Tab, UiNode};
///
/// let inventory = UiNode::new(
///     ObjectId::new_v4(),
///     Tab::new("Inventory").closeable(true),
/// )
/// .child(UiNode::new(ObjectId::new_v4(), Label::new("No items")));
///
/// assert_eq!(inventory.children.len(), 1);
/// ```
///
/// [`TabView`]: crate::TabView
/// [`UiEventKind::TabCloseRequested`]: crate::UiEventKind::TabCloseRequested
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Tab {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: VisualElement,
  /// Text shown in the native tab header.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub text: Prop<String>,
  /// Prepared graphical asset shown in the native tab header.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub icon: Prop<IconSource>,
  /// Whether the native tab header displays a close control.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub closeable: Prop<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl Tab {
  /// Creates a tab with the supplied header text.
  #[must_use]
  pub fn new(text: impl Into<String>) -> Self {
    Self {
      text: Prop::Set(text.into()),
      ..Self::default()
    }
  }

  impl_common_visual_element_methods!();

  /// Replaces or resets the native tab-header text.
  #[must_use]
  pub fn text(mut self, value: impl Into<Prop<String>>) -> Self {
    self.text = value.into();
    self
  }

  /// Applies sparse inline declarations to the native `TabHeader` part.
  #[must_use]
  pub fn header_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabHeader, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabLabel` part.
  #[must_use]
  pub fn label_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabLabel, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabIcon` part.
  #[must_use]
  pub fn icon_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabIcon, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabUnderline` part.
  #[must_use]
  pub fn underline_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabUnderline, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabCloseButton` part.
  #[must_use]
  pub fn close_button_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabCloseButton, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabDragHandle` part.
  #[must_use]
  pub fn drag_handle_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabDragHandle, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabDragHandleLeadingBar` part.
  #[must_use]
  pub fn drag_handle_leading_bar_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabDragHandleLeadingBar, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabDragHandleTrailingBar` part.
  #[must_use]
  pub fn drag_handle_trailing_bar_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabDragHandleTrailingBar, value);
    self
  }

  /// Applies sparse inline declarations to the native `TabContentContainer` part.
  #[must_use]
  pub fn content_container_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TabContentContainer, value);
    self
  }

  /// Selects a prepared graphical asset for the native header icon.
  #[must_use]
  pub fn icon(mut self, value: impl Into<Prop<IconSource>>) -> Self {
    self.icon = value.into();
    self
  }

  /// Shows or hides the native close control.
  #[must_use]
  pub fn closeable(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.closeable = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.text, Prop::Unset) {
      self.text.clone_from(&value.text);
    }
    if !matches!(value.icon, Prop::Unset) {
      self.icon.clone_from(&value.icon);
    }
    if !matches!(value.closeable, Prop::Unset) {
      self.closeable = value.closeable;
    }
    if matches!(value.closeable, Prop::Set(false) | Prop::Reset) {
      parts::remove(&mut self.parts, &[Part::TabCloseButton]);
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl VisualElementProperties for Tab {
  fn visual_element(&self) -> &VisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut VisualElement {
    &mut self.element
  }
}
