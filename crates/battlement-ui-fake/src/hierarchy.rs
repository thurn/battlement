use std::collections::HashSet;

use battlement_types::ObjectId;
use battlement_ui::{UiElementKind, UiNode};

use crate::UiWorldError;

pub(crate) fn require_container(kind: UiElementKind) -> Result<(), UiWorldError> {
  if matches!(
    kind,
    UiElementKind::Label
      | UiElementKind::TextElement
      | UiElementKind::RepeatButton
      | UiElementKind::Scroller
      | UiElementKind::Slider
      | UiElementKind::SliderInt
      | UiElementKind::MinMaxSlider
      | UiElementKind::ProgressBar
      | UiElementKind::TextField
      | UiElementKind::Toggle
      | UiElementKind::RadioButton
      | UiElementKind::RadioButtonGroup
      | UiElementKind::Image
  ) {
    Err(UiWorldError::InvalidHierarchy)
  } else {
    Ok(())
  }
}

pub(crate) fn require_placement(
  child: UiElementKind,
  parent: UiElementKind,
) -> Result<(), UiWorldError> {
  if (child == UiElementKind::Tab) != (parent == UiElementKind::TabView) {
    return Err(UiWorldError::InvalidHierarchy);
  }
  if parent == UiElementKind::ToggleButtonGroup && child != UiElementKind::Button {
    return Err(UiWorldError::InvalidHierarchy);
  }
  Ok(())
}

pub(crate) fn collect_ids(
  node: &UiNode,
  identities: &mut HashSet<ObjectId>,
) -> Result<(), UiWorldError> {
  if !identities.insert(node.object_id) {
    return Err(UiWorldError::DuplicateObject);
  }
  self::require_container(node.element.kind()).or_else(|error| {
    if node.children.is_empty() {
      Ok(())
    } else {
      Err(error)
    }
  })?;
  for child in &node.children {
    self::collect_ids(child, identities)?;
  }
  Ok(())
}

pub(crate) fn subtree_depth(node: &UiNode) -> usize {
  node
    .children
    .iter()
    .map(|child| self::subtree_depth(child) + 1)
    .max()
    .unwrap_or(0)
}
