use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// An output-only progress indicator with a Rust-authored value and title.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ProgressBar {
    /// Properties shared by every visual element.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Lower endpoint of the displayed range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub low_value: Option<f32>,
    /// Upper endpoint of the displayed range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_value: Option<f32>,
    /// Rust-authored displayed value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f32>,
    /// Text drawn over the progress track.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl ProgressBar {
    /// Creates an empty progress indicator spanning `0..100`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    parts::part_style_builders!(
        container_style => ProgressBarContainer,
        background_style => ProgressBarBackground,
        progress_style => ProgressBarProgress,
        title_container_style => ProgressBarTitleContainer,
        title_style => ProgressBarTitle,
    );

    /// Sets the lower endpoint of the displayed range.
    #[must_use]
    pub fn low_value(mut self, value: f32) -> Self {
        self.low_value = Some(value);
        self
    }

    /// Sets the upper endpoint of the displayed range.
    #[must_use]
    pub fn high_value(mut self, value: f32) -> Self {
        self.high_value = Some(value);
        self
    }

    /// Sets the Rust-authored displayed value.
    #[must_use]
    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    /// Sets the title drawn over the progress track.
    #[must_use]
    pub fn title(mut self, value: impl Into<String>) -> Self {
        self.title = Some(value.into());
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
        if value.value.is_some() {
            self.value = value.value;
        }
        if value.title.is_some() {
            self.title.clone_from(&value.title);
        }
        parts::merge(&mut self.parts, &value.parts);
    }
}

impl VisualElementProperties for ProgressBar {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
