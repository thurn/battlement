//! Battlement UI implementation of the shared Reactant host boundary.

use battlement::{
  self, Command, CommandBody, ObjectId, UiElement, UiElementKind, UiNode,
  UiVisualElementProperties, VisualElementCreate,
};

use crate::host_node::{HostAdapter, HostNode};

pub(crate) struct UiHostAdapter;

impl HostAdapter for UiHostAdapter {
  type Description = UiElement;

  fn requires_remount(previous: &UiElement, desired: &UiElement) -> bool {
    if previous.kind() != desired.kind() {
      return true;
    }
    if previous.visual_element().usage_hints != desired.visual_element().usage_hints {
      return true;
    }
    let Some(patch) = UiElement::difference(previous, desired, false) else {
      return false;
    };
    let mut merged = previous.clone();
    merged.apply_update(&patch);
    battlement::validate_element_state(&merged).is_err()
  }

  fn create_command(node: &HostNode, parent_id: ObjectId, child_index: u32) -> Command {
    let create = VisualElementCreate::new(parent_id, to_ui_node(node)).child_index(child_index);
    Command::new_v4(CommandBody::VisualElementCreate(Box::new(create)))
  }

  fn property_command(
    object_id: ObjectId,
    previous: &UiElement,
    desired: &UiElement,
    hierarchy_changed: bool,
  ) -> Option<Command> {
    let patch = UiElement::difference(previous, desired, hierarchy_changed)?;
    battlement::validate_element_update(&patch)
      .expect("Reactant generated an invalid property patch");
    Some(Command::update_visual_element(object_id, patch))
  }

  fn move_command(object_id: ObjectId, parent_id: ObjectId, child_index: u32) -> Command {
    Command::move_visual_element(object_id, parent_id, child_index)
  }

  fn index_command(object_id: ObjectId, child_index: u32) -> Command {
    Command::update_visual_element_index(object_id, child_index)
  }

  fn destroy_command(object_id: ObjectId) -> Command {
    Command::destroy_visual_element(object_id)
  }

  fn constrains_children(description: &UiElement) -> bool {
    description.kind() == UiElementKind::ToggleButtonGroup
  }
}

pub(crate) fn node(object_id: ObjectId, element: UiElement) -> HostNode {
  HostNode::new::<UiHostAdapter>(object_id, element)
}

pub(crate) fn from_ui_node(value: &UiNode) -> HostNode {
  let mut result = node(value.object_id, value.element.clone());
  result.children = value.children.iter().map(from_ui_node).collect();
  result
}

pub(crate) fn from_ui_nodes(values: &[UiNode]) -> Vec<HostNode> {
  values.iter().map(from_ui_node).collect()
}

pub(crate) fn to_ui_node(value: &HostNode) -> UiNode {
  UiNode::new(value.object_id, element(value).clone())
    .children(value.children.iter().map(to_ui_node))
}

pub(crate) fn to_ui_nodes(values: &[HostNode]) -> Vec<UiNode> {
  values.iter().map(to_ui_node).collect()
}

pub(crate) fn element(value: &HostNode) -> &UiElement {
  value.description::<UiHostAdapter>()
}

pub(crate) fn element_mut(value: &mut HostNode) -> &mut UiElement {
  value.description_mut::<UiHostAdapter>()
}
