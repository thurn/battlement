use serde::{Deserialize, Serialize};

use crate::{Style, VisualElement, VisualElementProperties};

/// A Unity UI Toolkit `Box`.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Box {
    /// Properties inherited from `VisualElement`.
    #[serde(flatten)]
    pub element: VisualElement,
}

impl Box {
    /// Creates a `Box`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    impl_common_visual_element_methods!();

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
    }
}

impl VisualElementProperties for Box {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
