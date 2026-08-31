use serde::{Deserialize, Serialize};

use crate::{
  LanguageDirection, PickingMode, Prop, Style, UiVisualElement, UiVisualElementProperties,
  UsageHint,
  elements::parts::{self, Part, PartStyle},
};

/// A controlled group that presents logical [`UiButton`] children as toggles.
///
/// Use this control when each choice benefits from a button-like label or icon.
/// By default the group selects one button and does not allow an empty
/// selection. [`Self::multiple_selection`] permits several selected buttons;
/// [`Self::allow_empty_selection`] permits none. [`Self::selected_indices`]
/// addresses direct children by their zero-based visual order.
///
/// Selection gestures produce [`UiEventKind::ValueCommitted`] proposals. Rust
/// remains authoritative until an update sends the accepted indices. Selected
/// indices must be unique, sorted, and within the direct-child list. Only
/// ordinary [`UiButton`] nodes are valid logical children.
///
/// See Unity's [ToggleButtonGroup manual](https://docs.unity3d.com/6000.5/Documentation/Manual/UIE-uxml-element-ToggleButtonGroup.html)
/// for single, multiple, and empty-selection behavior.
///
/// # Example
///
/// ```
/// use battlement_types::ObjectId;
/// use battlement_ui::{UiButton, UiToggleButtonGroup, UiEventKind, UiNode};
///
/// let alignment = UiNode::new(
///     ObjectId::new_v4(),
///     UiToggleButtonGroup::new()
///         .label("Alignment")
///         .selected_indices([0])
///         .events([UiEventKind::ValueCommitted]),
/// )
/// .children([
///     UiNode::new(ObjectId::new_v4(), UiButton::new("Left")),
///     UiNode::new(ObjectId::new_v4(), UiButton::new("Center")),
///     UiNode::new(ObjectId::new_v4(), UiButton::new("Right")),
/// ]);
///
/// assert_eq!(alignment.children.len(), 3);
/// ```
///
/// [`UiButton`]: crate::UiButton
/// [`UiEventKind::ValueCommitted`]: crate::UiEventKind::ValueCommitted
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct UiToggleButtonGroup {
  /// Shared visual properties, inline style, and event subscriptions.
  #[serde(flatten)]
  pub element: UiVisualElement,
  /// Caption associated with the complete field.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub label: Prop<String>,
  /// Whether more than one button may be selected.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub multiple_selection: Prop<bool>,
  /// Whether a nonempty group may have no selected button.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub allow_empty_selection: Prop<bool>,
  /// Unique sorted zero-based indices authored as selected by Rust.
  #[serde(default, skip_serializing_if = "Prop::is_unset")]
  pub selected_indices: Prop<Vec<u32>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub(crate) parts: Option<Vec<PartStyle>>,
}

impl UiToggleButtonGroup {
  /// Creates a single-selection group using its first child by default.
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  impl_common_visual_element_methods!();

  /// Applies sparse inline declarations to the native `ToggleButtonGroupLabel` part.
  #[must_use]
  pub fn label_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ToggleButtonGroupLabel, value);
    self
  }

  /// Applies sparse inline declarations to the native `ToggleButtonGroupInput` part.
  #[must_use]
  pub fn input_style(mut self, value: Style) -> Self {
    parts::append(&mut self.parts, Part::ToggleButtonGroupInput, value);
    self
  }

  /// Sets the field caption.
  #[must_use]
  pub fn label(mut self, value: impl Into<Prop<String>>) -> Self {
    self.label = value.into();
    self
  }

  /// Enables or disables multiple simultaneous selections.
  #[must_use]
  pub fn multiple_selection(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.multiple_selection = value.into();
    self
  }

  /// Enables or disables an empty selection in a nonempty group.
  #[must_use]
  pub fn allow_empty_selection(mut self, value: impl Into<Prop<bool>>) -> Self {
    self.allow_empty_selection = value.into();
    self
  }

  /// Replaces the unique sorted selected indices.
  #[must_use]
  pub fn selected_indices(mut self, values: impl IntoIterator<Item = u32>) -> Self {
    self.selected_indices = Prop::Set(values.into_iter().collect());
    self
  }

  /// Replaces or resets the unique sorted selected indices.
  #[must_use]
  pub fn selected_indices_value(mut self, value: impl Into<Prop<Vec<u32>>>) -> Self {
    self.selected_indices = value.into();
    self
  }

  pub(crate) fn apply_update(&mut self, value: &Self) {
    self.element.apply_update(&value.element);
    if !matches!(value.label, Prop::Unset) {
      self.label.clone_from(&value.label);
    }
    if !matches!(value.multiple_selection, Prop::Unset) {
      self.multiple_selection = value.multiple_selection;
    }
    if !matches!(value.allow_empty_selection, Prop::Unset) {
      self.allow_empty_selection = value.allow_empty_selection;
    }
    if !matches!(value.selected_indices, Prop::Unset) {
      self.selected_indices.clone_from(&value.selected_indices);
    }
    parts::merge(&mut self.parts, &value.parts);
  }
}

impl UiVisualElementProperties for UiToggleButtonGroup {
  fn visual_element(&self) -> &UiVisualElement {
    &self.element
  }

  fn visual_element_mut(&mut self) -> &mut UiVisualElement {
    &mut self.element
  }
}
