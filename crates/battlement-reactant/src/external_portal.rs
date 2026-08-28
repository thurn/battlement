//! Caller-owned portal containers and their session bindings.

use std::collections::{HashMap, HashSet};

use battlement::{
  self, CommandBody, ObjectId, Snapshot, UiDocument, UiElementKind, UiNode, Validate,
};

use crate::{
  portal::{PortalLayout, PortalRoot, PortalTarget},
  reconcile,
};

pub(crate) struct ExternalPortalRegistry {
  targets: Vec<ExternalTarget>,
  caller_ui: Vec<UiDocument>,
}

pub(crate) struct SessionExternal {
  bindings: Vec<(PortalTarget, ObjectId)>,
  roots: HashMap<PortalTarget, PortalRoot>,
}

pub(crate) struct PreparedExternal {
  pub(crate) groups: Vec<Vec<CommandBody>>,
  bindings: Vec<CommittedBinding>,
  caller_ui: Vec<UiDocument>,
}

struct ExternalTarget {
  target: PortalTarget,
  current_id: ObjectId,
  staged_id: Option<ObjectId>,
  prefix: Vec<UiNode>,
}

struct CommittedBinding {
  target: PortalTarget,
  id: ObjectId,
  prefix: Vec<UiNode>,
}

impl ExternalPortalRegistry {
  pub(crate) const fn new() -> Self {
    Self {
      targets: Vec::new(),
      caller_ui: Vec::new(),
    }
  }

  pub(crate) fn register(&mut self, target: PortalTarget, id: ObjectId) {
    assert!(
      self
        .targets
        .iter()
        .all(|registered| registered.current_id != id),
      "two external Reactant portal targets cannot share a container"
    );
    self.targets.push(ExternalTarget {
      target,
      current_id: id,
      staged_id: None,
      prefix: Vec::new(),
    });
  }

  pub(crate) fn stage(&mut self, runtime_id: u64, target: &PortalTarget, id: ObjectId) {
    assert!(
      target.belongs_to(runtime_id),
      "Reactant portal target belongs to another runtime"
    );
    self
      .targets
      .iter_mut()
      .find(|registered| registered.target == *target)
      .expect("only registered external portal targets can be rebound")
      .staged_id = Some(id);
  }

  pub(crate) fn active_bindings(&self) -> Vec<(PortalTarget, ObjectId)> {
    self
      .targets
      .iter()
      .map(|target| (target.target.clone(), target.current_id))
      .collect()
  }

  pub(crate) fn session_bindings(&self) -> Vec<(PortalTarget, ObjectId)> {
    let bindings = self
      .targets
      .iter()
      .map(|target| {
        (
          target.target.clone(),
          target.staged_id.unwrap_or(target.current_id),
        )
      })
      .collect::<Vec<_>>();
    assert!(
      bindings
        .iter()
        .map(|(_, id)| *id)
        .collect::<HashSet<_>>()
        .len()
        == bindings.len(),
      "two external Reactant portal targets cannot share a container"
    );
    bindings
  }

  pub(crate) fn active_groups(
    &self,
    previous: &PortalLayout,
    desired: &PortalLayout,
    documents: &[UiDocument],
  ) -> Vec<Vec<CommandBody>> {
    let mut prospective = self.caller_ui.clone();
    prospective.extend(documents.iter().cloned());
    for target in &self.targets {
      self::find_children_mut(&mut prospective, target.current_id)
        .expect("committed external portal target remains in caller UI")
        .extend(desired.externals[&target.target].hosts.iter().cloned());
    }
    battlement::validate_documents(&prospective)
      .expect("Reactant rendered an invalid external portal hierarchy");
    self.targets.iter().fold(Vec::new(), |groups, target| {
      let previous_hosts = previous
        .externals
        .get(&target.target)
        .map_or(&[][..], |root| root.hosts.as_slice());
      let desired_hosts = desired
        .externals
        .get(&target.target)
        .map_or(&[][..], |root| root.hosts.as_slice());
      self::merge_groups(
        groups,
        reconcile::command_groups(
          target.current_id,
          &self::with_prefix(&target.prefix, previous_hosts),
          &self::with_prefix(&target.prefix, desired_hosts),
        ),
      )
    })
  }

  pub(crate) fn commit(&mut self, prepared: PreparedExternal) -> Vec<Vec<CommandBody>> {
    self.caller_ui = prepared.caller_ui;
    for binding in prepared.bindings {
      let target = self
        .targets
        .iter_mut()
        .find(|target| target.target == binding.target)
        .expect("prepared external portal target is registered");
      target.current_id = binding.id;
      target.staged_id = None;
      target.prefix = binding.prefix;
    }
    prepared.groups
  }
}

