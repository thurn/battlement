use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// Inclusive lower bound or the native unbounded minimum.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum LowerLimit {
    /// Uses Unity's native minimum without serializing the extreme value.
    #[default]
    Unbounded,
    /// Uses one finite inclusive lower bound.
    Inclusive(f32),
}

/// Inclusive upper bound or the native unbounded maximum.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub enum UpperLimit {
    /// Uses Unity's native maximum without serializing the extreme value.
    #[default]
    Unbounded,
    /// Uses one finite inclusive upper bound.
    Inclusive(f32),
}

/// A controlled floating-point interval selector with two draggable thumbs.
///
/// Use a min-max slider when users should choose both ends of a range, such as a
/// price band or acceptable difficulty window. [`Self::low_limit`] and
/// [`Self::high_limit`] constrain the track; [`Self::min_value`] and
/// [`Self::max_value`] are the selected lower and upper values. The selected
/// minimum must not exceed the selected maximum.
///
/// Users can drag either thumb or the selected range between them. Subscribe to
/// [`UiEventKind::ValueChanging`] for live [`F32Range`] proposals and
/// [`UiEventKind::ValueCommitted`] for the completed interaction. Rust remains
/// authoritative until it sends an update accepting the proposal.
///
/// The control is a logical leaf. Its label, track, thumbs, and range dragger
/// can be styled independently through the named part-style builders.
///
/// See Unity's [MinMaxSlider manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-MinMaxSlider.html)
/// for native range behavior and attributes.
///
/// # Example
///
/// ```
/// use battlement_ui::{LowerLimit, MinMaxSlider, UiEventKind, UpperLimit};
///
/// let price = MinMaxSlider::new()
///     .label("Price")
///     .low_limit(LowerLimit::Inclusive(0.0))
///     .high_limit(UpperLimit::Inclusive(500.0))
///     .min_value(50.0)
///     .max_value(200.0)
///     .events([UiEventKind::ValueCommitted]);
///
/// assert_eq!(price.min_value, Some(50.0));
/// ```
///
/// [`F32Range`]: crate::F32Range
/// [`UiEventKind::ValueChanging`]: crate::UiEventKind::ValueChanging
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MinMaxSlider {
    /// Properties shared by every visual element.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Optional field label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Rust-authored selected lower value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f32>,
    /// Rust-authored selected upper value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f32>,
    /// Inclusive lower limit or an explicit unbounded limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low_limit: Option<LowerLimit>,
    /// Inclusive upper limit or an explicit unbounded limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_limit: Option<UpperLimit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl MinMaxSlider {
    /// Creates an unbounded selector with the native selected range `0..10`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    /// Applies sparse inline declarations to the native `MinMaxSliderLabel` part.
    #[must_use]
    pub fn label_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::MinMaxSliderLabel, value);
        self
    }

    /// Applies sparse inline declarations to the native `MinMaxSliderInput` part.
    #[must_use]
    pub fn input_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::MinMaxSliderInput, value);
        self
    }

    /// Applies sparse inline declarations to the native `MinMaxSliderTrack` part.
    #[must_use]
    pub fn track_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::MinMaxSliderTrack, value);
        self
    }

    /// Applies sparse inline declarations to the native `MinMaxSliderMinimumThumb` part.
    #[must_use]
    pub fn minimum_thumb_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::MinMaxSliderMinimumThumb, value);
        self
    }

    /// Applies sparse inline declarations to the native `MinMaxSliderMaximumThumb` part.
    #[must_use]
    pub fn maximum_thumb_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::MinMaxSliderMaximumThumb, value);
        self
    }

    /// Applies sparse inline declarations to the native `MinMaxSliderRangeDragger` part.
    #[must_use]
    pub fn range_dragger_style(mut self, value: Style) -> Self {
        parts::append(&mut self.parts, Part::MinMaxSliderRangeDragger, value);
        self
    }

    /// Sets the field caption.
    #[must_use]
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Sets the Rust-authored selected lower value.
    #[must_use]
    pub fn min_value(mut self, value: f32) -> Self {
        self.min_value = Some(value);
        self
    }

    /// Sets the Rust-authored selected upper value.
    #[must_use]
    pub fn max_value(mut self, value: f32) -> Self {
        self.max_value = Some(value);
        self
    }

    /// Sets the inclusive lower limit or restores the unbounded limit.
    #[must_use]
    pub fn low_limit(mut self, value: LowerLimit) -> Self {
        self.low_limit = Some(value);
        self
    }

    /// Sets the inclusive upper limit or restores the unbounded limit.
    #[must_use]
    pub fn high_limit(mut self, value: UpperLimit) -> Self {
        self.high_limit = Some(value);
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.label.is_some() {
            self.label.clone_from(&value.label);
        }
        if value.min_value.is_some() {
            self.min_value = value.min_value;
        }
        if value.max_value.is_some() {
            self.max_value = value.max_value;
        }
        if value.low_limit.is_some() {
            self.low_limit = value.low_limit;
        }
        if value.high_limit.is_some() {
            self.high_limit = value.high_limit;
        }
        parts::merge(&mut self.parts, &value.parts);
    }
}

impl VisualElementProperties for MinMaxSlider {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
