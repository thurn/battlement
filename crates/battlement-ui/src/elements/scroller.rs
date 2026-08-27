use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
  elements::parts::{self, Part, PartStyle},
};

/// Orientation of a [`Scroller`]'s track and value progression.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SliderDirection {
  /// Places the low button on the left and the high button on the right.
  Horizontal,
  /// Places the low button below the track and the high button above it.
  Vertical,
}

/// A controlled scrollbar that proposes floating-point values within a range.
///
/// User interaction changes the native value temporarily. Battlement emits
/// [`crate::UiEventKind::ValueChanging`] while subscribed and one
/// [`crate::UiEventKind::ValueCommitted`] when the pointer releases, then restores
/// the latest Rust-authored value until a response updates it. Unlike a slider,
/// a scroller exposes no authored page-size property.
///
/// A scroller includes decrement and increment buttons around its internal
/// slider. Use it as a standalone scrollbar when another view's offset is
/// controlled separately; [`ScrollView`] already owns its native scrollers.
/// Direction controls both the track axis and button placement.
///
/// See Unity's [Scroller manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Scroller.html)
/// and [scripting API](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.Scroller.html).
///
/// # Example
///
/// ```
/// use battlement_ui::{Scroller, SliderDirection, UiEventKind};
///
/// let timeline = Scroller::new()
///     .direction(SliderDirection::Horizontal)
///     .low_value(0.0)
///     .high_value(120.0)
///     .value(30.0)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(timeline.value, Some(30.0));
/// ```
///
/// [`ScrollView`]: crate::ScrollView
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Scroller {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: VisualElement,
  /// Inclusive minimum of the selectable range.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub low_value: Option<f32>,
  /// Inclusive maximum of the selectable range.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub high_value: Option<f32>,
  /// Track orientation and placement of the decrement and increment buttons.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub direction: Option<SliderDirection>,
  /// Latest value committed by Rust; user proposals are temporary until updated.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub value: Option<f32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl Scroller {
  /// Creates a vertical scroller using Unity's zero-valued range defaults.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `ScrollerSlider` part.
  #[must_use]
  pub fn slider_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ScrollerSlider, value);
    self
  }

  /// Applies sparse inline declarations to the native `ScrollerLowButton` part.
  #[must_use]
  pub fn low_button_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ScrollerLowButton, value);
    self
  }

  /// Applies sparse inline declarations to the native `ScrollerHighButton` part.
  #[must_use]
  pub fn high_button_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ScrollerHighButton, value);
    self
  }

  /// Applies sparse inline declarations to the native `ScrollerTrack` part.
  #[must_use]
  pub fn track_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ScrollerTrack, value);
    self
  }

  /// Applies sparse inline declarations to the native `ScrollerDragger` part.
  #[must_use]
  pub fn dragger_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ScrollerDragger, value);
    self
  }

  /// Applies sparse inline declarations to the native `ScrollerDraggerBorder` part.
  #[must_use]
  pub fn dragger_border_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ScrollerDraggerBorder, value);
    self
  }

  /// Sets the inclusive minimum of the selectable range.
  #[must_use]
  pub fn low_value(mut self, value: f32) -> Self {
    self.low_value = Some(value);
    self
  }
  /// Sets the inclusive maximum of the selectable range.
  #[must_use]
  pub fn high_value(mut self, value: f32) -> Self {
    self.high_value = Some(value);
    self
  }
  /// Sets the track orientation and button placement.
  #[must_use]
  pub fn direction(mut self, value: SliderDirection) -> Self {
    self.direction = Some(value);
    self
  }
  /// Sets the committed value written silently to the native slider.
  #[must_use]
  pub fn value(mut self, value: f32) -> Self {
    self.value = Some(value);
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if value.low_value.is_some() {
      self.low_value = value.low_value;
    }
    if value.high_value.is_some() {
      self.high_value = value.high_value;
    }
    if value.direction.is_some() {
      self.direction = value.direction;
    }
    if value.value.is_some() {
      self.value = value.value;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl VisualElementProperties for Scroller {
  fn visual_element(&self) -> &VisualElement {
    &self.element
  }
  fn visual_element_mut(&mut self) -> &mut VisualElement {
    &mut self.element
  }
}
