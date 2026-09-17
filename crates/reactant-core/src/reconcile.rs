use std::collections::{HashMap, HashSet};

#[cfg(test)]
use battlement::{Command, UiNode};
use battlement::{CommandBody, ObjectId};

#[cfg(test)]
use crate::ui_host_adapter;
use crate::{
  host_node::HostNode,
  mutation::{self, PlannedMutation},
};

#[cfg(test)]
pub(crate) fn commands(
  parent_id: ObjectId,
  previous: &[UiNode],
  desired: &[UiNode],
) -> Vec<Command> {
  self::command_groups(
    parent_id,
    &ui_host_adapter::from_ui_nodes(previous),
    &ui_host_adapter::from_ui_nodes(desired),
  )
  .into_iter()
  .flatten()
  .map(Command::new_v4)
  .collect()
}

#[cfg(test)]
pub(crate) fn ui_command_groups(
  parent_id: ObjectId,
  previous: &[UiNode],
  desired: &[UiNode],
) -> Vec<Vec<CommandBody>> {
  self::command_groups(
    parent_id,
    &ui_host_adapter::from_ui_nodes(previous),
    &ui_host_adapter::from_ui_nodes(desired),
  )
}

pub(crate) fn command_groups(
  parent_id: ObjectId,
  previous: &[HostNode],
  desired: &[HostNode],
) -> Vec<Vec<CommandBody>> {
  self::forest_command_groups(&[(parent_id, previous, desired)])
}

