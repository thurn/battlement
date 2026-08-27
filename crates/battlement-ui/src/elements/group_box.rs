use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
    elements::parts::{self, Part, PartStyle},
};

/// A Unity UI Toolkit container that groups related controls under an optional title.
///
/// Children are inserted through Unity's public content API. An empty title
/// keeps the native title label absent, while a nonempty title creates it.
/// Group boxes are useful for expressing a relationship between fields without
/// imposing [`Box`]'s themed border and background. They also establish the
/// native scope used by standalone radio buttons.
///
/// See Unity's [GroupBox manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-GroupBox.html)
/// for title and child-container behavior.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{GroupBox, Toggle, UiNode};
///
/// let accessibility = UiNode::new(
///     ObjectId::new_v4(),
///     GroupBox::new().text("Accessibility"),
/// )
/// .child(UiNode::new(
///     ObjectId::new_v4(),
///     Toggle::new().text("Show subtitles").value(true),
/// ));
///
/// assert_eq!(accessibility.children.len(), 1);
/// ```
///
/// [`Box`]: crate::Box
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
        if value.text.as_deref() == Some("") {
            parts::remove(&mut self.parts, &[Part::GroupBoxTitle]);
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
