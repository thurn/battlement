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
/// use battlement_ui::{Style, TextElement, UiEventKind, WhiteSpace};
///
/// let help = TextElement::new("Read the <link=rules>rules</link>")
///     .rich_text(true)
///     .selectable(true)
///     .tooltip_when_elided(true)
///     .style(Style::new().white_space(WhiteSpace::Normal))
///     .events([UiEventKind::LinkUp]);
///
/// assert_eq!(help.selectable, Some(true));
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
