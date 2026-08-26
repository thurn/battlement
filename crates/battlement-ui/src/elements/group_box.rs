use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// A Unity UI Toolkit container that groups related controls under an optional title.
///
/// Children are inserted through Unity's public content API. An empty title
/// keeps the native title label absent, while a nonempty title creates it.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct GroupBox {
    /// Name, enabled state, USS classes, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Text rendered by the native group title label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parts: Option<Vec<PartStyle>>,
}

impl GroupBox {
    /// Creates an untitled group container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    parts::part_style_builders!(title_style => GroupBoxTitle);

    /// Sets the optional group title; an empty value removes the native title label.
    #[must_use]
    pub fn text(mut self, value: impl Into<String>) -> Self {
        self.text = Some(value.into());
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.text.is_some() {
            self.text.clone_from(&value.text);
        }
        parts::merge(&mut self.parts, &value.parts);
    }
}

impl VisualElementProperties for GroupBox {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