pub(crate) fn forest_command_groups(
  roots: &[(ObjectId, &[HostNode], &[HostNode])],
) -> Vec<Vec<CommandBody>> {
  let previous = TreeIndex::forest(roots.iter().map(|(id, previous, _)| (*id, *previous)));
  let desired = TreeIndex::forest(roots.iter().map(|(id, _, desired)| (*id, *desired)));
  let mut plan = Plan::default();
  let mut current = previous.children.clone();
  let mut current_parents = previous.parents.clone();
  self::prepare_new_parents(
    roots[0].0,
    &previous,
    &desired,
    &mut current,
    &mut current_parents,
    &mut plan,
  );
  self::plan_reparents(
    roots[0].0,
    &previous,
    &desired,
    &mut current,
    &mut current_parents,
    &mut plan,
  );
  for (parent_id, _, _) in roots {
    self::plan_removals(
      *parent_id,
      &previous,
      &desired,
      &mut current,
      &current_parents,
      &mut plan,
    );
  }
  for (parent_id, _, _) in roots {
    self::reconcile_children(*parent_id, &previous, &desired, &mut current, &mut plan);
  }
  let constrained_parents = previous
    .nodes
    .iter()
    .chain(&desired.nodes)
    .filter_map(|(object_id, node)| node.constrains_children().then_some(*object_id))
    .collect();
  mutation::lower(
    plan
      .preparations
      .into_iter()
      .chain(plan.reparents)
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

#[derive(Default)]
struct Plan {
  preparations: Vec<PlannedMutation>,
  reparents: Vec<PlannedMutation>,
  destroys: Vec<PlannedMutation>,
  placements: Vec<PlannedMutation>,
  properties: Vec<PlannedMutation>,
}

struct TreeIndex<'a> {
  nodes: HashMap<ObjectId, &'a HostNode>,
  parents: HashMap<ObjectId, ObjectId>,
  children: HashMap<ObjectId, Vec<ObjectId>>,
  preorder: Vec<ObjectId>,
}

impl<'a> TreeIndex<'a> {
  fn forest(roots: impl IntoIterator<Item = (ObjectId, &'a [HostNode])>) -> Self {
    let mut result = Self {
      nodes: HashMap::new(),
      parents: HashMap::new(),
      children: HashMap::new(),
      preorder: Vec::new(),
    };
    for (root_id, nodes) in roots {
      self::collect_tree(root_id, nodes, &mut result);
    }
    result
  }
}

fn prepare_new_parents(
  root_id: ObjectId,
  previous: &TreeIndex<'_>,
  desired: &TreeIndex<'_>,
  current: &mut HashMap<ObjectId, Vec<ObjectId>>,
  parents: &mut HashMap<ObjectId, ObjectId>,
  plan: &mut Plan,
) {
  let mut needed = HashSet::new();
  for id in &desired.preorder {
    if !previous.nodes.contains_key(id) {
      continue;
    }
    let mut parent = desired.parents[id];
    while !previous.children.contains_key(&parent) {
      if !needed.insert(parent) {
        break;
      }
      parent = desired.parents[&parent];
    }
  }
  for id in &desired.preorder {
    if !needed.contains(id) {
      continue;
    }
    let parent = desired.parents[id];
    if self::is_toggle_group(parent, previous, desired) && current[&parent].len() == 64 {
      let departing = *current[&parent]
        .iter()
        .find(|child| desired.parents.get(child) != Some(&parent))
        .expect("a new child of a full choice group replaces a departing child");
      let staging_index = u32::try_from(current[&root_id].len()).expect("child index fits u32");
      plan.preparations.push(PlannedMutation::move_host(
        previous.nodes[&departing].move_command(root_id, staging_index),
        departing,
        parent,
        root_id,
      ));
      self::remove_child(current, parent, departing);
      current
        .get_mut(&root_id)
        .expect("staging root exists")
        .push(departing);
      parents.insert(departing, root_id);
    }
    let index = u32::try_from(current[&parent].len()).expect("child index fits u32");
    let node = desired.nodes[id].without_children();
    plan.preparations.push(PlannedMutation::create(
      node.create_command(parent, index),
      &node,
      parent,
    ));
    current
      .get_mut(&parent)
      .expect("prepared parent exists")
      .push(*id);
    current.insert(*id, Vec::new());
    parents.insert(*id, parent);
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
    let command = desired.nodes[&object_id].move_command(desired_parent, child_index);
    plan.reparents.push(PlannedMutation::move_host(
      command,
      object_id,
      previous_parent,
      desired_parent,
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
  let command = previous.nodes[&object_id].move_command(root_id, child_index);
  plan.reparents.push(PlannedMutation::move_host(
    command,
    object_id,
    previous_parent,
    root_id,
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
    current.contains_key(&parent_id),
    "a reused host requires a prepared parent"
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
    .is_some_and(|node| node.constrains_children())
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
  parents: &HashMap<ObjectId, ObjectId>,
  plan: &mut Plan,
) {
  for object_id in &previous.children[&parent_id] {
    if !desired.nodes.contains_key(object_id)
      && (desired.nodes.contains_key(&parent_id) || !previous.nodes.contains_key(&parent_id))
    {
      let current_parent = parents[object_id];
      self::remove_child(current, current_parent, *object_id);
      plan.destroys.push(PlannedMutation::destroy(
        previous.nodes[object_id].destroy_command(),
        *object_id,
        current_parent,
      ));
    }
    self::plan_removals(*object_id, previous, desired, current, parents, plan);
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
    if !current.contains_key(object_id) {
      let index = self::anchor_index(&current[&parent_id], anchor);
      self::create_subtree(child, parent_id, index, &mut plan.placements);
      current
        .get_mut(&parent_id)
        .expect("physical parent has a child sequence")
        .insert(index as usize, *object_id);
      anchor = Some(*object_id);
      continue;
    }
    if !retained.contains(object_id)
      && let Some(index) = self::place_before(current, parent_id, *object_id, anchor)
      && let Some(command) = child.index_command(index)
    {
      plan.placements.push(PlannedMutation::move_host(
        command, *object_id, parent_id, parent_id,
      ));
    }
    anchor = Some(*object_id);
  }
  assert_eq!(
    current[&parent_id], *desired_children,
    "Reactant hierarchy planning did not reach the desired child sequence"
  );
  for object_id in desired_children {
    if !current.contains_key(object_id) {
      continue;
    }
    self::reconcile_children(*object_id, previous, desired, current, plan);
    let Some(previous_child) = previous.nodes.get(object_id) else {
      continue;
    };
    let child = desired.nodes[object_id];
    let hierarchy_changed = previous.children[object_id] != desired.children[object_id];
    for command in previous_child.property_commands(child, hierarchy_changed) {
      plan
        .properties
        .push(PlannedMutation::properties(command, *object_id));
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

fn collect_tree<'a>(parent_id: ObjectId, nodes: &'a [HostNode], result: &mut TreeIndex<'a>) {
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

fn create_subtree(
  node: &HostNode,
  parent: ObjectId,
  index: u32,
  mutations: &mut Vec<PlannedMutation>,
) {
  if node.creates_children() {
    mutations.push(PlannedMutation::create(
      node.create_command(parent, index),
      node,
      parent,
    ));
  } else {
    mutations.push(PlannedMutation::create(
      node.create_command(parent, index),
      &node.without_children(),
      parent,
    ));
    for (index, child) in node.children.iter().enumerate() {
      self::create_subtree(child, node.object_id, index as u32, mutations);
    }
  }
}
