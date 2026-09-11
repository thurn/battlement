use std::collections::HashSet;

use battlement_types::ObjectId;

use crate::{
  Prop, StyleValue, UiDocument, UiElement, UiElementKind, UiNode, UiVisualElementProperties,
};

use super::{
  MAXIMUM_HIERARCHY_DEPTH, MAXIMUM_IDENTITIES, UiValidationError, insert_identity,
  validate_element, validate_visual,
};

/// The placement facts available while validating a node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlacementContext {
  /// The node is a direct child of a validated document root.
  DocumentRoot,
  /// The node is attached below a live parent with a known element kind.
  Attached { parent_kind: UiElementKind },
  /// The node is the root of a detached subtree awaiting insertion.
  DetachedRoot,
}

impl PlacementContext {
  const fn parent_kind(self) -> Option<UiElementKind> {
    match self {
      Self::DocumentRoot | Self::DetachedRoot => None,
      Self::Attached { parent_kind } => Some(parent_kind),
    }
  }

  const fn defers_live_parent_checks(self) -> bool {
    matches!(self, Self::DetachedRoot)
  }
}

/// Facts accumulated while walking one document or detached subtree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TraversalContext {
  depth: usize,
  has_scroll_ancestor: bool,
}

impl TraversalContext {
  const fn document_child() -> Self {
    Self {
      depth: 1,
      has_scroll_ancestor: false,
    }
  }

  const fn detached_root() -> Self {
    Self {
      depth: 0,
      has_scroll_ancestor: false,
    }
  }

  fn child(self, kind: UiElementKind) -> Self {
    Self {
      depth: self.depth + 1,
      has_scroll_ancestor: self.has_scroll_ancestor || kind == UiElementKind::ScrollView,
    }
  }
}

