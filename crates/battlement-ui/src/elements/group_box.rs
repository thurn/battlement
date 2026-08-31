use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A Unity UI Toolkit container that groups related controls under an optional title.
///
/// Children are inserted through Unity's public content API. An empty title
/// keeps the native title label absent, while a nonempty title creates it.
/// Group boxes are useful for expressing a relationship between fields without
/// imposing [`Box`]'s themed border and background. They also establish the
/// native scope used by standalone radio buttons.
///
/// See Unity's [GroupBox manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-GroupBox.html)
/// for title and child-container behavior.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{UiGroupBox, UiToggle, UiNode};
///
/// let accessibility = UiNode::new(
///     ObjectId::new_v4(),
///     UiGroupBox::new().text("Accessibility"),
/// )
/// .child(UiNode::new(
///     ObjectId::new_v4(),
///     UiToggle::new().text("Show subtitles").value(true),
/// ));
///
/// assert_eq!(accessibility.children.len(), 1);
/// ```
///
/// [`UiBox`]: crate::UiBox
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiGroupBox {
  /// Name, enabled state, USS classes, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Text rendered by the native group title label.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub text: Prop<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiGroupBox {
  /// Creates an untitled group container.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `GroupBoxTitle` part.
  #[must_use]
  pub fn title_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::GroupBoxTitle, value);
    self
  }

  /// Sets the optional group title; an empty value removes the native title label.
  #[must_use]
  pub fn text(mut self, value: impl Into<Prop<String>>) -> Self {
    self.text = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.text, Prop::Unset) {
      self.text.clone_from(&value.text);
    }
    if matches!(&value.text, Prop::Set(text) if text.is_empty())
      || matches!(value.text, Prop::Reset)
    {
      parts::remove(&mut self.parts, &[Part::GroupBoxTitle]);
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiGroupBox {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
