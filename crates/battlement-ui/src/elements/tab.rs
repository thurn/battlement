use serde::{Deserialize, Serialize};

use crate::{
    IconSource, LanguageDirection, PickingMode, Style, UsageHint, VisualElement,
    VisualElementProperties,
};

/// One labeled, optionally icon-bearing page inside a [`TabView`](crate::TabView).
///
/// A tab is a logical container for its page content. It may only be placed
/// directly beneath a tab view; other elements cannot be direct tab-view
/// children.
///
/// See Unity's [Tab manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-Tab.html).
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct Tab {
    /// Shared visual properties, inline style, and event subscriptions.
    #[serde(flatten)]
    pub element: VisualElement,
    /// Text shown in the native tab header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Prepared graphical asset shown in the native tab header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconSource>,
    /// Whether the native tab header displays a close control.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closeable: Option<bool>,
}

impl Tab {
    /// Creates a tab with the supplied header text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Self::default()
        }
    }

    impl_common_visual_element_methods!();

    /// Selects a prepared graphical asset for the native header icon.
    #[must_use]
    pub fn icon(mut self, value: impl Into<IconSource>) -> Self {
        self.icon = Some(value.into());
        self
    }

    /// Shows or hides the native close control.
    #[must_use]
    pub fn closeable(mut self, value: bool) -> Self {
        self.closeable = Some(value);
        self
    }

    pub(crate) fn apply_update(&mut self, value: &Self) {
        self.element.apply_update(&value.element);
        if value.text.is_some() {
            self.text.clone_from(&value.text);
        }
        if value.icon.is_some() {
            self.icon.clone_from(&value.icon);
        }
        if value.closeable.is_some() {
            self.closeable = value.closeable;
        }
    }
}

impl VisualElementProperties for Tab {
    fn visual_element(&self) -> &VisualElement {
        &self.element
    }

    fn visual_element_mut(&mut self) -> &mut VisualElement {
        &mut self.element
    }
}
