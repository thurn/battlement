//! Physical portal placement with logical Reactant ancestry.

#![allow(private_interfaces)]

use std::{
  any::TypeId,
  collections::{HashMap, HashSet},
};

use battlement::{
  Display, ObjectId, Overflow, PickingMode, Prop, StyleValue, UiElement, UiEventKind, UiEventPhase,
  UiEventSubscription, UiVisualElementProperties,
};

use crate::{
  event_handler::Handler,
  host_node::HostNode,
  render::{Render, RenderSink, RenderTree},
  render_value::Sealed,
  ui_host_adapter,
};

/// Identifies one portal container owned by a Reactant runtime.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PortalTarget {
  runtime_id: u64,
  target_id: u64,
}

/// Renders a logical child beneath a separate physical container.
pub struct Portal<R> {
  child: R,
  target: PortalTarget,
}

/// Creates one logical portal occurrence.
pub fn create_portal<R: Render>(child: R, target: PortalTarget) -> Portal<R> {
  Portal { child, target }
}

impl PortalTarget {
  pub(crate) const fn new(runtime_id: u64, target_id: u64) -> Self {
    Self {
      runtime_id,
      target_id,
    }
  }

  pub(crate) const fn belongs_to(&self, runtime_id: u64) -> bool {
    self.runtime_id == runtime_id
  }
}

impl<R: Render> Render for Portal<R> {}

impl<R: Render> Sealed for Portal<R> {
  fn descriptor(&self) -> TypeId {
    TypeId::of::<PortalMarker>()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.push_portal::<PortalMarker>(self.target.clone(), |children| {
      self.child.render_into(children);
    });
  }

  fn render_owned(self, sink: &mut RenderSink<'_>) {
    sink.push_portal::<PortalMarker>(self.target, |children| {
      self.child.render_owned(children);
    });
  }
}

pub(crate) struct PortalRoot {
  pub(crate) hosts: Vec<HostNode>,
  pub(crate) subscriptions: Vec<UiEventSubscription>,
}