impl SessionExternal {
  pub(crate) fn new(
    bindings: Vec<(PortalTarget, ObjectId)>,
    roots: HashMap<PortalTarget, PortalRoot>,
  ) -> Self {
    Self { bindings, roots }
  }

  pub(crate) fn prepare(
    self,
    snapshot: &mut Snapshot,
    documents: &[UiDocument],
  ) -> PreparedExternal {
    let caller_ui = snapshot.ui.clone();
    let prefixes = self
      .bindings
      .iter()
      .map(|(_, id)| {
        self::find_children(&snapshot.ui, *id)
          .cloned()
          .unwrap_or_else(|| panic!("external Reactant portal target is missing from snapshot"))
      })
      .collect::<Vec<_>>();
    snapshot.ui.extend(documents.iter().cloned());
    let mut prospective = snapshot.clone();
    for ((target, id), prefix) in self.bindings.iter().zip(&prefixes) {
      let hosts = &self
        .roots
        .get(target)
        .expect("external portal layout contains every registered target")
        .hosts;
      self::find_children_mut(&mut prospective.ui, *id)
        .expect("validated external portal target remains in snapshot")
        .extend(hosts.iter().cloned());
      debug_assert_eq!(
        &self::find_children(&snapshot.ui, *id).expect("external target exists")[..prefix.len()],
        prefix
      );
    }
    if let Err(error) = prospective.validate() {
      panic!("Reactant session snapshot is invalid: {error}");
    }
    let groups =
      self
        .bindings
        .iter()
        .zip(&prefixes)
        .fold(Vec::new(), |groups, ((target, id), prefix)| {
          let hosts = &self.roots[target].hosts;
          self::merge_groups(
            groups,
            reconcile::command_groups(*id, prefix, &self::with_prefix(prefix, hosts)),
          )
        });
    PreparedExternal {
      groups,
      caller_ui,
      bindings: self
        .bindings
        .into_iter()
        .zip(prefixes)
        .map(|((target, id), prefix)| CommittedBinding { target, id, prefix })
        .collect(),
    }
  }
}

fn with_prefix(prefix: &[UiNode], hosts: &[UiNode]) -> Vec<UiNode> {
  prefix.iter().chain(hosts).cloned().collect()
}

fn find_children(documents: &[UiDocument], id: ObjectId) -> Option<&Vec<UiNode>> {
  for document in documents {
    if document.root_id == id {
      return Some(&document.children);
    }
    if let Some(children) = self::find_node_children(&document.children, id) {
      return Some(children);
    }
  }
  None
}

fn find_node_children(nodes: &[UiNode], id: ObjectId) -> Option<&Vec<UiNode>> {
  for node in nodes {
    if node.object_id == id {
      assert!(
        self::is_container(node.element.kind()),
        "external Reactant portal target must be a container"
      );
      return Some(&node.children);
    }
    if let Some(children) = self::find_node_children(&node.children, id) {
      return Some(children);
    }
  }
  None
}

fn find_children_mut(documents: &mut [UiDocument], id: ObjectId) -> Option<&mut Vec<UiNode>> {
  for document in documents {
    if document.root_id == id {
      return Some(&mut document.children);
    }
    if let Some(children) = self::find_node_children_mut(&mut document.children, id) {
      return Some(children);
    }
  }
  None
}

fn find_node_children_mut(nodes: &mut [UiNode], id: ObjectId) -> Option<&mut Vec<UiNode>> {
  for node in nodes {
    if node.object_id == id {
      assert!(
        self::is_container(node.element.kind()),
        "external Reactant portal target must be a container"
      );
      return Some(&mut node.children);
    }
    if let Some(children) = self::find_node_children_mut(&mut node.children, id) {
      return Some(children);
    }
  }
  None
}

fn is_container(kind: UiElementKind) -> bool {
  matches!(
    kind,
    UiElementKind::VisualElement
      | UiElementKind::Box
      | UiElementKind::ToggleButtonGroup
      | UiElementKind::GroupBox
      | UiElementKind::PopupWindow
      | UiElementKind::ScrollView
      | UiElementKind::Tab
      | UiElementKind::TabView
  )
}

fn merge_groups(
  mut merged: Vec<Vec<CommandBody>>,
  groups: Vec<Vec<CommandBody>>,
) -> Vec<Vec<CommandBody>> {
  for (index, group) in groups.into_iter().enumerate() {
    if index == merged.len() {
      merged.push(group);
    } else {
      merged[index].extend(group);
    }
  }
  merged
}
