use serde::{Deserialize, Serialize};

use crate::{Style, VisualElement, VisualElementProperties};

/// A themed Unity UI Toolkit container with a visible box treatment.
///
/// `Box` has the same hierarchy and layout role as [`VisualElement`], but Unity
/// adds the `.unity-box` USS class. The runtime theme uses that class to provide
/// a box background, border color, and one-pixel border. Use it to visually
/// group related content; use [`VisualElement`] for a neutral structural
/// container whose appearance is entirely authored by USS or [`Style`].
///
/// A `Box` may contain logical children through [`UiNode::child`] and related
/// hierarchy builders.
///
/// See Unity's [Box manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Box.html)
/// for its native styling and inherited attributes.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Box, Label, Style, UiNode};
///
/// let panel = UiNode::new(
///     ObjectId::new_v4(),
///     Box::new().class("settings-panel").style(Style::new().padding(16.0)),
/// )
/// .child(UiNode::new(ObjectId::new_v4(), Label::new("Settings")));
///
/// assert_eq!(panel.children.len(), 1);
/// ```
///
/// [`UiNode::child`]: crate::UiNode::child
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Box {
    /// Name, enabled state, USS classes, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
}

impl Box {
    /// Creates an empty box with Unity's standard themed box styling.
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
