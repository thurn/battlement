use serde::{Deserialize, Serialize};

use crate::{
  IconSource, LanguageDirection, PickingMode, Prop, Style, UsageHint, VisualElement,
  VisualElementProperties,
  elements::parts::{self, Part, PartStyle},
};

/// A Unity UI Toolkit control that activates from a pointer or navigation submit.
///
/// Buttons are appropriate for discrete user commands such as confirming a
/// choice or opening another view. Calling [`Self::events`] with
/// [`UiEventKind::Click`] subscribes the Rust rules engine to activations;
/// constructing a button alone does not forward events.
///
/// Unity renders [`Self::text`] using the button's internal text element and
/// supplies the standard `.unity-button` appearance and interaction states.
/// Battlement models `Button` as a leaf, so additional logical [`UiNode`]
/// children are rejected. Use [`VisualElement`] or [`Box`] to compose content
/// that needs its own child hierarchy.
///
/// See Unity's [Button manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Button.html)
/// for native activation, content, and styling behavior.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Button, UiEventKind, UiNode};
///
/// let save = UiNode::new(
///     ObjectId::new_v4(),
///     Button::new("Save").name("save-button").events([UiEventKind::Click]),
/// );
///
/// assert!(save.children.is_empty());
/// ```
///
/// [`Box`]: crate::Box
/// [`UiEventKind::Click`]: crate::UiEventKind::Click
/// [`UiNode`]: crate::UiNode
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Button {
  /// Name, enabled state, USS classes, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: VisualElement,
  /// Text rendered inside the button's native Unity text element.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub text: Prop<String>,
  /// Whether supported rich-text tags are parsed.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub enable_rich_text: Prop<bool>,
  /// Whether emoji prefer the global emoji fallback list.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub emoji_fallback_support: Prop<bool>,
  /// Whether backslash escape sequences become control characters.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub parse_escape_sequences: Prop<bool>,
  /// Whether elided text exposes its complete value as a tooltip.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub display_tooltip_when_elided: Prop<bool>,
  /// Prepared asset displayed in Unity's native icon slot.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub icon: Prop<IconSource>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl Button {
  /// Creates a leaf button displaying `text` without an event subscription.
  #[must_use]
  pub fn new(text: impl Into<String>) -> Self {
    Self {
      element: VisualElement::default(),
      text: Prop::Set(text.into()),
      ..Self::default()
    }
  }

  impl_common_visual_element_methods!();

  /// Replaces or resets the button caption.
  #[must_use]
  pub fn text(mut self, value: impl Into<Prop<String>>) -> Self {
    self.text = value.into();
    self
  }

  /// Applies sparse inline declarations to the native `ButtonIcon` part.
  #[must_use]
  pub fn icon_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ButtonIcon, value);
    self
  }

  /// Enables or disables supported rich-text tag parsing.
  #[must_use]
  pub fn rich_text(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.enable_rich_text = value.into();
    self
  }
  /// Chooses whether emoji use Unity's emoji fallback list first.
  #[must_use]
  pub fn emoji_fallback(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.emoji_fallback_support = value.into();
    self
  }
  /// Chooses whether backslash escape sequences are interpreted.
  #[must_use]
  pub fn parse_escape_sequences(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.parse_escape_sequences = value.into();
    self
  }
  /// Shows the complete text in a tooltip when layout elides it.
  #[must_use]
  pub fn tooltip_when_elided(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.display_tooltip_when_elided = value.into();
    self
  }
  /// Selects a prepared graphical asset for Unity's native icon slot.
  #[must_use]
  pub fn icon(mut self, value: impl Into<Prop<IconSource>>) -> Self {
    self.icon = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.text, Prop::Unset) {
      self.text.clone_from(&value.text);
    }
    if !value.enable_rich_text.is_unset() {
      self.enable_rich_text = value.enable_rich_text;
    }
    if !value.emoji_fallback_support.is_unset() {
      self.emoji_fallback_support = value.emoji_fallback_support;
    }
    if !value.parse_escape_sequences.is_unset() {
      self.parse_escape_sequences = value.parse_escape_sequences;
    }
    if !value.display_tooltip_when_elided.is_unset() {
      self.display_tooltip_when_elided = value.display_tooltip_when_elided;
    }
    if !matches!(value.icon, Prop::Unset) {
      self.icon.clone_from(&value.icon);
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl VisualElementProperties for Button {
  fn visual_element(&self) -> &VisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut VisualElement {
    &mut self.element
  }
}
