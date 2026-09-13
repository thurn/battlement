use std::collections::{HashMap, HashSet};

use battlement::{
  self, Command, CommandBody, ObjectId, UiElement, UiElementKind, UiNode,
  UiVisualElementProperties, VisualElementCreate,
};

use crate::mutation;

#[cfg(test)]
pub(crate) fn commands(
  parent_id: ObjectId,
  previous: &[UiNode],
  desired: &[UiNode],
) -> Vec<Command> {
  self::command_groups(parent_id, previous, desired)
    .into_iter()
    .flatten()
    .map(Command::new_v4)
    .collect()
}

pub(crate) fn command_groups(
  parent_id: ObjectId,
  previous: &[UiNode],
  desired: &[UiNode],
) -> Vec<Vec<CommandBody>> {
  let previous = TreeIndex::new(parent_id, previous);
  let desired = TreeIndex::new(parent_id, desired);
  let mut plan = Plan::default();
  let mut current = previous.children.clone();
  let mut current_parents = previous.parents.clone();
  self::plan_reparents(
    parent_id,
    &previous,
    &desired,
    &mut current,
    &mut current_parents,
    &mut plan,
  );
  self::plan_removals(parent_id, &previous, &desired, &mut current, &mut plan);
  self::reconcile_children(parent_id, &previous, &desired, &mut current, &mut plan);
  let constrained_parents = previous
    .nodes
    .iter()
    .chain(&desired.nodes)
    .filter_map(|(object_id, node)| {
      (node.element.kind() == UiElementKind::ToggleButtonGroup).then_some(*object_id)
    })
    .collect();
  mutation::lower(
    plan
      .reparents
      .into_iter()
      .chain(plan.destroys)
      .chain(plan.placements)
      .chain(plan.properties)
      .collect(),
    &previous.parents,
    &previous.preorder,
    &desired.preorder,
    &constrained_parents,
  )
}

pub(crate) fn requires_remount(previous: &UiElement, desired: &UiElement) -> bool {
  if previous.kind() != desired.kind() {
    return true;
  }
  if previous.visual_element().usage_hints != desired.visual_element().usage_hints {
    return true;
  }
  let Some(patch) = self::property_patch(previous, desired, false) else {
    return false;
  };
  let mut merged = previous.clone();
  merged.apply_update(&patch);
  battlement::validate_element_state(&merged).is_err()
}

#[derive(Default)]
struct Plan {
  reparents: Vec<Command>,
  destroys: Vec<Command>,
  placements: Vec<Command>,
  properties: Vec<Command>,
}

struct TreeIndex<'a> {
  nodes: HashMap<ObjectId, &'a UiNode>,
  parents: HashMap<ObjectId, ObjectId>,
  children: HashMap<ObjectId, Vec<ObjectId>>,
  preorder: Vec<ObjectId>,
}

impl<'a> TreeIndex<'a> {
  fn new(root_id: ObjectId, nodes: &'a [UiNode]) -> Self {
    let mut result = Self {
      nodes: HashMap::new(),
      parents: HashMap::new(),
      children: HashMap::new(),
      preorder: Vec::new(),
    };
    self::collect_tree(root_id, nodes, &mut result);
    result
  }
}

fn plan_reparents(
  root_id: ObjectId,
  previous: &TreeIndex<'_>,
  desired: &TreeIndex<'_>,
  current: &mut HashMap<ObjectId, Vec<ObjectId>>,
  current_parents: &mut HashMap<ObjectId, ObjectId>,
  plan: &mut Plan,
) {
  let mut pending = desired
    .preorder
    .iter()
    .filter(|object_id| {
      previous
        .parents
        .get(object_id)
        .is_some_and(|parent_id| *parent_id != desired.parents[object_id])
    })
    .copied()
    .collect::<Vec<_>>();
  while !pending.is_empty() {
    let ready = pending
      .iter()
      .position(|object_id| self::reparent_is_ready(*object_id, previous, desired, current));
    let Some(ready) = ready else {
      self::stage_capacity_blocked_reparent(
        root_id,
        &pending,
        previous,
        desired,
        current,
        current_parents,
        plan,
      );
      continue;
    };
    let object_id = pending.remove(ready);
    let previous_parent = current_parents[&object_id];
    let desired_parent = desired.parents[&object_id];
    let siblings = &desired.children[&desired_parent];
    let desired_index = siblings
      .iter()
      .position(|candidate| *candidate == object_id)
      .expect("desired parent contains its child");
    let anchor = siblings[desired_index + 1..]
      .iter()
      .find(|candidate| current[&desired_parent].contains(candidate))
      .copied();
    let child_index = self::anchor_index(&current[&desired_parent], anchor);
    plan.reparents.push(Command::move_visual_element(
      object_id,
      desired_parent,
      child_index,
    ));
    self::remove_child(current, previous_parent, object_id);
    current
      .get_mut(&desired_parent)
      .expect("reused physical parent has a child sequence")
      .insert(child_index as usize, object_id);
    current_parents.insert(object_id, desired_parent);
  }
}

