//! Prepared native descriptions without live component state or callbacks.

use std::{collections::HashSet, rc::Rc};

use battlement::ObjectId;
use uuid::Uuid;

use crate::{
  hook_storage::HookOwner,
  host_node::HostNode,
  motion_lifecycle::MotionCallbacks,
  render::{RenderPosition, RenderTree},
  work_scope::WorkScope,
};

pub(crate) struct RetainedResources {
  pub(crate) hosts: Vec<HostNode>,
}

pub(crate) fn resources(position: &RenderPosition) -> Rc<RetainedResources> {
  let mut hosts = Vec::new();
  self::collect_resources(position, &mut hosts);
  Rc::new(RetainedResources { hosts })
}

pub(crate) fn freeze(position: &mut RenderPosition) {
  if let Some(host) = position.host.as_mut().filter(|host| !host.is_ui()) {
    host.inert();
  }
  position.handlers.clear();
  position.motion_callbacks = MotionCallbacks::default();
  position.motion_callback_history.clear();
  position.component = None;
  position.component_source = None;
  position.component_scope = None;
  position.memo_value = None;
  if position
    .provider
    .as_ref()
    .is_some_and(|provider| provider.get::<WorkScope>().is_none())
  {
    position.provider = None;
  }
  position.error_boundary = None;
  position.element_ref = None;
  position.drag_constraint_ref = None;
  position.overlay_reference = None;
  position.semantic = None;
  position.retained_render = None;
  position.exit_blueprint = None;
  position.presence = None;
  if let Some(suspense) = position.suspense.take() {
    for mut primary in suspense.primary.positions {
      primary.hidden = suspense.showing_fallback;
      position.children.positions.push(primary);
    }
  }
  for child in &mut position.children.positions {
    self::freeze(child);
  }
}

/// Extract moved live declarations from terminal visuals before native planning.
pub(crate) fn prepare(trees: &mut [RenderTree]) {
  if !trees.iter().any(self::has_terminal) {
    return;
  }
  let mut ids = HashSet::new();
  let mut hosts = HashSet::new();
  let mut owners = Vec::new();
  for tree in trees.iter() {
    self::collect_live(tree, &mut ids, &mut hosts);
    tree.hook_owners(&mut owners);
  }
  for tree in trees {
    self::prune(tree, &ids, &hosts, &owners);
  }
}

fn has_terminal(tree: &RenderTree) -> bool {
  tree.positions.iter().any(|position| {
    position.terminal_visual
      || self::has_terminal(&position.children)
      || position
        .suspense
        .as_ref()
        .is_some_and(|state| self::has_terminal(&state.primary))
  })
}

fn collect_resources(position: &RenderPosition, hosts: &mut Vec<HostNode>) {
  if let Some(host) = &position.host {
    hosts.push(host.without_children());
  }
  for child in &position.children.positions {
    self::collect_resources(child, hosts);
  }
}

fn collect_live(tree: &RenderTree, ids: &mut HashSet<Uuid>, hosts: &mut HashSet<ObjectId>) {
  for position in &tree.positions {
    if position.terminal_visual {
      continue;
    }
    if let Some(id) = position.presentation_id {
      ids.insert(id);
    }
    if let Some(host) = &position.host {
      hosts.insert(host.object_id);
    }
    if let Some(suspense) = &position.suspense {
      self::collect_live(&suspense.primary, ids, hosts);
    }
    self::collect_live(&position.children, ids, hosts);
  }
}

fn prune(
  tree: &mut RenderTree,
  ids: &HashSet<Uuid>,
  hosts: &HashSet<ObjectId>,
  owners: &[Rc<HookOwner>],
) {
  tree.positions.retain_mut(|position| {
    if position.terminal_visual {
      return self::prune_visual(position, ids, hosts);
    }
    if let Some(suspense) = &mut position.suspense {
      self::prune(&mut suspense.primary, ids, hosts, owners);
    }
    self::prune(&mut position.children, ids, hosts, owners);
    if let Some(presence) = &mut position.presence {
      presence.exits.retain_mut(|exit| {
        let retained = position
          .children
          .positions
          .iter()
          .find(|child| child.terminal_visual && child.key.as_ref() == Some(&exit.key));
        let Some(retained) = retained else {
          return exit.ready();
        };
        exit.resources = self::resources(retained);
        exit.automatic.retain(|automatic| {
          exit
            .resources
            .hosts
            .iter()
            .any(|host| host.object_id == automatic.descriptor_id)
        });
        exit
          .holds
          .retain(|(_, owner)| !owners.iter().any(|live| owner.same(live)));
        true
      });
    }
    true
  });
}

fn prune_visual(
  position: &mut RenderPosition,
  ids: &HashSet<Uuid>,
  hosts: &HashSet<ObjectId>,
) -> bool {
  if position.presentation_id.is_some_and(|id| ids.contains(&id)) {
    return false;
  }
  if position
    .host
    .as_ref()
    .is_some_and(|host| hosts.contains(&host.object_id))
  {
    return false;
  }
  position
    .children
    .positions
    .retain_mut(|child| self::prune_visual(child, ids, hosts));
  position.host.is_some() || !position.children.positions.is_empty()
}

pub(crate) fn collect_host_ids(position: &RenderPosition, hosts: &mut HashSet<ObjectId>) {
  if let Some(host) = &position.host {
    hosts.insert(host.object_id);
  }
  for child in &position.children.positions {
    self::collect_host_ids(child, hosts);
  }
}
