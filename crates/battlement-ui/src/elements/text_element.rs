use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
};

/// A leaf Unity UI Toolkit text element for styled, rich, or selectable text.
///
/// Unlike [`Label`](crate::Label), this maps directly to Unity's `TextElement`
/// base class and is useful when label-specific USS identity is unnecessary.
/// Battlement writes text without raising a native value-change event and does
/// not allow authored children or text editing APIs.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TextElement {
    /// Name, enabled state, USS classes, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Text rendered by Unity's text system.
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
}

impl TextElement {
    /// Creates a leaf text element displaying `text`.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    impl_common_visual_element_methods!();

    /// Enables or disables Unity rich-text tag parsing.
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
        macro_rules! update { ($($field:ident),+ $(,)?) => {$(if value.$field.is_some() { self.$field = value.$field; })+}; }
        update!(
            enable_rich_text,
            emoji_fallback_support,
            parse_escape_sequences,
            display_tooltip_when_elided,
            selectable,
            double_click_selects_word,
            triple_click_selects_line,
            select_all_on_focus,
            select_all_on_mouse_up
        );
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
