use std::collections::{HashMap, HashSet};

use battlement::{Command, CommandBody, ObjectId, UiNode, VisualElementUpdate};

pub(crate) fn lower(
  commands: Vec<Command>,
  previous_parents: &HashMap<ObjectId, ObjectId>,
  previous_preorder: &[ObjectId],
  desired_preorder: &[ObjectId],
  constrained_parents: &HashSet<ObjectId>,
) -> Vec<Vec<CommandBody>> {
  let previous_ordinals = self::ordinals(previous_preorder);
  let desired_ordinals = self::ordinals(desired_preorder);
  let mut current_parents = previous_parents.clone();
  let mut mutations = commands
    .into_iter()
    .enumerate()
    .map(|(sequence, command)| {
      Mutation::new(
        sequence,
        command.body,
        &mut current_parents,
        &previous_ordinals,
        &desired_ordinals,
      )
    })
    .collect::<Vec<_>>();
  for index in 0..mutations.len() {
    let dependencies = mutations[..index]
      .iter()
      .filter(|earlier| {
        self::requires_dependency(
          earlier,
          &mutations[index],
          previous_parents,
          constrained_parents,
        )
      })
      .map(|earlier| earlier.sequence)
      .collect();
    mutations[index].dependencies = dependencies;
  }
  self::groups(mutations)
}

struct Mutation {
  sequence: usize,
  ordinal: (usize, MutationKind, ObjectId),
  kind: MutationKind,
  target: ObjectId,
  old_parent: Option<ObjectId>,
  new_parent: Option<ObjectId>,
  created: HashSet<ObjectId>,
  body: CommandBody,
  conflicts: HashSet<ObjectId>,
  dependencies: Vec<usize>,
}

