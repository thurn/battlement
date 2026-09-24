//! Native batch start dependencies and the UI references they must order.

use std::collections::HashSet;

use battlement::{BatchStart, CommandBody, ObjectId, UiNode, VisualElementUpdate};
use battlement_ui_fake::UiWorld;

use crate::presentation::ScheduledBatch;

/// Whether `later` cannot start until `earlier` has finished, matching Unity admission.
pub(crate) fn depends_on(later: &ScheduledBatch, earlier: &ScheduledBatch) -> bool {
  match later.start {
    BatchStart::Now => false,
    BatchStart::AfterEarlierBlockingWork => {
      later.scope.is_none() || earlier.scope == later.scope || earlier.prepares_assets
    }
    BatchStart::AfterEarlierAssetPreparation => {
      earlier.prepares_assets || earlier.start == BatchStart::AfterEarlierAssetPreparation
    }
  }
}

/// Panics when `later` references UI that an unfinished, unordered batch creates or destroys.
///
/// Native hosts may run such batches in either order depending on frame timing, so the
/// reference is a latent missing-element failure even when this fake happens to order it.
pub(crate) fn assert_ordered(earlier: &[ScheduledBatch], later: &ScheduledBatch, ui: &UiWorld) {
  if later.cancellation {
    return;
  }
  let references = self::references(later);
  for batch in earlier {
    if batch.cancellation || self::depends_on(later, batch) {
      continue;
    }
    let mut changed = HashSet::new();
    for command in batch.groups.iter().flat_map(|group| &group.commands) {
      match &command.body {
        CommandBody::VisualElementCreate(value) => self::collect_node(&value.node, &mut changed),
        CommandBody::VisualElementDestroy(value) => {
          self::collect_live(ui, value.object_id, &mut changed);
        }
        _ => (),
      }
    }
    if let Some(id) = references.iter().find(|id| changed.contains(id)) {
      panic!(
        "batch {} references UI element {id}, which unordered batch {} still creates or \
         destroys; native hosts may run them in either order",
        later.batch_id, batch.batch_id
      );
    }
  }
}

fn references(batch: &ScheduledBatch) -> Vec<ObjectId> {
  let mut created = HashSet::new();
  let mut result = Vec::new();
  for command in batch.groups.iter().flat_map(|group| &group.commands) {
    match &command.body {
      CommandBody::VisualElementCreate(value) => {
        result.push(value.parent_id);
        self::collect_node(&value.node, &mut created);
      }
      CommandBody::VisualElementUpdate(value) => {
        result.push(value.object_id());
        if let VisualElementUpdate::Parent { parent_id, .. } = value.as_ref() {
          result.push(*parent_id);
        }
      }
      CommandBody::VisualElementDestroy(value) => result.push(value.object_id),
      CommandBody::VisualElementPerformAction(value) => result.push(value.object_id),
      _ => (),
    }
  }
  result.retain(|id| !created.contains(id));
  result
}

fn collect_node(node: &UiNode, result: &mut HashSet<ObjectId>) {
  result.insert(node.object_id);
  for child in &node.children {
    self::collect_node(child, result);
  }
}

fn collect_live(ui: &UiWorld, id: ObjectId, result: &mut HashSet<ObjectId>) {
  result.insert(id);
  if let Some(element) = ui.element(id) {
    for child in element.children() {
      self::collect_live(ui, *child, result);
    }
  }
}
