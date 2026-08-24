use serde::{Deserialize, Serialize};

use crate::{Style, VisualElement, VisualElementProperties};

/// A clickable button with a text label element.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Button {
    /// Properties inherited from `VisualElement`.
    #[serde(flatten)]
    pub element: VisualElement,
    /// The text to be displayed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Button {
    /// Creates a button containing `text`.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            element: VisualElement::default(),
            text: Some(text.into()),
        }
    }

    impl_common_visual_element_methods!();

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if let Some(text) = &value.text {
            self.text = Some(text.clone());
        }
    }
}

impl VisualElementProperties for Button {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
