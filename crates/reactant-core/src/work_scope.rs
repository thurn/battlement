//! Command ownership follows affected hosts through ordinary reconciliations.

use std::collections::HashMap;

use battlement::{
  Batch, BatchId, BatchStart, Command, CommandBody, GameObject, ObjectId, ParallelCommandGroup,
  SessionId, Snapshot, UiNode, VisualElementCreate,
};

use crate::{host_node::HostNode, render::RenderTree};

/// Application-owned native work lifetime inherited by descendant hosts.
#[doc(hidden)]
#[derive(Clone, Copy, PartialEq)]
pub struct WorkScope(pub u64);

pub(crate) fn collect(
  tree: &RenderTree,
  inherited: Option<u64>,
  result: &mut HashMap<ObjectId, u64>,
) {
  for position in &tree.positions {
    let scope = position
      .provider
      .as_ref()
      .and_then(|provider| provider.get::<WorkScope>())
      .map(|scope| scope.0)
      .or(inherited)
      .filter(|scope| *scope != 0);
    if let (Some(host), Some(scope)) = (&position.host, scope) {
      result.insert(host.object_id, scope);
    }
    if let Some(suspense) = &position.suspense {
      self::collect(&suspense.primary, scope, result);
    }
    self::collect(&position.children, scope, result);
  }
}

pub(crate) fn batches(
  session: SessionId,
  groups: Vec<Vec<Command>>,
  owners: &HashMap<ObjectId, u64>,
  independent: bool,
) -> Vec<Batch> {
  let mut batches: Vec<Batch> = Vec::new();
  for group in groups {
    let mut split: Vec<(Option<u64>, Vec<Command>)> = Vec::new();
    for command in group {
      let scope = self::target(&command.body)
        .and_then(|id| owners.get(&id).copied())
        .or_else(|| {
          matches!(
            command.body,
            CommandBody::AccessibilityUpdate(_) | CommandBody::GeometryObservationUpdate(_)
          )
          .then(|| owners.values().copied().max())
          .flatten()
        });
      if let Some((_, commands)) = split.iter_mut().find(|(owner, _)| *owner == scope) {
        commands.push(command);
      } else {
        split.push((scope, vec![command]));
      }
    }
    for (scope, commands) in split {
      let index = batches
        .iter()
        .position(|batch| batch.work_scope == scope)
        .unwrap_or_else(|| {
          let mut batch = Batch::new(BatchId::new_v4(), session, Vec::new());
          batch.work_scope = scope;
          batch.start = if independent && scope.is_none() {
            BatchStart::AfterEarlierAssetPreparation
          } else {
            BatchStart::AfterEarlierBlockingWork
          };
          batches.push(batch);
          batches.len() - 1
        });
      batches[index]
        .groups
        .push(ParallelCommandGroup::new(commands));
    }
  }
  batches
}

pub(crate) fn target(command: &CommandBody) -> Option<ObjectId> {
  match command {
    CommandBody::ObjectCreate(value) => Some(value.object.object_id),
    CommandBody::ObjectDestroy(value) => Some(value.object_id),
    CommandBody::ObjectSetRenderOrder(value) => Some(value.object_id),
    CommandBody::BoxHitRegionSetGeometry(value) => Some(value.object_id),
    CommandBody::RendererSetInstances(value) => Some(value.object_id),
    CommandBody::ObjectSetActive(value) => Some(value.object_id),
    CommandBody::ObjectReparent(value) => Some(value.object_id),
    CommandBody::TransformSetLocalPosition(value) => Some(value.payload.object_id),
    CommandBody::TransformSetLocalRotation(value) => Some(value.payload.object_id),
    CommandBody::TransformSetLocalScale(value) => Some(value.payload.object_id),
    CommandBody::InputSetPointerEvents(value) => Some(value.object_id),
    CommandBody::VisualElementCreate(value) => Some(value.node.object_id),
    CommandBody::VisualElementUpdate(value) => Some(value.object_id()),
    CommandBody::VisualElementDestroy(value) => Some(value.object_id),
    CommandBody::VisualElementPerformAction(value) => Some(value.object_id),
    CommandBody::MotionValue(value) => Some(value.value_id),
    CommandBody::MotionValuePlayback(value) => Some(value.playback_id),
    CommandBody::MotionPlayback(value) => Some(value.descriptor_id),
    CommandBody::MotionControlledClock(value) => Some(value.clock_id),
    CommandBody::MotionControl(value) => Some(value.control_id),
    CommandBody::MotionScope(value) => Some(value.scope_id),
    CommandBody::MotionDragControl(value) => Some(value.control_id),
    _ => None,
  }
}

pub(crate) fn extract_snapshot(
  snapshot: &mut Snapshot,
  owners: &HashMap<ObjectId, u64>,
) -> Vec<Batch> {
  let mut commands = Vec::new();
  for document in &mut snapshot.ui {
    self::extract_nodes(
      &mut document.children,
      document.root_id,
      owners,
      &mut commands,
    );
  }
  let mut objects = Vec::new();
  snapshot.objects.retain(|object| {
    if owners.contains_key(&object.object_id) {
      objects.push(object.clone());
      false
    } else {
      true
    }
  });
  let mut groups = self::object_groups(objects);
  if !commands.is_empty() {
    groups.insert(0, commands);
  }
  self::batches(snapshot.session_id, groups, owners, true)
}

fn extract_nodes(
  nodes: &mut Vec<UiNode>,
  parent: ObjectId,
  owners: &HashMap<ObjectId, u64>,
  commands: &mut Vec<Command>,
) {
  *nodes = std::mem::take(nodes)
    .into_iter()
    .enumerate()
    .filter_map(|(index, mut node)| {
      if owners.contains_key(&node.object_id) {
        commands.push(Command::new_v4(CommandBody::VisualElementCreate(Box::new(
          VisualElementCreate::new(parent, node).child_index(index as u32),
        ))));
        None
      } else {
        self::extract_nodes(&mut node.children, node.object_id, owners, commands);
        Some(node)
      }
    })
    .collect();
}

pub(crate) fn recovery(
  hosts: &[HostNode],
  parent: ObjectId,
  scope: u64,
  owners: &HashMap<ObjectId, u64>,
  commands: &mut Vec<Command>,
) {
  for (index, node) in hosts.iter().enumerate() {
    if owners.get(&node.object_id) == Some(&scope) {
      commands.push(node.create_command(parent, index as u32));
    } else {
      self::recovery(&node.children, node.object_id, scope, owners, commands);
    }
  }
}

pub(crate) fn current() -> Option<u64> {
  crate::context::read_optional::<WorkScope>().map(|scope| scope.0)
}

pub(crate) fn object_groups(objects: Vec<GameObject>) -> Vec<Vec<Command>> {
  let mut depths = HashMap::new();
  let mut groups = Vec::<Vec<Command>>::new();
  for object in objects {
    let depth = object
      .parent_id
      .and_then(|parent| depths.get(&parent))
      .map_or(0, |depth| depth + 1);
    depths.insert(object.object_id, depth);
    groups.resize_with(groups.len().max(depth + 1), Vec::new);
    groups[depth].push(Command::new_v4(CommandBody::object_create(object)));
  }
  groups
}