pub(crate) struct PortalLayout {
  pub(crate) attachments: HashMap<PortalTarget, ObjectId>,
  pub(crate) externals: HashMap<PortalTarget, PortalRoot>,
  pub(crate) roots: Vec<PortalRoot>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PortalSourceOrdinal {
  root: usize,
  portal: usize,
}

pub(crate) fn layout(
  runtime_id: u64,
  trees: &[&RenderTree],
  externals: &[(PortalTarget, ObjectId)],
) -> PortalLayout {
  let mut catalog = PortalCatalog::default();
  for (root, tree) in trees.iter().enumerate() {
    let mut portal = 0;
    self::collect_portals(runtime_id, tree, false, root, &mut portal, &mut catalog);
  }
  let external_targets = externals
    .iter()
    .map(|(target, _)| target.clone())
    .collect::<HashSet<_>>();
  for target in &external_targets {
    assert!(
      !catalog.attachments.contains_key(target),
      "an external Reactant portal target cannot attach to a host"
    );
  }
  let attached_ids = catalog
    .attachments
    .values()
    .copied()
    .collect::<HashSet<_>>();
  assert!(
    externals.iter().all(|(_, id)| !attached_ids.contains(id)),
    "two Reactant portal targets resolve to the same container"
  );
  for target in &catalog.referenced {
    assert!(
      catalog.attachments.contains_key(target) || external_targets.contains(target),
      "a referenced Reactant portal target is not attached"
    );
  }
  let roots = trees
    .iter()
    .map(|tree| self::physical_hosts(tree, &catalog.ranges, &mut Vec::new()))
    .collect::<Vec<_>>();
  self::validate_overlay_hosts(&roots, &catalog.attachments);
  let externals = external_targets
    .into_iter()
    .map(|target| {
      let hosts = catalog
        .ranges
        .get(&target)
        .into_iter()
        .flatten()
        .flat_map(|range| {
          let mut hosts = self::physical_hosts(range.tree, &catalog.ranges, &mut Vec::new());
          if range.hidden {
            self::hide_roots(&mut hosts);
          }
          hosts
        })
        .collect::<Vec<_>>();
      (target, self::external_root(hosts, trees))
    })
    .collect::<HashMap<_, _>>();
  let mut physical_hosts = HashSet::new();
  for root in &roots {
    self::collect_unique_host_ids(root, &mut physical_hosts);
  }
  for root in externals.values() {
    self::collect_unique_host_ids(&root.hosts, &mut physical_hosts);
  }
  assert_eq!(
    catalog.logical_hosts, physical_hosts,
    "Reactant portal targets form an unanchored physical cycle"
  );
  let roots = roots
    .into_iter()
    .map(|hosts| PortalRoot {
      subscriptions: self::coverage_subscriptions(&hosts, trees),
      hosts,
    })
    .collect();
  PortalLayout {
    attachments: catalog.attachments,
    externals,
    roots,
  }
}

pub(crate) fn attachment_hosts(
  runtime_id: u64,
  trees: &[&RenderTree],
) -> HashMap<PortalTarget, ObjectId> {
  let mut attachments = HashMap::new();
  for tree in trees {
    self::collect_attachment_hosts(runtime_id, tree, &mut attachments);
  }
  attachments
}

fn collect_attachment_hosts(
  runtime_id: u64,
  tree: &RenderTree,
  attachments: &mut HashMap<PortalTarget, ObjectId>,
) {
  for position in &tree.positions {
    if let Some(target) = &position.portal_target {
      self::validate_target(runtime_id, target);
      assert!(
        attachments
          .insert(target.clone(), position.host_id())
          .is_none(),
        "a Reactant portal target is attached to more than one host"
      );
    }
    if let Some(suspense) = &position.suspense {
      self::collect_attachment_hosts(runtime_id, &suspense.primary, attachments);
    }
    self::collect_attachment_hosts(runtime_id, &position.children, attachments);
  }
}

fn validate_overlay_hosts(roots: &[Vec<HostNode>], attachments: &HashMap<PortalTarget, ObjectId>) {
  let target_ids = attachments.values().copied().collect::<HashSet<_>>();
  for root in roots {
    let configured = self::collect_overlay_hosts(root, &target_ids);
    assert!(
      configured.len() <= 1,
      "one document root cannot contain more than one OverlayHost"
    );
    let Some(host_id) = configured.first() else {
      assert!(
        !self::contains_overlay_wrapper(root),
        "overlay portal content requires an OverlayHost target"
      );
      continue;
    };
    assert!(
      root.len() == 1 && matches!(ui_host_adapter::element(&root[0]), UiElement::Stack(_)),
      "OverlayHost requires one document-root Stack"
    );
    assert_eq!(
      root[0].children.last().map(|child| child.object_id),
      Some(*host_id),
      "OverlayHost must be the final child of its document-root Stack"
    );
    let _ = self::find_host(root, *host_id).expect("validated OverlayHost remains attached");
  }
}

fn collect_overlay_hosts(roots: &[HostNode], target_ids: &HashSet<ObjectId>) -> Vec<ObjectId> {
  let mut values = Vec::new();
  for node in roots {
    if target_ids.contains(&node.object_id) && self::is_overlay_host(node) {
      values.push(node.object_id);
    }
    values.extend(self::collect_overlay_hosts(&node.children, target_ids));
  }
  values
}

fn is_overlay_host(node: &HostNode) -> bool {
  let element = ui_host_adapter::element(node);
  let visual = element.visual_element();
  matches!(element, UiElement::Stack(_))
    && visual.picking_mode == Prop::Set(PickingMode::Ignore)
    && matches!(
      visual.stack_item,
      Prop::Set(value) if value.order == i32::MAX && !value.contributes_to_size
    )
    && matches!(
      visual.style.overflow,
      Prop::Set(StyleValue::Value(Overflow::Visible))
    )
}

fn contains_overlay_wrapper(roots: &[HostNode]) -> bool {
  roots.iter().any(|node| {
    matches!(
      ui_host_adapter::element(node)
        .visual_element()
        .overlay_placement,
      Prop::Set(_)
    ) || self::contains_overlay_wrapper(&node.children)
  })
}

fn find_host(roots: &[HostNode], id: ObjectId) -> Option<&HostNode> {
  for node in roots {
    if node.object_id == id {
      return Some(node);
    }
    if let Some(found) = self::find_host(&node.children, id) {
      return Some(found);
    }
  }
  None
}

pub(crate) fn changed_attachments(
  previous: &PortalLayout,
  desired: &HashMap<PortalTarget, ObjectId>,
) -> HashSet<PortalTarget> {
  previous
    .attachments
    .iter()
    .filter_map(|(target, previous_host)| {
      desired
        .get(target)
        .is_some_and(|desired_host| desired_host != previous_host)
        .then_some(target.clone())
    })
    .collect()
}

#[derive(Default)]
struct PortalCatalog<'a> {
  attachments: HashMap<PortalTarget, ObjectId>,
  logical_hosts: HashSet<ObjectId>,
  object_targets: HashMap<ObjectId, PortalTarget>,
  ranges: HashMap<PortalTarget, Vec<PortalRange<'a>>>,
  referenced: HashSet<PortalTarget>,
}

