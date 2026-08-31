use serde::{Deserialize, Serialize};

use crate::{Prop, Style, UiVisualElementProperties};

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
/// Use a `UiVisualElement` to group child elements, apply a shared style, or
/// create a structural region that needs no control behavior. Unlike [`UiBox`],
/// it does not receive Unity's themed box background, border color, or border
/// width. Unlike [`UiLabel`] and [`UiButton`], it may contain logical children in a
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
/// use battlement_ui::{UiLabel, Style, UiNode, UiVisualElement};
///
/// let group = UiNode::new(
///     ObjectId::new_v4(),
///     UiVisualElement::new().name("status").style(Style::new().padding(12.0)),
/// )
/// .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("Connected")));
///
/// assert_eq!(group.children.len(), 1);
/// ```
///
/// [`UiBox`]: crate::UiBox
/// [`UiButton`]: crate::UiButton
/// [`UiLabel`]: crate::UiLabel
/// [`UiNode`]: crate::UiNode
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiVisualElement {
  /// Name used by Unity queries and the `#name` USS selector.
  ///
  /// Names are not the Battlement object identity. Use the enclosing
  /// [`UiNode::object_id`](crate::UiNode::object_id) for commands and events.
  /// Reset restores the empty name captured after native construction.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub name: Prop<String>,
  /// Local enabled state of this element, or a request to restore `true`.
  ///
  /// A locally enabled element is still disabled in the hierarchy when an
  /// ancestor is disabled. Disabled elements do not receive ordinary input
  /// events and Unity applies its disabled USS class.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub enabled: Prop<bool>,
  /// Controls whether pointer hit testing may select this element.
  ///
  /// [`PickingMode::Ignore`] also prevents Unity from applying the hover
  /// pseudo-state to this element, but does not make its descendants ignore
  /// picking. Reset restores [`PickingMode::Position`].
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub picking_mode: Prop<PickingMode>,
  /// Text direction for this element and descendants that inherit it.
  ///
  /// [`LanguageDirection::Inherit`] follows the nearest ancestor with an
  /// explicit direction. The value affects text directionality rather than
  /// flex layout order. Reset restores [`LanguageDirection::Inherit`].
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub language_direction: Prop<LanguageDirection>,
  /// Whether this element is eligible to receive focus.
  ///
  /// Eligibility does not guarantee focus: the element must also be attached,
  /// enabled in its hierarchy, and accepted by Unity's focus controller. Reset
  /// restores the concrete native element constructor's focusability.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub focusable: Prop<bool>,
  /// Position in Unity's keyboard focus ring.
  ///
  /// Nonnegative values participate in tab navigation. Negative values remove
  /// the element from the tab sequence while leaving programmatic focus
  /// eligibility controlled by [`Self::focusable`]. Reset restores the
  /// concrete native element constructor's tab index.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub tab_index: Prop<i32>,
  /// Whether focus requested on this element transfers to a descendant.
  ///
  /// Unity chooses the first eligible descendant in focus-ring order; callers
  /// cannot name a particular delegated target. Reset restores the concrete
  /// native element constructor's delegation behavior.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub delegates_focus: Prop<bool>,
  /// USS classes applied to this element in list order.
  ///
  /// Class names are matched by `.class-name` selectors. Empty or duplicate
  /// entries are rejected by [`validate_documents`](crate::validate_documents).
  /// Reset removes every Battlement-authored class while retaining native
  /// constructor classes.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub classes: Prop<Vec<String>>,
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
  /// Reset removes every shorthand subscription.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub events: Prop<Vec<crate::UiEventKind>>,
  /// Event subscriptions with explicit logical route phases. Reset removes
  /// every routed subscription.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub event_subscriptions: Prop<Vec<crate::UiEventSubscription>>,
}

impl UiVisualElement {
  /// Creates an unstyled, enabled-by-default structural element.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  pub(crate) fn apply_update(&mut self, value: &Self) {
    if !value.name.is_unset() {
      self.name = value.name.clone();
    }
    if !value.enabled.is_unset() {
      self.enabled = value.enabled;
    }
    if !value.picking_mode.is_unset() {
      self.picking_mode = value.picking_mode;
    }
    if !value.language_direction.is_unset() {
      self.language_direction = value.language_direction;
    }
    if !value.focusable.is_unset() {
      self.focusable = value.focusable;
    }
    if !value.tab_index.is_unset() {
      self.tab_index = value.tab_index;
    }
    if !value.delegates_focus.is_unset() {
      self.delegates_focus = value.delegates_focus;
    }
    if !value.classes.is_unset() {
      self.classes = value.classes.clone();
    }
    self.style = self.style.clone().merge(value.style.clone());
    if !value.events.is_unset() {
      self.events = value.events.clone();
    }
    if !value.event_subscriptions.is_unset() {
      self.event_subscriptions = value.event_subscriptions.clone();
    }
  }
}

impl UiVisualElementProperties for UiVisualElement {
  fn visual_element(&self) -> &UiVisualElement {
    self
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    self
  }
}
