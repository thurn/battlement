use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, SliderDirection, Style, UsageHint, VisualElement,
    VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// A controlled floating-point range slider.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Slider {
    /// Properties shared by every visual element.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Optional field label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Lower endpoint of the selectable range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low_value: Option<f32>,
    /// Upper endpoint of the selectable range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_value: Option<f32>,
    /// Controlled selected value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f32>,
    /// Whether the track is filled through the selected value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<bool>,
    /// Distance changed by a track-page interaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<f32>,
    /// Whether the native numeric input is shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_input_field: Option<bool>,
    /// Axis along which the slider moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<SliderDirection>,
    /// Whether the visual range direction is reversed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl Slider {
    /// Creates a horizontal floating-point slider with native defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    parts::part_style_builders!(
        label_style => SliderLabel,
        input_style => SliderInput,
        track_style => SliderTrack,
        dragger_style => SliderDragger,
        dragger_border_style => SliderDraggerBorder,
        fill_style => SliderFill,
        text_input_style => SliderTextInput,
    );

    /// Sets the field caption.
    #[must_use]
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Sets the inclusive minimum.
    #[must_use]
    pub fn low_value(mut self, value: f32) -> Self {
        self.low_value = Some(value);
        self
    }

    /// Sets the inclusive maximum.
    #[must_use]
    pub fn high_value(mut self, value: f32) -> Self {
        self.high_value = Some(value);
        self
    }

    /// Sets the Rust-authored committed value.
    #[must_use]
    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    /// Controls whether the selected track segment is filled.
    #[must_use]
    pub fn fill(mut self, value: bool) -> Self {
        self.fill = Some(value);
        self
    }

    /// Sets the track-page increment.
    #[must_use]
    pub fn page_size(mut self, value: f32) -> Self {
        self.page_size = Some(value);
        self
    }

    /// Controls whether a numeric input is displayed.
    #[must_use]
    pub fn show_input_field(mut self, value: bool) -> Self {
        self.show_input_field = Some(value);
        self
    }

    /// Sets the track orientation.
    #[must_use]
    pub fn direction(mut self, value: SliderDirection) -> Self {
        self.direction = Some(value);
        self
    }

    /// Reverses the low-to-high visual direction.
    #[must_use]
    pub fn inverted(mut self, value: bool) -> Self {
        self.inverted = Some(value);
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.label.is_some() {
            self.label.clone_from(&value.label);
        }
        if value.low_value.is_some() {
            self.low_value = value.low_value;
        }
        if value.high_value.is_some() {
            self.high_value = value.high_value;
        }
        if value.value.is_some() {
            self.value = value.value;
        }
        if value.fill.is_some() {
            self.fill = value.fill;
        }
        if value.fill == Some(false) {
            parts::remove(&mut self.parts, &[Part::SliderFill]);
        }
        if value.page_size.is_some() {
            self.page_size = value.page_size;
        }
        if value.show_input_field.is_some() {
            self.show_input_field = value.show_input_field;
        }
        if value.show_input_field == Some(false) {
            parts::remove(&mut self.parts, &[Part::SliderTextInput]);
        }
        if value.direction.is_some() {
            self.direction = value.direction;
        }
        if value.inverted.is_some() {
            self.inverted = value.inverted;
        }
        parts::merge(&mut self.parts, &value.parts);
    }
}

impl VisualElementProperties for Slider {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}

/// A controlled integer range slider.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct SliderInt {
    /// Properties shared by every visual element.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Optional field label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Lower endpoint of the selectable range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low_value: Option<i32>,
    /// Upper endpoint of the selectable range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_value: Option<i32>,
    /// Controlled selected value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<i32>,
    /// Whether the track is filled through the selected value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<bool>,
    /// Distance changed by a track-page interaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_size: Option<f32>,
    /// Whether the native numeric input is shown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_input_field: Option<bool>,
    /// Axis along which the slider moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<SliderDirection>,
    /// Whether the visual range direction is reversed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inverted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl SliderInt {
    /// Creates a horizontal integer slider with native defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    parts::part_style_builders!(
        label_style => SliderIntLabel,
        input_style => SliderIntInput,
        track_style => SliderIntTrack,
        dragger_style => SliderIntDragger,
        dragger_border_style => SliderIntDraggerBorder,
        fill_style => SliderIntFill,
        text_input_style => SliderIntTextInput,
    );

    /// Sets the field caption.
    #[must_use]
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }

    /// Sets the inclusive minimum.
    #[must_use]
    pub fn low_value(mut self, value: i32) -> Self {
        self.low_value = Some(value);
        self
    }

    /// Sets the inclusive maximum.
    #[must_use]
    pub fn high_value(mut self, value: i32) -> Self {
        self.high_value = Some(value);
        self
    }

    /// Sets the Rust-authored committed value.
    #[must_use]
    pub fn value(mut self, value: i32) -> Self {
        self.value = Some(value);
        self
    }

    /// Controls whether the selected track segment is filled.
    #[must_use]
    pub fn fill(mut self, value: bool) -> Self {
        self.fill = Some(value);
        self
    }

    /// Sets the track-page increment.
    #[must_use]
    pub fn page_size(mut self, value: f32) -> Self {
        self.page_size = Some(value);
        self
    }

    /// Controls whether a numeric input is displayed.
    #[must_use]
    pub fn show_input_field(mut self, value: bool) -> Self {
        self.show_input_field = Some(value);
        self
    }

    /// Sets the track orientation.
    #[must_use]
    pub fn direction(mut self, value: SliderDirection) -> Self {
        self.direction = Some(value);
        self
    }

    /// Reverses the low-to-high visual direction.
    #[must_use]
    pub fn inverted(mut self, value: bool) -> Self {
        self.inverted = Some(value);
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.label.is_some() {
            self.label.clone_from(&value.label);
        }
        if value.low_value.is_some() {
            self.low_value = value.low_value;
        }
        if value.high_value.is_some() {
            self.high_value = value.high_value;
        }
        if value.value.is_some() {
            self.value = value.value;
        }
        if value.fill.is_some() {
            self.fill = value.fill;
        }
        if value.fill == Some(false) {
            parts::remove(&mut self.parts, &[Part::SliderIntFill]);
        }
        if value.page_size.is_some() {
            self.page_size = value.page_size;
        }
        if value.show_input_field.is_some() {
            self.show_input_field = value.show_input_field;
        }
        if value.show_input_field == Some(false) {
            parts::remove(&mut self.parts, &[Part::SliderIntTextInput]);
        }
        if value.direction.is_some() {
            self.direction = value.direction;
        }
        if value.inverted.is_some() {
            self.inverted = value.inverted;
        }
        parts::merge(&mut self.parts, &value.parts);
    }
}

impl VisualElementProperties for SliderInt {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
