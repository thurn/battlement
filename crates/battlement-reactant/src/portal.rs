//! Physical portal placement with logical Reactant ancestry.

#![allow(private_interfaces)]

use std::{
  any::TypeId,
  collections::{HashMap, HashSet},
};

use battlement::{
  Display, ObjectId, Overflow, PickingMode, Prop, StyleValue, UiElement, UiEventPhase,
  UiEventSubscription, UiNode, UiVisualElementProperties,
};

use crate::{
  event_handler::Handler,
  render::{Render, RenderSink, RenderTree},
  render_value::Sealed,
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
  pub(crate) hosts: Vec<UiNode>,
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
  trees: &[RenderTree],
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

fn validate_overlay_hosts(roots: &[Vec<UiNode>], attachments: &HashMap<PortalTarget, ObjectId>) {
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
      root.len() == 1 && matches!(root[0].element, UiElement::Stack(_)),
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

fn collect_overlay_hosts(roots: &[UiNode], target_ids: &HashSet<ObjectId>) -> Vec<ObjectId> {
  let mut values = Vec::new();
  for node in roots {
    if target_ids.contains(&node.object_id) && self::is_overlay_host(node) {
      values.push(node.object_id);
    }
    values.extend(self::collect_overlay_hosts(&node.children, target_ids));
  }
  values
}

fn is_overlay_host(node: &UiNode) -> bool {
  let visual = node.element.visual_element();
  matches!(node.element, UiElement::Stack(_))
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

fn contains_overlay_wrapper(roots: &[UiNode]) -> bool {
  roots.iter().any(|node| {
    matches!(
      node.element.visual_element().overlay_placement,
      Prop::Set(_)
    ) || self::contains_overlay_wrapper(&node.children)
  })
}

fn find_host(roots: &[UiNode], id: ObjectId) -> Option<&UiNode> {
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
  desired: &PortalLayout,
) -> HashSet<PortalTarget> {
  previous
    .attachments
    .iter()
    .filter_map(|(target, previous_host)| {
      desired
        .attachments
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
) -> Vec<UiNode> {
  let mut hosts = Vec::new();
  for position in &tree.positions {
    if position.portal.is_some() {
      continue;
    }
    let mut retained = position
      .suspense
      .as_ref()
      .filter(|suspense| suspense.showing_fallback)
      .map_or_else(Vec::new, |suspense| {
        self::physical_hosts(&suspense.primary, ranges, expanding)
      });
    self::hide_roots(&mut retained);
    if let Some(host) = &position.host {
      let mut host = host.clone();
      host.children = retained;
      host
        .children
        .extend(self::physical_hosts(&position.children, ranges, expanding));
      if let Some(target) = &position.portal_target {
        assert!(
          !expanding.contains(target),
          "Reactant portal targets form a physical cycle"
        );
        expanding.push(target.clone());
        let mut target_ranges = ranges.get(target).into_iter().flatten().collect::<Vec<_>>();
        target_ranges.sort_by_key(|range| range.source);
        for range in target_ranges {
          let mut portal_hosts = self::physical_hosts(range.tree, ranges, expanding);
          if range.hidden {
            self::hide_roots(&mut portal_hosts);
          }
          host.children.extend(portal_hosts);
        }
        expanding.pop();
      }
      hosts.push(host);
    } else {
      hosts.extend(retained);
      hosts.extend(self::physical_hosts(&position.children, ranges, expanding));
    }
  }
  hosts
}

fn hide_roots(hosts: &mut [UiNode]) {
  for host in hosts {
    let visual = host.element.visual_element_mut();
    visual.auto_focus = Prop::Set(false);
    visual.inert = Prop::Set(true);
    visual.style.display = Prop::Set(StyleValue::Value(Display::None));
  }
}

fn coverage_subscriptions(hosts: &[UiNode], trees: &[RenderTree]) -> Vec<UiEventSubscription> {
  let mut object_ids = Vec::new();
  self::collect_host_ids(hosts, &mut object_ids);
  let mut kinds = Vec::new();
  for object_id in object_ids {
    let path = trees
      .iter()
      .find_map(|tree| tree.event_path(object_id))
      .expect("every physical Reactant host has a logical path");
    for node in path {
      kinds.extend(
        node
          .handlers
          .iter()
          .map(Handler::native_kind)
          .filter(|kind| kind.propagates()),
      );
    }
  }
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

fn external_root(mut hosts: Vec<UiNode>, trees: &[RenderTree]) -> PortalRoot {
  for host in &mut hosts {
    let subscriptions = self::coverage_subscriptions(std::slice::from_ref(host), trees);
    let visual = host.element.visual_element_mut();
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

fn collect_host_ids(hosts: &[UiNode], object_ids: &mut Vec<ObjectId>) {
  for host in hosts {
    object_ids.push(host.object_id);
    self::collect_host_ids(&host.children, object_ids);
  }
}

fn collect_unique_host_ids(hosts: &[UiNode], object_ids: &mut HashSet<ObjectId>) {
  for host in hosts {
    assert!(
      object_ids.insert(host.object_id),
      "Reactant physical hosts must have unique IDs"
    );
    self::collect_unique_host_ids(&host.children, object_ids);
  }
}

pub(crate) struct PortalMarker;