struct PortalRange<'a> {
  tree: &'a RenderTree,
  hidden: bool,
  source: PortalSourceOrdinal,
}

fn collect_portals<'a>(
  runtime_id: u64,
  tree: &'a RenderTree,
  hidden: bool,
  root: usize,
  portal: &mut usize,
  catalog: &mut PortalCatalog<'a>,
) {
  for position in &tree.positions {
    if let Some(host) = &position.host {
      assert!(
        catalog.logical_hosts.insert(host.object_id),
        "Reactant hosts must have unique IDs"
      );
    }
    if let Some(target) = &position.portal_target {
      self::validate_target(runtime_id, target);
      assert!(
        catalog
          .attachments
          .insert(target.clone(), position.host_id())
          .is_none(),
        "a Reactant portal target is attached to more than one host"
      );
      assert!(
        catalog
          .object_targets
          .insert(position.host_id(), target.clone())
          .is_none(),
        "a Reactant portal host has more than one target"
      );
    }
    if let Some(target) = &position.portal {
      self::validate_target(runtime_id, target);
      catalog.referenced.insert(target.clone());
      let source = PortalSourceOrdinal {
        root,
        portal: *portal,
      };
      *portal = portal
        .checked_add(1)
        .expect("Reactant portal preorder ordinal overflow");
      catalog
        .ranges
        .entry(target.clone())
        .or_default()
        .push(PortalRange {
          tree: &position.children,
          hidden,
          source,
        });
    }
    if let Some(suspense) = &position.suspense {
      self::collect_portals(runtime_id, &suspense.primary, true, root, portal, catalog);
    }
    self::collect_portals(
      runtime_id,
      &position.children,
      hidden,
      root,
      portal,
      catalog,
    );
  }
}

fn validate_target(runtime_id: u64, target: &PortalTarget) {
  assert!(
    target.belongs_to(runtime_id),
    "Reactant portal target belongs to another runtime"
  );
}

fn physical_hosts(
  tree: &RenderTree,
  ranges: &HashMap<PortalTarget, Vec<PortalRange<'_>>>,
  expanding: &mut Vec<PortalTarget>,
) -> Vec<HostNode> {
  let mut hosts = Vec::new();
  self::append_physical_hosts(tree, ranges, expanding, &mut hosts);
  hosts
}

fn append_physical_hosts(
  tree: &RenderTree,
  ranges: &HashMap<PortalTarget, Vec<PortalRange<'_>>>,
  expanding: &mut Vec<PortalTarget>,
  hosts: &mut Vec<HostNode>,
) {
  for position in &tree.positions {
    if position.portal.is_some() {
      continue;
    }
    if let Some(host) = &position.host {
      let mut host = host.without_children();
      if let Some(suspense) = position
        .suspense
        .as_ref()
        .filter(|suspense| suspense.showing_fallback)
      {
        let start = host.children.len();
        self::append_physical_hosts(&suspense.primary, ranges, expanding, &mut host.children);
        self::hide_roots(&mut host.children[start..]);
      }
      self::append_physical_hosts(&position.children, ranges, expanding, &mut host.children);
      if let Some(target) = &position.portal_target {
        assert!(
          !expanding.contains(target),
          "Reactant portal targets form a physical cycle"
        );
        expanding.push(target.clone());
        let mut target_ranges = ranges.get(target).into_iter().flatten().collect::<Vec<_>>();
        target_ranges.sort_by_key(|range| range.source);
        for range in target_ranges {
          let start = host.children.len();
          self::append_physical_hosts(range.tree, ranges, expanding, &mut host.children);
          if range.hidden {
            self::hide_roots(&mut host.children[start..]);
          }
        }
        expanding.pop();
      }
      hosts.push(host);
    } else {
      if let Some(suspense) = position
        .suspense
        .as_ref()
        .filter(|suspense| suspense.showing_fallback)
      {
        let start = hosts.len();
        self::append_physical_hosts(&suspense.primary, ranges, expanding, hosts);
        self::hide_roots(&mut hosts[start..]);
      }
      self::append_physical_hosts(&position.children, ranges, expanding, hosts);
    }
  }
}