impl Mutation {
  fn new(
    sequence: usize,
    body: CommandBody,
    current_parents: &mut HashMap<ObjectId, ObjectId>,
    previous_ordinals: &HashMap<ObjectId, usize>,
    desired_ordinals: &HashMap<ObjectId, usize>,
  ) -> Self {
    let (target, kind, old_parent, new_parent, created, conflicts) = match &body {
      CommandBody::VisualElementCreate(value) => {
        let mut conflicts = HashSet::from([value.parent_id]);
        self::collect_created(
          &value.node,
          value.parent_id,
          current_parents,
          &mut conflicts,
        );
        let mut created = conflicts.clone();
        created.remove(&value.parent_id);
        (
          value.node.object_id,
          MutationKind::Create,
          None,
          Some(value.parent_id),
          created,
          conflicts,
        )
      }
      CommandBody::VisualElementUpdate(value) => match value.as_ref() {
        VisualElementUpdate::Properties { object_id, .. } => (
          *object_id,
          MutationKind::Properties,
          None,
          None,
          HashSet::new(),
          HashSet::from([*object_id]),
        ),
        VisualElementUpdate::Parent {
          object_id,
          parent_id,
          ..
        } => {
          let old_parent = current_parents[object_id];
          current_parents.insert(*object_id, *parent_id);
          (
            *object_id,
            MutationKind::Move,
            Some(old_parent),
            Some(*parent_id),
            HashSet::new(),
            HashSet::from([*object_id, old_parent, *parent_id]),
          )
        }
        VisualElementUpdate::Index { object_id, .. } => {
          let parent_id = current_parents[object_id];
          (
            *object_id,
            MutationKind::Move,
            Some(parent_id),
            Some(parent_id),
            HashSet::new(),
            HashSet::from([*object_id, parent_id]),
          )
        }
      },
      CommandBody::VisualElementDestroy(value) => {
        let mut conflicts = HashSet::from([value.object_id]);
        let old_parent = current_parents.remove(&value.object_id);
        if let Some(parent_id) = old_parent {
          conflicts.insert(parent_id);
        }
        (
          value.object_id,
          MutationKind::Destroy,
          old_parent,
          None,
          HashSet::new(),
          conflicts,
        )
      }
      _ => panic!("Reactant reconciliation emitted a non-UI mutation"),
    };
    let preorder = desired_ordinals
      .get(&target)
      .or_else(|| previous_ordinals.get(&target))
      .copied()
      .expect("Reactant mutation target has a preorder ordinal");
    Self {
      sequence,
      ordinal: (preorder, kind, target),
      kind,
      target,
      old_parent,
      new_parent,
      created,
      body,
      conflicts,
      dependencies: Vec::new(),
    }
  }
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum MutationKind {
  Create,
  Move,
  Properties,
  Destroy,
}

fn ordinals(preorder: &[ObjectId]) -> HashMap<ObjectId, usize> {
  preorder
    .iter()
    .enumerate()
    .map(|(index, object_id)| (*object_id, index))
    .collect()
}

fn collect_created(
  node: &UiNode,
  parent_id: ObjectId,
  current_parents: &mut HashMap<ObjectId, ObjectId>,
  conflicts: &mut HashSet<ObjectId>,
) {
  conflicts.insert(node.object_id);
  current_parents.insert(node.object_id, parent_id);
  for child in &node.children {
    self::collect_created(child, node.object_id, current_parents, conflicts);
  }
}

fn requires_dependency(
  earlier: &Mutation,
  mutation: &Mutation,
  previous_parents: &HashMap<ObjectId, ObjectId>,
  constrained_parents: &HashSet<ObjectId>,
) -> bool {
  if earlier.kind == MutationKind::Create {
    let parent_created = mutation
      .new_parent
      .is_some_and(|parent_id| earlier.created.contains(&parent_id));
    if parent_created || earlier.created.contains(&mutation.target) {
      return true;
    }
  }
  if earlier.kind == MutationKind::Move
    && mutation.kind == MutationKind::Move
    && earlier.target == mutation.target
  {
    return true;
  }
  if mutation.kind == MutationKind::Properties && self::affects_child_list(earlier, mutation.target)
  {
    return true;
  }
  if self::same_placement_parent(earlier, mutation) {
    return true;
  }
  if self::departure_frees_constrained_parent(earlier, mutation, constrained_parents) {
    return true;
  }
  if mutation.kind != MutationKind::Destroy {
    return false;
  }
  if !matches!(earlier.kind, MutationKind::Move | MutationKind::Destroy) {
    return false;
  }
  self::is_ancestor(mutation.target, earlier.target, previous_parents)
}

fn affects_child_list(mutation: &Mutation, parent_id: ObjectId) -> bool {
  if mutation.kind == MutationKind::Destroy {
    return mutation.old_parent == Some(parent_id);
  }
  if !matches!(mutation.kind, MutationKind::Create | MutationKind::Move) {
    return false;
  }
  mutation.old_parent == Some(parent_id) || mutation.new_parent == Some(parent_id)
}

fn same_placement_parent(earlier: &Mutation, mutation: &Mutation) -> bool {
  if !matches!(earlier.kind, MutationKind::Create | MutationKind::Move) {
    return false;
  }
  if !matches!(mutation.kind, MutationKind::Create | MutationKind::Move) {
    return false;
  }
  earlier.new_parent == mutation.new_parent
}

fn departure_frees_constrained_parent(
  earlier: &Mutation,
  mutation: &Mutation,
  constrained_parents: &HashSet<ObjectId>,
) -> bool {
  if earlier.kind == MutationKind::Destroy {
    return earlier.old_parent == mutation.new_parent;
  }
  if earlier.kind != MutationKind::Move {
    return false;
  }
  let Some(parent_id) = earlier.old_parent else {
    return false;
  };
  constrained_parents.contains(&parent_id) && mutation.new_parent == Some(parent_id)
}

fn is_ancestor(
  ancestor_id: ObjectId,
  mut object_id: ObjectId,
  parents: &HashMap<ObjectId, ObjectId>,
) -> bool {
  while let Some(parent_id) = parents.get(&object_id) {
    if *parent_id == ancestor_id {
      return true;
    }
    object_id = *parent_id;
  }
  false
}

fn groups(mutations: Vec<Mutation>) -> Vec<Vec<CommandBody>> {
  let mut pending = mutations.into_iter().map(Some).collect::<Vec<_>>();
  let mut completed = HashSet::new();
  let mut result = Vec::new();
  while pending.iter().any(Option::is_some) {
    let mut ready = pending
      .iter()
      .enumerate()
      .filter_map(|(index, mutation)| {
        mutation.as_ref().and_then(|mutation| {
          mutation
            .dependencies
            .iter()
            .all(|dependency| completed.contains(dependency))
            .then_some(index)
        })
      })
      .collect::<Vec<_>>();
    ready.sort_by_key(|index| {
      let mutation = pending[*index].as_ref().expect("ready mutation exists");
      (mutation.ordinal, mutation.sequence)
    });
    let mut conflicts = HashSet::new();
    let selected = ready
      .into_iter()
      .filter(|index| {
        let mutation = pending[*index].as_ref().expect("ready mutation exists");
        if !mutation.conflicts.is_disjoint(&conflicts) {
          return false;
        }
        conflicts.extend(mutation.conflicts.iter().copied());
        true
      })
      .collect::<Vec<_>>();
    assert!(
      !selected.is_empty(),
      "Reactant mutation dependencies are cyclic"
    );
    let mut group = Vec::new();
    for index in selected {
      let mutation = pending[index].take().expect("selected mutation exists");
      completed.insert(mutation.sequence);
      group.push(mutation.body);
    }
    result.push(group);
  }
  result
}