/// Validates complete UI document trees and returns all reserved identities.
///
/// The returned set contains each document host ID, document root ID, and node
/// ID. Validation rejects duplicate identities across the complete collection;
/// empty or duplicate USS classes; duplicate event subscriptions; nonfinite
/// style numbers or colors; leaf controls with children; more than 100,000
/// children on one node; and hierarchy depth beyond 256 edges.
///
/// # Errors
///
/// Returns the first [`UiValidationError`] encountered in document and child
/// order. No input value is modified.
pub fn validate_documents(
  documents: &[UiDocument],
) -> Result<HashSet<ObjectId>, UiValidationError> {
  let mut identities = HashSet::new();
  for document in documents {
    insert_identity(&mut identities, document.document_id)?;
    insert_identity(&mut identities, document.root_id)?;
    validate_document_root(document)?;
    for child in &document.children {
      validate_node(
        child,
        &mut identities,
        TraversalContext::document_child(),
        PlacementContext::DocumentRoot,
      )?;
    }
  }
  if documents.iter().map(auto_focus_count).sum::<usize>() > 1 {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(identities)
}

fn validate_document_root(document: &UiDocument) -> Result<(), UiValidationError> {
  if document.element.usage_hints.is_some() {
    return Err(UiValidationError::InvalidProperty);
  }
  validate_visual(&document.element)
}

fn auto_focus_count(document: &UiDocument) -> usize {
  usize::from(matches!(document.element.auto_focus, Prop::Set(true)))
    + document
      .children
      .iter()
      .map(node_auto_focus_count)
      .sum::<usize>()
}

fn node_auto_focus_count(node: &UiNode) -> usize {
  usize::from(matches!(
    node.element.visual_element().auto_focus,
    Prop::Set(true)
  )) + node
    .children
    .iter()
    .map(node_auto_focus_count)
    .sum::<usize>()
}

/// Validates a detached element subtree before a create command is executed.
///
/// The returned identities are unique within the subtree. Live-session identity
/// conflicts and the final depth beneath the selected parent must be checked by
/// the executor because they depend on current client state.
///
/// # Errors
///
/// Returns the first [`UiValidationError`] in preorder without modifying the
/// subtree.
pub fn validate_create_subtree(node: &UiNode) -> Result<HashSet<ObjectId>, UiValidationError> {
  let mut identities = HashSet::new();
  validate_node(
    node,
    &mut identities,
    TraversalContext::detached_root(),
    PlacementContext::DetachedRoot,
  )?;
  Ok(identities)
}

fn validate_node(
  node: &UiNode,
  identities: &mut HashSet<ObjectId>,
  traversal: TraversalContext,
  placement: PlacementContext,
) -> Result<(), UiValidationError> {
  insert_identity(identities, node.object_id)?;
  if traversal.depth > MAXIMUM_HIERARCHY_DEPTH || node.children.len() > MAXIMUM_IDENTITIES {
    return Err(UiValidationError::InvalidHierarchy);
  }
  let accepts_children = match &node.element {
    UiElement::VisualElement(_)
    | UiElement::Flex(_)
    | UiElement::Grid(_)
    | UiElement::Stack(_)
    | UiElement::Box(_)
    | UiElement::Button(_)
    | UiElement::GroupBox(_)
    | UiElement::PopupWindow(_)
    | UiElement::ScrollView(_)
    | UiElement::Tab(_)
    | UiElement::TabView(_)
    | UiElement::ToggleButtonGroup(_) => true,
    UiElement::Label(_)
    | UiElement::TextElement(_)
    | UiElement::TextField(_)
    | UiElement::Toggle(_)
    | UiElement::RadioButton(_)
    | UiElement::RadioButtonGroup(_)
    | UiElement::DropdownField(_)
    | UiElement::RepeatButton(_)
    | UiElement::Image(_)
    | UiElement::Scroller(_)
    | UiElement::Slider(_)
    | UiElement::SliderInt(_)
    | UiElement::MinMaxSlider(_)
    | UiElement::ProgressBar(_) => false,
  };
  if !accepts_children && !node.children.is_empty() {
    return Err(UiValidationError::InvalidHierarchy);
  }

  let kind = node.element.kind();
  validate_placement(&node.element, placement)?;
  validate_contextual_hierarchy(&node.element, kind, placement, traversal)?;
  if let UiElement::ToggleButtonGroup(value) = &node.element {
    validate_toggle_button_group(value, node.children.len())?;
  }
  if let UiElement::TabView(value) = &node.element
    && matches!(value.selected_tab_index, Prop::Set(index) if index as usize >= node.children.len())
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if let UiElement::RepeatButton(value) = &node.element
    && (!matches!(value.delay_ms, Prop::Set(_)) || !matches!(value.interval_ms, Prop::Set(_)))
  {
    return Err(UiValidationError::InvalidProperty);
  }
  validate_element(&node.element, super::ValidationMode::Complete)?;
  if let Some(descriptor) = node.element.visual_element().motion.set_value()
    && descriptor.host_id != node.object_id
  {
    return Err(UiValidationError::InvalidReference);
  }
  let child_placement = PlacementContext::Attached { parent_kind: kind };
  for child in &node.children {
    validate_node(child, identities, traversal.child(kind), child_placement)?;
  }
  Ok(())
}

fn validate_placement(
  element: &UiElement,
  placement: PlacementContext,
) -> Result<(), UiValidationError> {
  let visual = element.visual_element();
  let parent_kind = placement.parent_kind();
  let in_grid = parent_kind == Some(UiElementKind::Grid);
  let in_stack = parent_kind == Some(UiElementKind::Stack);
  if matches!(visual.grid_item, Prop::Set(_)) && !in_grid && !placement.defers_live_parent_checks()
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if matches!(visual.stack_item, Prop::Set(_))
    && !in_stack
    && !placement.defers_live_parent_checks()
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if in_grid || in_stack {
    validate_grid_or_stack_style(visual, placement)?;
  }
  Ok(())
}

fn validate_grid_or_stack_style(
  visual: &crate::UiVisualElement,
  placement: PlacementContext,
) -> Result<(), UiValidationError> {
  if placement.defers_live_parent_checks() {
    return Ok(());
  }
  let position_is_absolute = matches!(
    visual.style.position,
    Prop::Set(StyleValue::Value(crate::Position::Absolute))
  );
  let offsets_are_automatic = [
    &visual.style.top,
    &visual.style.right,
    &visual.style.bottom,
    &visual.style.left,
  ]
  .into_iter()
  .all(layout_offset_is_automatic);
  if position_is_absolute || !offsets_are_automatic {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn layout_offset_is_automatic(value: &Prop<StyleValue<crate::LengthOrAuto>>) -> bool {
  matches!(
    value,
    Prop::Unset
      | Prop::Reset
      | Prop::Set(StyleValue::Value(crate::LengthOrAuto::Auto))
      | Prop::Set(StyleValue::Keyword { .. })
  )
}

fn validate_contextual_hierarchy(
  element: &UiElement,
  kind: UiElementKind,
  placement: PlacementContext,
  traversal: TraversalContext,
) -> Result<(), UiValidationError> {
  let parent_kind = placement.parent_kind();
  if matches!(element.visual_element().sticky, Prop::Set(_))
    && !placement.defers_live_parent_checks()
    && !traversal.has_scroll_ancestor
  {
    return Err(UiValidationError::InvalidProperty);
  }
  if parent_kind == Some(UiElementKind::TabView) && kind != UiElementKind::Tab {
    return Err(UiValidationError::InvalidHierarchy);
  }
  if kind == UiElementKind::Tab
    && parent_kind != Some(UiElementKind::TabView)
    && !placement.defers_live_parent_checks()
  {
    return Err(UiValidationError::InvalidHierarchy);
  }
  if parent_kind == Some(UiElementKind::ToggleButtonGroup) && kind != UiElementKind::Button {
    return Err(UiValidationError::InvalidHierarchy);
  }
  Ok(())
}

fn validate_toggle_button_group(
  value: &crate::UiToggleButtonGroup,
  child_count: usize,
) -> Result<(), UiValidationError> {
  if child_count > 64 {
    return Err(UiValidationError::InvalidHierarchy);
  }
  let default_selected = [0];
  let selected: &[u32] = match value.selected_indices.set_value() {
    Some(values) => values,
    None if child_count == 0 || matches!(value.allow_empty_selection, Prop::Set(true)) => &[],
    None => &default_selected,
  };
  validate_selected_indices(selected)?;
  if selected.iter().any(|index| *index as usize >= child_count) {
    return Err(UiValidationError::InvalidProperty);
  }
  if !matches!(value.multiple_selection, Prop::Set(true)) && selected.len() > 1 {
    return Err(UiValidationError::InvalidProperty);
  }
  if child_count > 0
    && !matches!(value.allow_empty_selection, Prop::Set(true))
    && selected.is_empty()
  {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}

fn validate_selected_indices(values: &[u32]) -> Result<(), UiValidationError> {
  if values.windows(2).any(|pair| pair[0] >= pair[1]) {
    return Err(UiValidationError::InvalidProperty);
  }
  Ok(())
}
