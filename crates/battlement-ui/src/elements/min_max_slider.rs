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

/// A controlled dual-thumb floating-point range selector.
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

    parts::part_style_builders!(
        label_style => MinMaxSliderLabel,
        input_style => MinMaxSliderInput,
        track_style => MinMaxSliderTrack,
        minimum_thumb_style => MinMaxSliderMinimumThumb,
        maximum_thumb_style => MinMaxSliderMaximumThumb,
        range_dragger_style => MinMaxSliderRangeDragger,
    );

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
