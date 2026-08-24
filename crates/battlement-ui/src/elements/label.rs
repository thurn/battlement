use serde::{Deserialize, Serialize};

use crate::{
    LanguageDirection, PickingMode, Style, UsageHint, VisualElement, VisualElementProperties,
};

/// A Unity UI Toolkit text element for titles, captions, and descriptions.
///
/// A label renders its [`Self::text`] through Unity's text system. Text-related
/// inline styles such as [`Style::color`] and [`Style::font_size`] apply to the
/// rendered text, while ordinary layout styles control the label's box. A
/// Battlement label is a leaf and cannot contain logical [`UiNode`] children.
/// Use [`Button`] when the text should activate an action.
///
/// See Unity's [Label manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Label.html)
/// for native text behavior and styling.
///
/// # Example
///
/// ```
/// use battlement_types::{Color, ObjectId};
/// use battlement_ui::{Label, Style, UiNode};
///
/// let title = UiNode::new(
///     ObjectId::new_v4(),
///     Label::new("Mission ready").style(
///         Style::new().color(Color::rgb(0.8, 0.9, 1.0)).font_size(18.0),
///     ),
/// );
///
/// assert!(title.children.is_empty());
/// ```
///
/// [`Button`]: crate::Button
/// [`UiNode`]: crate::UiNode
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Label {
    /// Name, enabled state, USS classes, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Text rendered by the label's native Unity `TextElement`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Label {
    /// Creates a leaf label displaying `text`.
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

impl VisualElementProperties for Label {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
