use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
};

/// A Unity UI Toolkit text element for titles, captions, and descriptions.
///
/// A label renders its [`Self::text`] through Unity's text system. Text-related
/// inline styles such as [`Style::color`] and [`Style::font_size`] apply to the
/// rendered text, while ordinary layout styles control the label's box. A
/// Battlement label is a leaf and cannot contain logical [`UiNode`] children.
/// Use [`UiButton`] when the text should activate an action.
///
/// See Unity's [Label manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Label.html)
/// for native text behavior and styling.
///
/// # Example
///
/// ```
/// use battlement_types::{Color, ObjectId};
/// use battlement_ui::{UiLabel, Style, UiNode};
///
/// let title = UiNode::new(
///     ObjectId::new_v4(),
///     UiLabel::new("Mission ready").style(
///         Style::new().color(Color::rgb(0.8, 0.9, 1.0)).font_size(18.0),
///     ),
/// );
///
/// assert!(title.children.is_empty());
/// ```
///
/// [`UiButton`]: crate::UiButton
/// [`UiNode`]: crate::UiNode
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiLabel {
  /// Name, enabled state, USS classes, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Text rendered by the label's native Unity `TextElement`.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub text: Prop<String>,
  /// Whether Unity parses supported rich-text tags in the displayed string.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub enable_rich_text: Prop<bool>,
  /// Whether Unicode emoji prefer Unity's global emoji fallback list.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub emoji_fallback_support: Prop<bool>,
  /// Whether escape sequences such as `\\n` become control characters.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub parse_escape_sequences: Prop<bool>,
  /// Whether elided text exposes its complete value as a tooltip.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub display_tooltip_when_elided: Prop<bool>,
  /// Whether pointer and keyboard input may select rendered text.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub selectable: Prop<bool>,
  /// Whether a double click selects the word beneath the pointer.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub double_click_selects_word: Prop<bool>,
  /// Whether a triple click selects the complete rendered line.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub triple_click_selects_line: Prop<bool>,
  /// Whether focus selects the complete text value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub select_all_on_focus: Prop<bool>,
  /// Whether releasing the pointer selects the complete text value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub select_all_on_mouse_up: Prop<bool>,
}

impl UiLabel {
  /// Creates a leaf label displaying `text`.
  #[must_use]
  pub fn new(text: impl Into<String>) -> Self {
    Self {
      element: UiVisualElement::default(),
      text: Prop::Set(text.into()),
      ..Self::default()
    }
  }

  impl_common_visual_element_methods!();

  /// Replaces or resets the rendered text.
  #[must_use]
  pub fn text(mut self, value: impl Into<Prop<String>>) -> Self {
    self.text = value.into();
    self
  }

  /// Enables or disables Unity rich-text tag parsing.
  #[must_use]
  pub fn rich_text(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.enable_rich_text = value.into();
    self
  }

  /// Chooses whether emoji use Unity's emoji fallback list before ordinary fallbacks.
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

  /// Enables rendered-text selection and the associated selection preferences.
  #[must_use]
  pub fn selectable(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.selectable = value.into();
    self
  }

  /// Chooses whether double-clicking selects the word under the pointer.
  #[must_use]
  pub fn double_click_selects_word(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.double_click_selects_word = value.into();
    self
  }

  /// Chooses whether triple-clicking selects the rendered line under the pointer.
  #[must_use]
  pub fn triple_click_selects_line(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.triple_click_selects_line = value.into();
    self
  }

  /// Chooses whether receiving focus selects all text.
  #[must_use]
  pub fn select_all_on_focus(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.select_all_on_focus = value.into();
    self
  }

  /// Chooses whether releasing the pointer selects all text.
  #[must_use]
  pub fn select_all_on_mouse_up(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.select_all_on_mouse_up = value.into();
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
    if !value.selectable.is_unset() {
      self.selectable = value.selectable;
    }
    if !value.double_click_selects_word.is_unset() {
      self.double_click_selects_word = value.double_click_selects_word;
    }
    if !value.triple_click_selects_line.is_unset() {
      self.triple_click_selects_line = value.triple_click_selects_line;
    }
    if !value.select_all_on_focus.is_unset() {
      self.select_all_on_focus = value.select_all_on_focus;
    }
    if !value.select_all_on_mouse_up.is_unset() {
      self.select_all_on_mouse_up = value.select_all_on_mouse_up;
    }
  }
}

impl UiVisualElementProperties for UiLabel {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