fn stage_capacity_blocked_reparent(
  root_id: ObjectId,
  pending: &[ObjectId],
  previous: &TreeIndex<'_>,
  desired: &TreeIndex<'_>,
  current: &mut HashMap<ObjectId, Vec<ObjectId>>,
  current_parents: &mut HashMap<ObjectId, ObjectId>,
  plan: &mut Plan,
) {
  let object_id = pending
    .iter()
    .find(|object_id| {
      let parent_id = current_parents[object_id];
      parent_id != root_id && self::is_toggle_group(parent_id, previous, desired)
    })
    .copied()
    .expect("Reactant cannot order the requested host reparents");
  let previous_parent = current_parents[&object_id];
  let child_index = u32::try_from(current[&root_id].len()).expect("validated child index fits u32");
  plan.reparents.push(Command::move_visual_element(
    object_id,
    root_id,
    child_index,
  ));
  self::remove_child(current, previous_parent, object_id);
  current
    .get_mut(&root_id)
    .expect("document root has a child sequence")
    .push(object_id);
  current_parents.insert(object_id, root_id);
}

fn reparent_is_ready(
  object_id: ObjectId,
  previous: &TreeIndex<'_>,
  desired: &TreeIndex<'_>,
  current: &HashMap<ObjectId, Vec<ObjectId>>,
) -> bool {
  let parent_id = desired.parents[&object_id];
  assert!(
    previous.children.contains_key(&parent_id),
    "a reused host cannot move beneath a new host"
  );
  if self::is_descendant(current, object_id, parent_id) {
    return false;
  }
  !self::is_toggle_group(parent_id, previous, desired) || current[&parent_id].len() < 64
}

fn is_toggle_group(object_id: ObjectId, previous: &TreeIndex<'_>, desired: &TreeIndex<'_>) -> bool {
  desired
    .nodes
    .get(&object_id)
    .or_else(|| previous.nodes.get(&object_id))
    .is_some_and(|node| node.element.kind() == UiElementKind::ToggleButtonGroup)
}

fn is_descendant(
  current: &HashMap<ObjectId, Vec<ObjectId>>,
  ancestor_id: ObjectId,
  candidate_id: ObjectId,
) -> bool {
  current[&ancestor_id].iter().any(|child_id| {
    *child_id == candidate_id || self::is_descendant(current, *child_id, candidate_id)
  })
}

fn plan_removals(
  parent_id: ObjectId,
  previous: &TreeIndex<'_>,
  desired: &TreeIndex<'_>,
  current: &mut HashMap<ObjectId, Vec<ObjectId>>,
  plan: &mut Plan,
) {
  for object_id in &previous.children[&parent_id] {
    if !desired.nodes.contains_key(object_id) {
      self::remove_child(current, parent_id, *object_id);
      plan
        .destroys
        .push(Command::destroy_visual_element(*object_id));
      continue;
    }
    self::plan_removals(*object_id, previous, desired, current, plan);
  }
}

