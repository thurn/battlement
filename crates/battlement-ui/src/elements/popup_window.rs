use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
  elements::parts::{self, Part, PartStyle},
};

/// A Unity UI Toolkit popup-styled text container with a public content container.
///
/// This element provides the visual structure of a popup card without
/// positioning, modality, menus, or lifecycle behavior. Logical children are
/// inserted through Unity's public `contentContainer` route.
///
/// Use `PopupWindow` when application logic already owns when and where a popup
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
/// use battlement_ui::{Button, PopupWindow, Style, UiNode};
///
/// let popup = UiNode::new(
///     ObjectId::new_v4(),
///     PopupWindow::new()
///         .text("Connection lost")
///         .style(Style::new().padding(16.0)),
/// )
/// .child(UiNode::new(ObjectId::new_v4(), Button::new("Retry")));
///
/// assert_eq!(popup.children.len(), 1);
/// ```
///
/// [`UiNode`]: crate::UiNode
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PopupWindow {
  /// Name, enabled state, USS classes, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: VisualElement,
  /// Text rendered by the popup's inherited native text element.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub text: Option<String>,
  /// Whether supported rich-text tags are parsed.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub enable_rich_text: Option<bool>,
  /// Whether emoji prefer the global emoji fallback list.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub emoji_fallback_support: Option<bool>,
  /// Whether backslash escape sequences become control characters.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub parse_escape_sequences: Option<bool>,
  /// Whether elided text exposes its complete value as a tooltip.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub display_tooltip_when_elided: Option<bool>,
  /// Whether rendered text may be selected.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub selectable: Option<bool>,
  /// Whether a double click selects a word.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub double_click_selects_word: Option<bool>,
  /// Whether a triple click selects a rendered line.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub triple_click_selects_line: Option<bool>,
  /// Whether focus selects the complete text.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub select_all_on_focus: Option<bool>,
  /// Whether pointer release selects the complete text.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub select_all_on_mouse_up: Option<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl PopupWindow {
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
  pub fn text(mut self, value: impl Into<String>) -> Self {
    self.text = Some(value.into());
    self
  }

  /// Enables or disables supported rich-text tag parsing.
  #[must_use]
  pub fn rich_text(mut self, value: bool) -> Self {
    self.enable_rich_text = Some(value);
    self
  }

  /// Chooses whether emoji use Unity's emoji fallback list first.
  #[must_use]
  pub fn emoji_fallback(mut self, value: bool) -> Self {
    self.emoji_fallback_support = Some(value);
    self
  }

  /// Chooses whether backslash escape sequences are interpreted.
  #[must_use]
  pub fn parse_escape_sequences(mut self, value: bool) -> Self {
    self.parse_escape_sequences = Some(value);
    self
  }

  /// Shows the complete text in a tooltip when layout elides it.
  #[must_use]
  pub fn tooltip_when_elided(mut self, value: bool) -> Self {
    self.display_tooltip_when_elided = Some(value);
    self
  }

  /// Enables rendered-text selection.
  #[must_use]
  pub fn selectable(mut self, value: bool) -> Self {
    self.selectable = Some(value);
    self
  }

  /// Chooses whether double-clicking selects a word.
  #[must_use]
  pub fn double_click_selects_word(mut self, value: bool) -> Self {
    self.double_click_selects_word = Some(value);
    self
  }

  /// Chooses whether triple-clicking selects a line.
  #[must_use]
  pub fn triple_click_selects_line(mut self, value: bool) -> Self {
    self.triple_click_selects_line = Some(value);
    self
  }

  /// Chooses whether receiving focus selects all text.
  #[must_use]
  pub fn select_all_on_focus(mut self, value: bool) -> Self {
    self.select_all_on_focus = Some(value);
    self
  }

  /// Chooses whether releasing the pointer selects all text.
  #[must_use]
  pub fn select_all_on_mouse_up(mut self, value: bool) -> Self {
    self.select_all_on_mouse_up = Some(value);
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if value.text.is_some() {
      self.text.clone_from(&value.text);
    }
    if value.enable_rich_text.is_some() {
      self.enable_rich_text = value.enable_rich_text;
    }
    if value.emoji_fallback_support.is_some() {
      self.emoji_fallback_support = value.emoji_fallback_support;
    }
    if value.parse_escape_sequences.is_some() {
      self.parse_escape_sequences = value.parse_escape_sequences;
    }
    if value.display_tooltip_when_elided.is_some() {
      self.display_tooltip_when_elided = value.display_tooltip_when_elided;
    }
    if value.selectable.is_some() {
      self.selectable = value.selectable;
    }
    if value.double_click_selects_word.is_some() {
      self.double_click_selects_word = value.double_click_selects_word;
    }
    if value.triple_click_selects_line.is_some() {
      self.triple_click_selects_line = value.triple_click_selects_line;
    }
    if value.select_all_on_focus.is_some() {
      self.select_all_on_focus = value.select_all_on_focus;
    }
    if value.select_all_on_mouse_up.is_some() {
      self.select_all_on_mouse_up = value.select_all_on_mouse_up;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl VisualElementProperties for PopupWindow {
  fn visual_element(&self) -> &VisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut VisualElement {
    &mut self.element
  }
}
