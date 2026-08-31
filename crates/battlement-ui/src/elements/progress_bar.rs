use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A read-only indicator that visualizes progress through a numeric range.
///
/// Use a progress bar to communicate ongoing work or completion without asking
/// the user for input. [`Self::low_value`] and [`Self::high_value`] define the
/// range, [`Self::value`] controls the filled proportion, and [`Self::title`]
/// draws explanatory text over the track. Values outside the range are clamped
/// by Unity for display.
///
/// Unlike [`UiSlider`], a progress bar is not interactive and does not emit value
/// proposals. Update its Rust-authored value as the underlying operation
/// advances. The control is a logical leaf, with named style builders for the
/// background, progress fill, and title layers.
///
/// See Unity's [ProgressBar manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-ProgressBar.html)
/// for native range and styling behavior.
///
/// # Example
///
/// ```
/// use battlement_ui::UiProgressBar;
///
/// let loading = UiProgressBar::new()
///     .low_value(0.0)
///     .high_value(10.0)
///     .value(7.0)
///     .title("Loading encounter 7/10");
///
/// assert_eq!(loading.value, battlement_ui::Prop::Set(7.0));
/// ```
///
/// [`UiSlider`]: crate::UiSlider
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiProgressBar {
  /// Properties shared by every visual element.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Lower endpoint of the displayed range.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub low_value: Prop<f32>,
  /// Upper endpoint of the displayed range.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub high_value: Prop<f32>,
  /// Rust-authored displayed value.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub value: Prop<f32>,
  /// Text drawn over the progress track.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub title: Prop<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiProgressBar {
  /// Creates an empty progress indicator spanning `0..100`.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `ProgressBarContainer` part.
  #[must_use]
  pub fn container_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ProgressBarContainer, value);
    self
  }

  /// Applies sparse inline declarations to the native `ProgressBarBackground` part.
  #[must_use]
  pub fn background_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ProgressBarBackground, value);
    self
  }

  /// Applies sparse inline declarations to the native `ProgressBarProgress` part.
  #[must_use]
  pub fn progress_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ProgressBarProgress, value);
    self
  }

  /// Applies sparse inline declarations to the native `ProgressBarTitleContainer` part.
  #[must_use]
  pub fn title_container_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ProgressBarTitleContainer, value);
    self
  }

  /// Applies sparse inline declarations to the native `ProgressBarTitle` part.
  #[must_use]
  pub fn title_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ProgressBarTitle, value);
    self
  }

  /// Sets the lower endpoint of the displayed range.
  #[must_use]
  pub fn low_value(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.low_value = value.into();
    self
  }

  /// Sets the upper endpoint of the displayed range.
  #[must_use]
  pub fn high_value(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.high_value = value.into();
    self
  }

  /// Sets the Rust-authored displayed value.
  #[must_use]
  pub fn value(mut self, value: impl Into<Prop<f32>>) -> Self {
    self.value = value.into();
    self
  }

  /// Sets the title drawn over the progress track.
  #[must_use]
  pub fn title(mut self, value: impl Into<Prop<String>>) -> Self {
    self.title = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !value.low_value.is_unset() {
      self.low_value = value.low_value;
    }
    if !value.high_value.is_unset() {
      self.high_value = value.high_value;
    }
    if !value.value.is_unset() {
      self.value = value.value;
    }
    if !value.title.is_unset() {
      self.title.clone_from(&value.title);
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiProgressBar {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
