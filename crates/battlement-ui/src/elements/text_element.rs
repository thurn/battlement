use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UsageHint, VisualElement, VisualElementProperties,
};

/// A leaf Unity UI Toolkit text element for styled, rich, or selectable text.
///
/// Unlike [`Label`](crate::Label), this maps directly to Unity's `TextElement`
/// base class and is useful when label-specific USS identity is unnecessary.
/// Battlement writes text without raising a native value-change event and does
/// not allow authored children or text editing APIs.
///
/// Supported Unity rich-text tags can change presentation and define link
/// regions. Subscribe to the `Link*` [`UiEventKind`] variants when Rust needs to
/// react to those regions. Selection settings allow users to copy rendered text
/// but do not make the element editable; use [`TextField`] for input.
///
/// Text layout follows inherited and inline text styles. In particular,
/// [`Style::white_space`] controls wrapping and [`Style::text_overflow`] controls
/// elision. [`Self::tooltip_when_elided`] asks Unity to reveal the complete text
/// in a tooltip when truncation occurs.
///
/// See Unity's [TextElement manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-TextElement.html)
/// for rich text, selection, and inherited text attributes.
///
/// # Example
///
/// ```
/// use battlement_ui::{Prop, Style, TextElement, UiEventKind, WhiteSpace};
///
/// let help = TextElement::new("Read the <link=rules>rules</link>")
///     .rich_text(true)
///     .selectable(true)
///     .tooltip_when_elided(true)
///     .style(Style::new().white_space(WhiteSpace::Normal))
///     .events([UiEventKind::LinkUp]);
///
/// assert_eq!(help.selectable, Prop::Set(true));
/// ```
///
/// [`TextField`]: crate::TextField
/// [`UiEventKind`]: crate::UiEventKind
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TextElement {
  /// Name, enabled state, USS classes, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: VisualElement,
  /// Text rendered by Unity's text system.
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
}

impl TextElement {
  /// Creates a leaf text element displaying `text`.
  #[must_use]
  pub fn new(text: impl Into<String>) -> Self {
    Self {
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

impl VisualElementProperties for TextElement {
  fn visual_element(&self) -> &VisualElement {
    &self.element
  }
  fn visual_element_mut(&mut self) -> &mut VisualElement {
    &mut self.element
  }
}
