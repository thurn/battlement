use serde::{Deserialize, Serialize};

use crate::{Style, UiEventKind, VisualElementProperties};

/// Unity UI Toolkit's general-purpose layout and hierarchy element.
///
/// Use a `VisualElement` to group child elements, apply a shared style, or
/// create a structural region that needs no control behavior. Unlike [`Box`],
/// it does not receive Unity's themed box background, border color, or border
/// width. Unlike [`Label`] and [`Button`], it may contain logical children in a
/// [`UiNode`] tree.
///
/// Battlement serializes only the shared properties it supports. The Unity host
/// creates a native `UnityEngine.UIElements.VisualElement` and adds authored
/// [`UiNode`] children directly to its content container.
///
/// See Unity's [VisualElement manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-VisualElement.html)
/// for the corresponding native element and inherited UI Toolkit behavior.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{Label, Style, UiNode, VisualElement};
///
/// let group = UiNode::new(
///     ObjectId::new_v4(),
///     VisualElement::new().name("status").style(Style::new().padding(12.0)),
/// )
/// .child(UiNode::new(ObjectId::new_v4(), Label::new("Connected")));
///
/// assert_eq!(group.children.len(), 1);
/// ```
///
/// [`Box`]: crate::Box
/// [`Button`]: crate::Button
/// [`Label`]: crate::Label
/// [`UiNode`]: crate::UiNode
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct VisualElement {
    /// Name used by Unity queries and the `#name` USS selector.
    ///
    /// Names are not the Battlement object identity. Use the enclosing
    /// [`UiNode::object_id`](crate::UiNode::object_id) for commands and events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Local enabled state of this element.
    ///
    /// A locally enabled element is still disabled in the hierarchy when an
    /// ancestor is disabled. Disabled elements do not receive ordinary input
    /// events and Unity applies its disabled USS class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// USS classes applied to this element in list order.
    ///
    /// Class names are matched by `.class-name` selectors. Empty or duplicate
    /// entries are rejected by [`validate_documents`](crate::validate_documents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// Inline style declarations applied after matching USS rules.
    ///
    /// During a property update, populated style fields replace their live
    /// counterparts and unpopulated fields preserve the current value.
    #[serde(default, skip_serializing_if = "Style::is_empty")]
    pub style: Style,
    /// Native event kinds that Unity forwards to the Rust rules engine.
    ///
    /// Subscriptions are opt-in. Repeating an event kind is invalid; ordering is
    /// retained in the protocol but does not change event dispatch semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<UiEventKind>>,
}

impl VisualElement {
    /// Creates an unstyled, enabled-by-default structural element.
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
