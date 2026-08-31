use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, SliderDirection, Style, UiVisualElement,
  UiVisualElementProperties, UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled floating-point value selector with a draggable thumb.
///
/// Use a slider for approximate adjustments within a bounded range, such as
/// volume or brightness. Users can drag the thumb, click the track, use arrow
/// keys for fine adjustments, hold Shift with an arrow key for larger steps,
/// or press Home and End to reach the range limits. Enable
/// [`Self::show_input_field`] when exact numeric entry is also useful.
///
/// User interaction is provisional: subscribe to
/// [`UiEventKind::ValueChanging`] for live proposals and
/// [`UiEventKind::ValueCommitted`] for the completed change. Unity then restores
/// the latest [`Self::value`] authored by Rust until an update accepts the
/// proposal. The control is a logical leaf; style its named native parts with
/// the `*_style` builders rather than adding children.
///
/// A positive [`Self::page_size`] is a percentage of the complete range, not an
/// absolute value. A page size of zero makes a track click move directly to the
/// pointer position.
///
/// See Unity's [Slider manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Slider.html)
/// for track interaction, keyboard shortcuts, and native attributes.
///
/// # Example
///
/// ```
/// use battlement_ui::{UiSlider, UiEventKind};
///
/// let volume = UiSlider::new()
///     .label("Volume")
///     .low_value(0.0)
///     .high_value(100.0)
///     .value(40.0)
///     .page_size(10.0)
///     .show_input_field(true)
///     .events([
///         UiEventKind::ValueChanging,
///         UiEventKind::ValueCommitted,
///     ]);
///
/// assert_eq!(volume.value, battlement_ui::Prop::Set(40.0));
/// ```
///
/// [`UiEventKind::ValueChanging`]: crate::UiEventKind::ValueChanging
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiSlider {
  /// Properties shared by every visual element.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Optional field label.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub label: Prop<String>,
  /// Lower endpoint of the selectable range.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub low_value: Prop<f32>,
  /// Upper endpoint of the selectable range.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub high_value: Prop<f32>,
  /// Controlled selected value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub value: Prop<f32>,
  /// Whether the track is filled through the selected value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub fill: Prop<bool>,
  /// Track-click step as a percentage of the range; zero jumps to the pointer.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub page_size: Prop<f32>,
  /// Whether the native numeric input is shown.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub show_input_field: Prop<bool>,
  /// Axis along which the slider moves.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub direction: Prop<SliderDirection>,
  /// Whether the visual range direction is reversed.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub inverted: Prop<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiSlider {
  /// Creates a horizontal floating-point slider with native defaults.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `SliderLabel` part.
  #[must_use]
  pub fn label_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderLabel, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderInput` part.
  #[must_use]
  pub fn input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderInput, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderTrack` part.
  #[must_use]
  pub fn track_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderTrack, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderDragger` part.
  #[must_use]
  pub fn dragger_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderDragger, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderDraggerBorder` part.
  #[must_use]
  pub fn dragger_border_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderDraggerBorder, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderFill` part.
  #[must_use]
  pub fn fill_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderFill, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderTextInput` part.
  #[must_use]
  pub fn text_input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderTextInput, value);
    self
  }

  /// Sets the field caption.
  #[must_use]
  pub fn label(mut self, value: impl Into<Prop<String>>) -> Self {
    self.label = value.into();
    self
  }

  /// Sets the inclusive minimum.
  #[must_use]
  pub fn low_value(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.low_value = value.into();
    self
  }

  /// Sets the inclusive maximum.
  #[must_use]
  pub fn high_value(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.high_value = value.into();
    self
  }

  /// Sets the Rust-authored committed value.
  #[must_use]
  pub fn value(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.value = value.into();
    self
  }

  /// Controls whether the selected track segment is filled.
  #[must_use]
  pub fn fill(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.fill = value.into();
    self
  }

  /// Sets the track-click step as a percentage of the complete range.
  ///
  /// Set this to zero to make a track click jump directly to the pointer.
  #[must_use]
  pub fn page_size(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.page_size = value.into();
    self
  }

  /// Controls whether a numeric input is displayed.
  #[must_use]
  pub fn show_input_field(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.show_input_field = value.into();
    self
  }

  /// Sets the track orientation.
  #[must_use]
  pub fn direction(mut self, value: impl Into<Prop<SliderDirection>>) -> Self {
    self.direction = value.into();
    self
  }

  /// Reverses the low-to-high visual direction.
  #[must_use]
  pub fn inverted(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.inverted = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !value.label.is_unset() {
      self.label.clone_from(&value.label);
    }
    if !value.low_value.is_unset() {
      self.low_value = value.low_value;
    }
    if !value.high_value.is_unset() {
      self.high_value = value.high_value;
    }
    if !value.value.is_unset() {
      self.value = value.value;
    }
    if !value.fill.is_unset() {
      self.fill = value.fill;
    }
    if value.fill == Prop::Set(false) || value.fill == Prop::Reset {
      parts::remove(&mut self.parts, &[Part::SliderFill]);
    }
    if !value.page_size.is_unset() {
      self.page_size = value.page_size;
    }
    if !value.show_input_field.is_unset() {
      self.show_input_field = value.show_input_field;
    }
    if value.show_input_field == Prop::Set(false) || value.show_input_field == Prop::Reset {
      parts::remove(&mut self.parts, &[Part::SliderTextInput]);
    }
    if !value.direction.is_unset() {
      self.direction = value.direction;
    }
    if !value.inverted.is_unset() {
      self.inverted = value.inverted;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiSlider {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}

/// A controlled integer value selector with a draggable thumb.
///
/// `UiSliderInt` has the same native interaction and visual parts as [`UiSlider`],
/// but restricts committed values to integers. It is a good fit for bounded
/// counts and discrete settings whose full range is still easier to scan on a
/// track than in a text field.
///
/// Users can drag the thumb, click the track, use arrow keys, or press Home and
/// End. Subscribe to [`UiEventKind::ValueChanging`] for live proposals and
/// [`UiEventKind::ValueCommitted`] for completed changes. The latest
/// Rust-authored [`Self::value`] remains authoritative until an update accepts a
/// proposal.
///
/// A positive [`Self::page_size`] is a percentage of the complete range. Zero
/// makes a track click jump directly to the pointer position. Enable
/// [`Self::show_input_field`] when users also need exact numeric entry.
///
/// See Unity's [SliderInt manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-SliderInt.html)
/// for native input and keyboard behavior.
///
/// # Example
///
/// ```
/// use battlement_ui::{UiSliderInt, UiEventKind};
///
/// let party_size = UiSliderInt::new()
///     .label("Party size")
///     .low_value(1)
///     .high_value(8)
///     .value(4)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(party_size.value, battlement_ui::Prop::Set(4));
/// ```
///
/// [`UiEventKind::ValueChanging`]: crate::UiEventKind::ValueChanging
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiSliderInt {
  /// Properties shared by every visual element.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Optional field label.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub label: Prop<String>,
  /// Lower endpoint of the selectable range.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub low_value: Prop<i32>,
  /// Upper endpoint of the selectable range.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub high_value: Prop<i32>,
  /// Controlled selected value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub value: Prop<i32>,
  /// Whether the track is filled through the selected value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub fill: Prop<bool>,
  /// Track-click step as a percentage of the range; zero jumps to the pointer.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub page_size: Prop<f32>,
  /// Whether the native numeric input is shown.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub show_input_field: Prop<bool>,
  /// Axis along which the slider moves.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub direction: Prop<SliderDirection>,
  /// Whether the visual range direction is reversed.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub inverted: Prop<bool>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiSliderInt {
  /// Creates a horizontal integer slider with native defaults.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `SliderIntLabel` part.
  #[must_use]
  pub fn label_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderIntLabel, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderIntInput` part.
  #[must_use]
  pub fn input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderIntInput, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderIntTrack` part.
  #[must_use]
  pub fn track_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderIntTrack, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderIntDragger` part.
  #[must_use]
  pub fn dragger_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderIntDragger, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderIntDraggerBorder` part.
  #[must_use]
  pub fn dragger_border_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderIntDraggerBorder, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderIntFill` part.
  #[must_use]
  pub fn fill_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderIntFill, value);
    self
  }

  /// Applies sparse inline declarations to the native `SliderIntTextInput` part.
  #[must_use]
  pub fn text_input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::SliderIntTextInput, value);
    self
  }

  /// Sets the field caption.
  #[must_use]
  pub fn label(mut self, value: impl Into<Prop<String>>) -> Self {
    self.label = value.into();
    self
  }

  /// Sets the inclusive minimum.
  #[must_use]
  pub fn low_value(mut self, value: impl Into<Prop<i32>>) -> Self {
    self.low_value = value.into();
    self
  }

  /// Sets the inclusive maximum.
  #[must_use]
  pub fn high_value(mut self, value: impl Into<Prop<i32>>) -> Self {
    self.high_value = value.into();
    self
  }

  /// Sets the Rust-authored committed value.
  #[must_use]
  pub fn value(mut self, value: impl Into<Prop<i32>>) -> Self {
    self.value = value.into();
    self
  }

  /// Controls whether the selected track segment is filled.
  #[must_use]
  pub fn fill(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.fill = value.into();
    self
  }

  /// Sets the track-click step as a percentage of the complete range.
  ///
  /// Set this to zero to make a track click jump directly to the pointer.
  #[must_use]
  pub fn page_size(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.page_size = value.into();
    self
  }

  /// Controls whether a numeric input is displayed.
  #[must_use]
  pub fn show_input_field(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.show_input_field = value.into();
    self
  }

  /// Sets the track orientation.
  #[must_use]
  pub fn direction(mut self, value: impl Into<Prop<SliderDirection>>) -> Self {
    self.direction = value.into();
    self
  }

  /// Reverses the low-to-high visual direction.
  #[must_use]
  pub fn inverted(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.inverted = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !value.label.is_unset() {
      self.label.clone_from(&value.label);
    }
    if !value.low_value.is_unset() {
      self.low_value = value.low_value;
    }
    if !value.high_value.is_unset() {
      self.high_value = value.high_value;
    }
    if !value.value.is_unset() {
      self.value = value.value;
    }
    if !value.fill.is_unset() {
      self.fill = value.fill;
    }
    if value.fill == Prop::Set(false) || value.fill == Prop::Reset {
      parts::remove(&mut self.parts, &[Part::SliderIntFill]);
    }
    if !value.page_size.is_unset() {
      self.page_size = value.page_size;
    }
    if !value.show_input_field.is_unset() {
      self.show_input_field = value.show_input_field;
    }
    if value.show_input_field == Prop::Set(false) || value.show_input_field == Prop::Reset {
      parts::remove(&mut self.parts, &[Part::SliderIntTextInput]);
    }
    if !value.direction.is_unset() {
      self.direction = value.direction;
    }
    if !value.inverted.is_unset() {
      self.inverted = value.inverted;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiSliderInt {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