fn hide_roots(hosts: &mut [HostNode]) {
  for host in hosts {
    let visual = ui_host_adapter::element_mut(host).visual_element_mut();
    visual.auto_focus = Prop::Set(false);
    visual.inert = Prop::Set(true);
    visual.style.display = Prop::Set(StyleValue::Value(Display::None));
  }
}

fn coverage_subscriptions(hosts: &[HostNode], trees: &[&RenderTree]) -> Vec<UiEventSubscription> {
  let mut unmatched_object_ids = HashSet::new();
  self::collect_host_ids(hosts, &mut unmatched_object_ids);
  let mut kinds = Vec::new();
  let mut path = Vec::new();
  for tree in trees {
    self::collect_coverage_kinds(
      tree,
      &mut unmatched_object_ids,
      false,
      &mut path,
      &mut kinds,
    );
    if unmatched_object_ids.is_empty() {
      break;
    }
  }
  assert!(
    unmatched_object_ids.is_empty(),
    "every physical Reactant host has a logical path"
  );
  kinds.sort_by_key(|kind| *kind as usize);
  kinds.dedup();
  kinds
    .into_iter()
    .flat_map(|kind| {
      [
        UiEventSubscription::target(kind),
        UiEventSubscription::new(kind, UiEventPhase::Trickle),
      ]
    })
    .collect()
}

fn collect_coverage_kinds(
  tree: &RenderTree,
  unmatched_object_ids: &mut HashSet<ObjectId>,
  hidden: bool,
  path: &mut Vec<UiEventKind>,
  kinds: &mut Vec<UiEventKind>,
) {
  for position in &tree.positions {
    let path_length = path.len();
    if let Some(host) = &position.host {
      if !hidden {
        path.extend(
          position
            .handlers
            .iter()
            .map(Handler::native_kind)
            .filter(|kind| kind.propagates()),
        );
      }
      if unmatched_object_ids.remove(&host.object_id) {
        kinds.extend(path.iter().copied());
      }
    }
    if let Some(suspense) = &position.suspense
      && (hidden || suspense.showing_fallback)
    {
      self::collect_coverage_kinds(&suspense.primary, unmatched_object_ids, true, path, kinds);
    }
    self::collect_coverage_kinds(
      &position.children,
      unmatched_object_ids,
      hidden,
      path,
      kinds,
    );
    path.truncate(path_length);
  }
}

fn external_root(mut hosts: Vec<HostNode>, trees: &[&RenderTree]) -> PortalRoot {
  for host in &mut hosts {
    let subscriptions = self::coverage_subscriptions(std::slice::from_ref(host), trees);
    let visual = ui_host_adapter::element_mut(host).visual_element_mut();
    let mut combined = match &visual.event_subscriptions {
      Prop::Set(subscriptions) => subscriptions.clone(),
      Prop::Unset | Prop::Reset => Vec::new(),
    };
    combined.extend(subscriptions);
    combined.sort_by_key(|subscription| {
      let phase = match subscription.phase {
        UiEventPhase::Target => 0,
        UiEventPhase::Trickle => 1,
        UiEventPhase::Bubble => 2,
      };
      (subscription.kind as usize, phase)
    });
    combined.dedup();
    visual.event_subscriptions = if combined.is_empty() {
      Prop::Unset
    } else {
      Prop::Set(combined)
    };
  }
  PortalRoot {
    hosts,
    subscriptions: Vec::new(),
  }
}

fn collect_host_ids(hosts: &[HostNode], object_ids: &mut HashSet<ObjectId>) {
  for host in hosts {
    object_ids.insert(host.object_id);
    self::collect_host_ids(&host.children, object_ids);
  }
}

fn collect_unique_host_ids(hosts: &[HostNode], object_ids: &mut HashSet<ObjectId>) {
  for host in hosts {
    assert!(
      object_ids.insert(host.object_id),
      "Reactant physical hosts must have unique IDs"
    );
    self::collect_unique_host_ids(&host.children, object_ids);
  }
}

pub(crate) struct PortalMarker;
