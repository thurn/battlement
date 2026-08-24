use serde::{Deserialize, Serialize};

use crate::{Style, VisualElement, VisualElementProperties};

/// A Unity UI Toolkit control that activates from a pointer or navigation submit.
///
/// Buttons are appropriate for discrete user commands such as confirming a
/// choice or opening another view. Calling [`Self::events`] with
/// [`UiEventKind::Click`] subscribes the Rust rules engine to activations;
/// constructing a button alone does not forward events.
///
/// Unity renders [`Self::text`] using the button's internal text element and
/// supplies the standard `.unity-button` appearance and interaction states.
/// Battlement models `Button` as a leaf, so additional logical [`UiNode`]
/// children are rejected. Use [`VisualElement`] or [`Box`] to compose content
/// that needs its own child hierarchy.
///
/// See Unity's [Button manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Button.html)
/// for native activation, content, and styling behavior.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Button, UiEventKind, UiNode};
///
/// let save = UiNode::new(
///     ObjectId::new_v4(),
///     Button::new("Save").name("save-button").events([UiEventKind::Click]),
/// );
///
/// assert!(save.children.is_empty());
/// ```
///
/// [`Box`]: crate::Box
/// [`UiEventKind::Click`]: crate::UiEventKind::Click
/// [`UiNode`]: crate::UiNode
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Button {
    /// Name, enabled state, USS classes, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Text rendered inside the button's native Unity text element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Button {
    /// Creates a leaf button displaying `text` without an event subscription.
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
