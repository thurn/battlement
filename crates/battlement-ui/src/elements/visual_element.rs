use serde::{Deserialize, Serialize};

use crate::{Style, UiEventKind, VisualElementProperties};

/// Builds Unity's general-purpose UI Toolkit `VisualElement`.
///
/// A visual element is the base layout, styling, and hierarchy node used by UI
/// Toolkit. It has no control-specific behavior or built-in box presentation,
/// making it suitable for structural containers and custom styled regions.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct VisualElement {
    /// The name of this visual element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether this visual element is enabled locally.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// The USS classes of this visual element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// The style values on this visual element.
    #[serde(default, skip_serializing_if = "Style::is_empty")]
    pub style: Style,
    /// UI events forwarded to Rust.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<UiEventKind>>,
}

impl VisualElement {
    /// Creates a `VisualElement`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    pub(crate) fn apply_update(&mut self, value: &Self) {
        if let Some(name) = &value.name {
            self.name = Some(name.clone());
        }
        if let Some(enabled) = value.enabled {
            self.enabled = Some(enabled);
        }
        if let Some(classes) = &value.classes {
            self.classes = Some(classes.clone());
        }
        self.style = self.style.clone().merge(value.style.clone());
        if let Some(events) = &value.events {
            self.events = Some(events.clone());
        }
    }
}

impl VisualElementProperties for VisualElement {
    fn visual_element(&self) -> &VisualElement {
        self
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        self
    }
}
