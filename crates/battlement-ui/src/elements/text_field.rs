use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, ScrollerVisibility, Style, UiVisualElement,
  UiVisualElementProperties, UsageHint,
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
/// use battlement_ui::{Prop, UiTextField, UiEventKind};
///
/// let callsign = UiTextField::new()
///     .label("Callsign")
///     .placeholder("Enter a name")
///     .value("Rook")
///     .events([UiEventKind::Input, UiEventKind::ValueCommitted]);
///
/// assert_eq!(callsign.value, Prop::Set("Rook".into()));
/// ```
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiTextField {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// UiLabel displayed beside or above the editable value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub label: Prop<String>,
  /// Latest text committed by Rust.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub value: Prop<String>,
  /// Whether the field accepts newline characters.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub multiline: Prop<bool>,
  /// Visibility policy for the multiline editor's vertical scroller.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub vertical_scroller_visibility: Prop<ScrollerVisibility>,
  /// Whether the native editor masks its visible characters.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub password: Prop<bool>,
  /// Whether user editing is disabled while selection remains available.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub read_only: Prop<bool>,
  /// Hint shown while the committed value and local draft are empty.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub placeholder: Prop<String>,
  /// Whether the placeholder disappears while the field has focus.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub hide_placeholder_on_focus: Prop<bool>,
  /// Rust-authored caret endpoint measured in UTF-16 code units.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub cursor_index: Prop<u32>,
  /// Rust-authored selection anchor measured in UTF-16 code units.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub select_index: Prop<u32>,
  /// Whether focus selects the complete value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub select_all_on_focus: Prop<bool>,
  /// Whether pointer release selects the complete value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub select_all_on_mouse_up: Prop<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiTextField {
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
  pub fn label(mut self, value: impl Into<Prop<String>>) -> Self {
    self.label = value.into();
    self
  }

  /// Sets the latest Rust-committed value.
  #[must_use]
  pub fn value(mut self, value: impl Into<Prop<String>>) -> Self {
    self.value = value.into();
    self
  }

  /// Enables or disables multiline editing.
  #[must_use]
  pub fn multiline(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.multiline = value.into();
    self
  }

  /// Sets the vertical scroller policy used by multiline editing.
  #[must_use]
  pub fn vertical_scroller_visibility(
    mut self,
    value: impl Into<Prop<ScrollerVisibility>>,
  ) -> Self {
    self.vertical_scroller_visibility = value.into();
    self
  }

  /// Enables or disables password masking.
  #[must_use]
  pub fn password(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.password = value.into();
    self
  }

  /// Enables or disables read-only editing behavior.
  #[must_use]
  pub fn read_only(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.read_only = value.into();
    self
  }

  /// Sets the empty-value editing hint.
  #[must_use]
  pub fn placeholder(mut self, value: impl Into<Prop<String>>) -> Self {
    self.placeholder = value.into();
    self
  }

  /// Sets whether focus hides the placeholder.
  #[must_use]
  pub fn hide_placeholder_on_focus(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.hide_placeholder_on_focus = value.into();
    self
  }

  /// Sets the caret endpoint index.
  #[must_use]
  pub fn cursor_index(mut self, value: impl Into<Prop<u32>>) -> Self {
    self.cursor_index = value.into();
    self
  }

  /// Sets the selection anchor index.
  #[must_use]
  pub fn select_index(mut self, value: impl Into<Prop<u32>>) -> Self {
    self.select_index = value.into();
    self
  }

  /// Sets whether focus selects all text.
  #[must_use]
  pub fn select_all_on_focus(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.select_all_on_focus = value.into();
    self
  }

  /// Sets whether pointer release selects all text.
  #[must_use]
  pub fn select_all_on_mouse_up(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.select_all_on_mouse_up = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.label, Prop::Unset) {
      self.label.clone_from(&value.label);
    }
    if !matches!(value.value, Prop::Unset) {
      self.value.clone_from(&value.value);
    }
    if !matches!(value.multiline, Prop::Unset) {
      self.multiline = value.multiline;
    }
    if matches!(value.multiline, Prop::Set(false) | Prop::Reset) {
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
    if !matches!(value.vertical_scroller_visibility, Prop::Unset) {
      self.vertical_scroller_visibility = value.vertical_scroller_visibility;
    }
    if !matches!(value.password, Prop::Unset) {
      self.password = value.password;
    }
    if !matches!(value.read_only, Prop::Unset) {
      self.read_only = value.read_only;
    }
    if !matches!(value.placeholder, Prop::Unset) {
      self.placeholder.clone_from(&value.placeholder);
    }
    if !matches!(value.hide_placeholder_on_focus, Prop::Unset) {
      self.hide_placeholder_on_focus = value.hide_placeholder_on_focus;
    }
    if !matches!(value.cursor_index, Prop::Unset) {
      self.cursor_index = value.cursor_index;
    }
    if !matches!(value.select_index, Prop::Unset) {
      self.select_index = value.select_index;
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

impl UiVisualElementProperties for UiTextField {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
