use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A Unity UI Toolkit popup-styled text container with a public content container.
///
/// This element provides the visual structure of a popup card without
/// positioning, modality, menus, or lifecycle behavior. Logical children are
/// inserted through Unity's public `contentContainer` route.
///
/// Use `UiPopupWindow` when application logic already owns when and where a popup
/// appears and only needs Unity's popup styling and hierarchy. It does not open
/// itself, capture focus, dismiss on outside clicks, or choose screen placement.
/// Apply positioning through [`Style`] and create or destroy the corresponding
/// [`UiNode`] as the application opens or closes the popup.
///
/// The inherited text properties can provide a heading or selectable message;
/// logical children hold richer popup content. The content-container part can be
/// styled separately from the outer popup.
///
/// See Unity's [PopupWindow manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-PopupWindow.html)
/// for its native content container and USS identity.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{UiButton, UiPopupWindow, Style, UiNode};
///
/// let popup = UiNode::new(
///     ObjectId::new_v4(),
///     UiPopupWindow::new()
///         .text("Connection lost")
///         .style(Style::new().padding(16.0)),
/// )
/// .child(UiNode::new(ObjectId::new_v4(), UiButton::new("Retry")));
///
/// assert_eq!(popup.children.len(), 1);
/// ```
///
/// [`UiNode`]: crate::UiNode
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiPopupWindow {
  /// Name, enabled state, USS classes, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Text rendered by the popup's inherited native text element.
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
  /// Whether rendered text may be selected.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub selectable: Prop<bool>,
  /// Whether a double click selects a word.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub double_click_selects_word: Prop<bool>,
  /// Whether a triple click selects a rendered line.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub triple_click_selects_line: Prop<bool>,
  /// Whether focus selects the complete text.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub select_all_on_focus: Prop<bool>,
  /// Whether pointer release selects the complete text.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub select_all_on_mouse_up: Prop<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiPopupWindow {
  /// Creates an empty popup-styled content container.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `PopupWindowContentContainer` part.
  #[must_use]
  pub fn content_container_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::PopupWindowContentContainer, value);
    self
  }

  /// Sets the popup heading text.
  #[must_use]
  pub fn text(mut self, value: impl Into<Prop<String>>) -> Self {
    self.text = value.into();
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

  /// Enables rendered-text selection.
  #[must_use]
  pub fn selectable(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.selectable = value.into();
    self
  }

  /// Chooses whether double-clicking selects a word.
  #[must_use]
  pub fn double_click_selects_word(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.double_click_selects_word = value.into();
    self
  }

  /// Chooses whether triple-clicking selects a line.
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
    if !matches!(value.enable_rich_text, Prop::Unset) {
      self.enable_rich_text = value.enable_rich_text;
    }
    if !matches!(value.emoji_fallback_support, Prop::Unset) {
      self.emoji_fallback_support = value.emoji_fallback_support;
    }
    if !matches!(value.parse_escape_sequences, Prop::Unset) {
      self.parse_escape_sequences = value.parse_escape_sequences;
    }
    if !matches!(value.display_tooltip_when_elided, Prop::Unset) {
      self.display_tooltip_when_elided = value.display_tooltip_when_elided;
    }
    if !matches!(value.selectable, Prop::Unset) {
      self.selectable = value.selectable;
    }
    if !matches!(value.double_click_selects_word, Prop::Unset) {
      self.double_click_selects_word = value.double_click_selects_word;
    }
    if !matches!(value.triple_click_selects_line, Prop::Unset) {
      self.triple_click_selects_line = value.triple_click_selects_line;
    }
    if !matches!(value.select_all_on_focus, Prop::Unset) {
      self.select_all_on_focus = value.select_all_on_focus;
    }
    if !matches!(value.select_all_on_mouse_up, Prop::Unset) {
      self.select_all_on_mouse_up = value.select_all_on_mouse_up;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiPopupWindow {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
