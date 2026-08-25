use serde::{Deserialize, Serialize};

use crate::{Style, UiEventKind, VisualElementProperties};

/// Determines whether Unity can select an element during pointer hit testing.
///
/// This maps to Unity's `PickingMode`. Picking affects the element itself, not
/// its descendants: an ignored container may still contain pickable children.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum PickingMode {
    /// Tests the element's layout rectangle and permits it to receive pointer events.
    Position,
    /// Excludes the element from pointer picking and its hover pseudo-state.
    Ignore,
}

/// Controls the direction used to lay out and render an element's text.
///
/// The value maps to Unity's `LanguageDirection` and cascades to descendants.
/// Use [`Self::Inherit`] to follow the nearest ancestor with an explicit
/// direction, or select a concrete direction for a localized subtree.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum LanguageDirection {
    /// Uses the nearest ancestor's directionality.
    Inherit,
    /// Renders text from left to right.
    Ltr,
    /// Renders text from right to left.
    Rtl,
}

/// A rendering optimization hint supplied before an element joins a panel.
///
/// Hints map to Unity's `UsageHints` flags. They do not change layout,
/// rendering, or input results; Unity may ignore a hint when the current
/// renderer or hardware cannot use the corresponding optimization.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum UsageHint {
    /// Optimizes an element whose position or transform changes frequently.
    DynamicTransform,
    /// Optimizes a transform-changing container with many dynamic descendants.
    GroupTransform,
    /// Optimizes a container whose descendants use nested clipping masks.
    MaskContainer,
    /// Optimizes an element whose rendered colors change frequently.
    DynamicColor,
    /// Optimizes an element that receives post-processing effects.
    DynamicPostProcessing,
    /// Optimizes an element that covers a large pixel area on the panel.
    LargePixelCoverage,
}

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
    /// Controls whether pointer hit testing may select this element.
    ///
    /// [`PickingMode::Ignore`] also prevents Unity from applying the hover
    /// pseudo-state to this element, but does not make its descendants ignore
    /// picking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picking_mode: Option<PickingMode>,
    /// Text direction for this element and descendants that inherit it.
    ///
    /// [`LanguageDirection::Inherit`] follows the nearest ancestor with an
    /// explicit direction. The value affects text directionality rather than
    /// flex layout order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_direction: Option<LanguageDirection>,
    /// Whether this element is eligible to receive focus.
    ///
    /// Eligibility does not guarantee focus: the element must also be attached,
    /// enabled in its hierarchy, and accepted by Unity's focus controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focusable: Option<bool>,
    /// Position in Unity's keyboard focus ring.
    ///
    /// Nonnegative values participate in tab navigation. Negative values remove
    /// the element from the tab sequence while leaving programmatic focus
    /// eligibility controlled by [`Self::focusable`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tab_index: Option<i32>,
    /// Whether focus requested on this element transfers to a descendant.
    ///
    /// Unity chooses the first eligible descendant in focus-ring order; callers
    /// cannot name a particular delegated target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delegates_focus: Option<bool>,
    /// USS classes applied to this element in list order.
    ///
    /// Class names are matched by `.class-name` selectors. Empty or duplicate
    /// entries are rejected by [`validate_documents`](crate::validate_documents).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classes: Option<Vec<String>>,
    /// Create-time rendering optimization hints combined on the native element.
    ///
    /// Hints do not affect observable behavior and may be ignored by Unity.
    /// They can be authored only before the element joins a panel, so
    /// Battlement rejects them in sparse property updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_hints: Option<Vec<UsageHint>>,
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
        if let Some(picking_mode) = value.picking_mode {
            self.picking_mode = Some(picking_mode);
        }
        if let Some(language_direction) = value.language_direction {
            self.language_direction = Some(language_direction);
        }
        if let Some(focusable) = value.focusable {
            self.focusable = Some(focusable);
        }
        if let Some(tab_index) = value.tab_index {
            self.tab_index = Some(tab_index);
        }
        if let Some(delegates_focus) = value.delegates_focus {
            self.delegates_focus = Some(delegates_focus);
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