fn reconcile_children(
  parent_id: ObjectId,
  previous: &TreeIndex<'_>,
  desired: &TreeIndex<'_>,
  current: &mut HashMap<ObjectId, Vec<ObjectId>>,
  plan: &mut Plan,
) {
  let desired_children = &desired.children[&parent_id];
  let retained = self::retained_subsequence(parent_id, desired_children, previous);
  let mut anchor = None;
  for object_id in desired_children.iter().rev() {
    let child = desired.nodes[object_id];
    if !previous.nodes.contains_key(object_id) {
      let index = self::anchor_index(&current[&parent_id], anchor);
      let create = VisualElementCreate::new(parent_id, child.clone()).child_index(index);
      plan
        .placements
        .push(Command::new_v4(CommandBody::VisualElementCreate(Box::new(
          create,
        ))));
      current
        .get_mut(&parent_id)
        .expect("physical parent has a child sequence")
        .insert(index as usize, *object_id);
      anchor = Some(*object_id);
      continue;
    }
    if !retained.contains(object_id)
      && let Some(index) = self::place_before(current, parent_id, *object_id, anchor)
    {
      plan
        .placements
        .push(Command::update_visual_element_index(*object_id, index));
    }
    anchor = Some(*object_id);
  }
  assert_eq!(
    current[&parent_id], *desired_children,
    "Reactant hierarchy planning did not reach the desired child sequence"
  );
  for object_id in desired_children {
    let Some(previous_child) = previous.nodes.get(object_id) else {
      continue;
    };
    let child = desired.nodes[object_id];
    self::reconcile_children(*object_id, previous, desired, current, plan);
    let hierarchy_changed = previous.children[object_id] != desired.children[object_id];
    if let Some(patch) =
      self::property_patch(&previous_child.element, &child.element, hierarchy_changed)
    {
      battlement::validate_element_update(&patch)
        .expect("Reactant generated an invalid property patch");
      plan
        .properties
        .push(Command::update_visual_element(*object_id, patch));
    }
  }
}

fn retained_subsequence(
  parent_id: ObjectId,
  desired_children: &[ObjectId],
  previous: &TreeIndex<'_>,
) -> HashSet<ObjectId> {
  let candidates = desired_children
    .iter()
    .filter(|object_id| previous.parents.get(object_id) == Some(&parent_id))
    .map(|object_id| {
      (
        *object_id,
        previous.children[&parent_id]
          .iter()
          .position(|candidate| candidate == object_id)
          .expect("indexed child is present in its parent"),
      )
    })
    .collect::<Vec<_>>();
  let mut suffix_lengths = vec![1; candidates.len()];
  for index in (0..candidates.len()).rev() {
    for later in index + 1..candidates.len() {
      if candidates[later].1 > candidates[index].1 {
        suffix_lengths[index] = suffix_lengths[index].max(suffix_lengths[later] + 1);
      }
    }
  }
  let mut remaining = suffix_lengths.iter().copied().max().unwrap_or(0);
  let mut last_index = None;
  let mut result = HashSet::new();
  for (index, (object_id, old_index)) in candidates.iter().enumerate() {
    let increasing = last_index.is_none_or(|last| *old_index > last);
    if increasing && suffix_lengths[index] >= remaining {
      result.insert(*object_id);
      last_index = Some(*old_index);
      remaining -= 1;
    }
  }
  result
}

fn place_before(
  current: &mut HashMap<ObjectId, Vec<ObjectId>>,
  parent_id: ObjectId,
  object_id: ObjectId,
  anchor: Option<ObjectId>,
) -> Option<u32> {
  let children = current
    .get_mut(&parent_id)
    .expect("physical parent has a child sequence");
  let previous_index = children
    .iter()
    .position(|candidate| *candidate == object_id)
    .expect("reused child is present beneath its desired parent");
  children.remove(previous_index);
  let index = self::anchor_index(children, anchor);
  children.insert(index as usize, object_id);
  (previous_index != index as usize).then_some(index)
}

fn anchor_index(children: &[ObjectId], anchor: Option<ObjectId>) -> u32 {
  u32::try_from(anchor.map_or(children.len(), |object_id| {
    children
      .iter()
      .position(|candidate| *candidate == object_id)
      .expect("placement anchor is present")
  }))
  .expect("validated child index fits u32")
}

fn remove_child(
  current: &mut HashMap<ObjectId, Vec<ObjectId>>,
  parent_id: ObjectId,
  object_id: ObjectId,
) {
  current
    .get_mut(&parent_id)
    .expect("physical parent has a child sequence")
    .retain(|candidate| *candidate != object_id);
}

fn property_patch(
  previous: &UiElement,
  desired: &UiElement,
  hierarchy_changed: bool,
) -> Option<UiElement> {
  UiElement::difference(previous, desired, hierarchy_changed)
}

fn collect_tree<'a>(parent_id: ObjectId, nodes: &'a [UiNode], result: &mut TreeIndex<'a>) {
  result
    .children
    .insert(parent_id, nodes.iter().map(|node| node.object_id).collect());
  for node in nodes {
    result.nodes.insert(node.object_id, node);
    result.parents.insert(node.object_id, parent_id);
    result.preorder.push(node.object_id);
    self::collect_tree(node.object_id, &node.children, result);
  }
}
