use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// Orientation of a [`UiScroller`]'s track and value progression.
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
/// controlled separately; [`UiScrollView`] already owns its native scrollers.
/// Direction controls both the track axis and button placement.
///
/// See Unity's [Scroller manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Scroller.html)
/// and [scripting API](https://docs.unity3d.com/6000.5/Documentation/ScriptReference/UIElements.Scroller.html).
///
/// # Example
///
/// ```
/// use battlement_ui::{UiScroller, SliderDirection, UiEventKind};
///
/// let timeline = UiScroller::new()
///     .direction(SliderDirection::Horizontal)
///     .low_value(0.0)
///     .high_value(120.0)
///     .value(30.0)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(timeline.value, battlement_ui::Prop::Set(30.0));
/// ```
///
/// [`UiScrollView`]: crate::UiScrollView
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiScroller {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Inclusive minimum of the selectable range.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub low_value: Prop<f32>,
  /// Inclusive maximum of the selectable range.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub high_value: Prop<f32>,
  /// Track orientation and placement of the decrement and increment buttons.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub direction: Prop<SliderDirection>,
  /// Latest value committed by Rust; user proposals are temporary until updated.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub value: Prop<f32>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiScroller {
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
  pub fn low_value(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.low_value = value.into();
    self
  }
  /// Sets the inclusive maximum of the selectable range.
  #[must_use]
  pub fn high_value(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.high_value = value.into();
    self
  }
  /// Sets the track orientation and button placement.
  #[must_use]
  pub fn direction(mut self, value: impl Into<Prop<SliderDirection>>) -> Self {
    self.direction = value.into();
    self
  }
  /// Sets the committed value written silently to the native slider.
  #[must_use]
  pub fn value(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.value = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.low_value, Prop::Unset) {
      self.low_value = value.low_value;
    }
    if !matches!(value.high_value, Prop::Unset) {
      self.high_value = value.high_value;
    }
    if !matches!(value.direction, Prop::Unset) {
      self.direction = value.direction;
    }
    if !matches!(value.value, Prop::Unset) {
      self.value = value.value;
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiScroller {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }
  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
