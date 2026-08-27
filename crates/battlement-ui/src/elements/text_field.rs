use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, ScrollerVisibility, Style, UsageHint, VisualElement,
  VisualElementProperties,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled editable text input with a native local draft.
///
/// Typing changes only the native draft. Subscribed [`crate::UiEventKind::Input`]
/// events expose that draft, while Enter in a single-line field or focus loss
/// submits one [`crate::UiEventKind::ValueCommitted`] proposal. Escape silently
/// restores the latest value authored by Rust.
///
/// Single-line fields submit on Enter; multiline fields insert a newline and
/// submit when focus leaves the field. [`Self::placeholder`] is an empty-value
/// hint, not a default value. [`Self::read_only`] preserves selection and copy
/// behavior while preventing edits, whereas disabling the complete element also
/// removes normal interaction. Password mode masks display but does not encrypt
/// [`Self::value`] or event payloads.
///
/// Cursor and selection indices use UTF-16 code units to match Unity. The two
/// endpoints may differ to describe a selection; equal values describe a caret.
/// A multiline field can expose its internal scroll view and vertical scroller
/// through the corresponding part-style builders.
///
/// See Unity's [TextField manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-TextField.html)
/// for editing modes, keyboard behavior, placeholder text, and native styling.
///
/// # Example
///
/// ```
/// use battlement_ui::{TextField, UiEventKind};
///
/// let callsign = TextField::new()
///     .label("Callsign")
///     .placeholder("Enter a name")
///     .value("Rook")
///     .events([UiEventKind::Input, UiEventKind::ValueCommitted]);
///
/// assert_eq!(callsign.value.as_deref(), Some("Rook"));
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TextField {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: VisualElement,
  /// Label displayed beside or above the editable value.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  /// Latest text committed by Rust.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub value: Option<String>,
  /// Whether the field accepts newline characters.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub multiline: Option<bool>,
  /// Visibility policy for the multiline editor's vertical scroller.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub vertical_scroller_visibility: Option<ScrollerVisibility>,
  /// Whether the native editor masks its visible characters.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub password: Option<bool>,
  /// Whether user editing is disabled while selection remains available.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub read_only: Option<bool>,
  /// Hint shown while the committed value and local draft are empty.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub placeholder: Option<String>,
  /// Whether the placeholder disappears while the field has focus.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub hide_placeholder_on_focus: Option<bool>,
  /// Rust-authored caret endpoint measured in UTF-16 code units.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub cursor_index: Option<u32>,
  /// Rust-authored selection anchor measured in UTF-16 code units.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub select_index: Option<u32>,
  /// Whether focus selects the complete value.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub select_all_on_focus: Option<bool>,
  /// Whether pointer release selects the complete value.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub select_all_on_mouse_up: Option<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl TextField {
  /// Creates an empty single-line text field.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `TextFieldLabel` part.
  #[must_use]
  pub fn label_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldLabel, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldInput` part.
  #[must_use]
  pub fn input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldInput, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldTextElement` part.
  #[must_use]
  pub fn text_element_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldTextElement, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldMultilineScrollView` part.
  #[must_use]
  pub fn multiline_scroll_view_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldMultilineScrollView, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldVerticalScroller` part.
  #[must_use]
  pub fn vertical_scroller_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldVerticalScroller, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldVerticalSlider` part.
  #[must_use]
  pub fn vertical_slider_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldVerticalSlider, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldVerticalLowButton` part.
  #[must_use]
  pub fn vertical_low_button_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldVerticalLowButton, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldVerticalHighButton` part.
  #[must_use]
  pub fn vertical_high_button_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldVerticalHighButton, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldVerticalTrack` part.
  #[must_use]
  pub fn vertical_track_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldVerticalTrack, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldVerticalDragger` part.
  #[must_use]
  pub fn vertical_dragger_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldVerticalDragger, value);
    self
  }

  /// Applies sparse inline declarations to the native `TextFieldVerticalDraggerBorder` part.
  #[must_use]
  pub fn vertical_dragger_border_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::TextFieldVerticalDraggerBorder, value);
    self
  }

  /// Sets the field label.
  #[must_use]
  pub fn label(mut self, value: impl Into<String>) -> Self {
    self.label = Some(value.into());
    self
  }

  /// Sets the latest Rust-committed value.
  #[must_use]
  pub fn value(mut self, value: impl Into<String>) -> Self {
    self.value = Some(value.into());
    self
  }

  /// Enables or disables multiline editing.
  #[must_use]
  pub fn multiline(mut self, value: bool) -> Self {
    self.multiline = Some(value);
    self
  }

  /// Sets the vertical scroller policy used by multiline editing.
  #[must_use]
  pub fn vertical_scroller_visibility(mut self, value: ScrollerVisibility) -> Self {
    self.vertical_scroller_visibility = Some(value);
    self
  }

  /// Enables or disables password masking.
  #[must_use]
  pub fn password(mut self, value: bool) -> Self {
    self.password = Some(value);
    self
  }

  /// Enables or disables read-only editing behavior.
  #[must_use]
  pub fn read_only(mut self, value: bool) -> Self {
    self.read_only = Some(value);
    self
  }

  /// Sets the empty-value editing hint.
  #[must_use]
  pub fn placeholder(mut self, value: impl Into<String>) -> Self {
    self.placeholder = Some(value.into());
    self
  }

  /// Sets whether focus hides the placeholder.
  #[must_use]
  pub fn hide_placeholder_on_focus(mut self, value: bool) -> Self {
    self.hide_placeholder_on_focus = Some(value);
    self
  }

  /// Sets the caret endpoint index.
  #[must_use]
  pub fn cursor_index(mut self, value: u32) -> Self {
    self.cursor_index = Some(value);
    self
  }

  /// Sets the selection anchor index.
  #[must_use]
  pub fn select_index(mut self, value: u32) -> Self {
    self.select_index = Some(value);
    self
  }

  /// Sets whether focus selects all text.
  #[must_use]
  pub fn select_all_on_focus(mut self, value: bool) -> Self {
    self.select_all_on_focus = Some(value);
    self
  }

  /// Sets whether pointer release selects all text.
  #[must_use]
  pub fn select_all_on_mouse_up(mut self, value: bool) -> Self {
    self.select_all_on_mouse_up = Some(value);
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if value.label.is_some() {
      self.label.clone_from(&value.label);
    }
    if value.value.is_some() {
      self.value.clone_from(&value.value);
    }
    if value.multiline.is_some() {
      self.multiline = value.multiline;
    }
    if value.multiline == Some(false) {
      parts::remove(
        &mut self.parts,
        &[
          Part::TextFieldMultilineScrollView,
          Part::TextFieldVerticalScroller,
          Part::TextFieldVerticalSlider,
          Part::TextFieldVerticalLowButton,
          Part::TextFieldVerticalHighButton,
          Part::TextFieldVerticalTrack,
          Part::TextFieldVerticalDragger,
          Part::TextFieldVerticalDraggerBorder,
        ],
      );
    }
    if value.vertical_scroller_visibility.is_some() {
      self.vertical_scroller_visibility = value.vertical_scroller_visibility;
    }
    if value.password.is_some() {
      self.password = value.password;
    }
    if value.read_only.is_some() {
      self.read_only = value.read_only;
    }
    if value.placeholder.is_some() {
      self.placeholder.clone_from(&value.placeholder);
    }
    if value.hide_placeholder_on_focus.is_some() {
      self.hide_placeholder_on_focus = value.hide_placeholder_on_focus;
    }
    if value.cursor_index.is_some() {
      self.cursor_index = value.cursor_index;
    }
    if value.select_index.is_some() {
      self.select_index = value.select_index;
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

impl VisualElementProperties for TextField {
  fn visual_element(&self) -> &VisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut VisualElement {
    &mut self.element
  }
}
