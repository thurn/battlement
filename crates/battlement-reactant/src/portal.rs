//! Physical portal placement with logical Reactant ancestry.

#![allow(private_interfaces)]

use std::{
  any::TypeId,
  collections::{HashMap, HashSet},
  hash::Hash,
};

use battlement::{ObjectId, UiEventPhase, UiEventSubscription, UiNode};

use crate::{
  event::{EventHandler, EventHost},
  event_handler::Handler,
  key::Keyed,
  primitive::{Children, private::Host},
  render::{Render, RenderSink, RenderTree, private::Sealed},
};

/// Marks a render adapter that still resolves to exactly one host.
pub trait HostRender: Render + private::Sealed {}

/// Attaches an internal portal target to a host render value.
///
/// The adapter is terminal for properties, children, and events.
///
/// ```compile_fail
/// use battlement::VisualElement;
/// use battlement_reactant::portal::{PortalTarget, ReactantHostExt};
///
/// fn invalid(target: PortalTarget) {
///   let _ = VisualElement::new().portal_target(target).name("late");
/// }
/// ```
///
/// ```compile_fail
/// use battlement::{Label, VisualElement};
/// use battlement_reactant::{
///   portal::{PortalTarget, ReactantHostExt},
///   primitive::ContainerRenderExt,
/// };
///
/// fn invalid(target: PortalTarget) {
///   let _ = VisualElement::new().portal_target(target).child(Label::new("late"));
/// }
/// ```
///
/// ```compile_fail
/// use battlement::VisualElement;
/// use battlement_reactant::{
///   event::EventRenderExt,
///   portal::{PortalTarget, ReactantHostExt},
/// };
///
/// fn invalid(target: PortalTarget) {
///   let _ = VisualElement::new().portal_target(target).on_click(|_: &mut ()| {});
/// }
/// ```
pub trait ReactantHostExt: HostRender + Sized {
  /// Makes this host the unique container for `target`.
  fn portal_target(self, target: PortalTarget) -> PortalContainer<Self> {
    PortalContainer {
      render: self,
      target,
    }
  }
}

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

/// Attaches one internal portal target to a host.
pub struct PortalContainer<R> {
  render: R,
  target: PortalTarget,
}

/// Creates one logical portal occurrence.
pub fn create_portal<R: Render>(child: R, target: PortalTarget) -> Portal<R> {
  Portal { child, target }
}

impl<R: HostRender> ReactantHostExt for R {}

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
}

impl<R: HostRender> Render for PortalContainer<R> {}

impl<R: HostRender> Sealed for PortalContainer<R> {
  fn descriptor(&self) -> TypeId {
    self.render.descriptor()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.with_portal_target(self.target.clone(), |sink| {
      self.render.render_into(sink);
    });
  }
}

impl<T: Host + Render> private::Sealed for T {}
impl<T: Host + Render> HostRender for T {}

impl<H: Host, C: Render> private::Sealed for Children<H, C> {}
impl<H: Host, C: Render> HostRender for Children<H, C> {}

impl<R: EventHost + HostRender> private::Sealed for EventHandler<R> {}
impl<R: EventHost + HostRender> HostRender for EventHandler<R> {}

impl<R, K> private::Sealed for Keyed<R, K>
where
  R: HostRender,
  K: Clone + Eq + Hash + 'static,
{
}

impl<R, K> HostRender for Keyed<R, K>
where
  R: HostRender,
  K: Clone + Eq + Hash + 'static,
{
}

impl<R: HostRender> private::Sealed for PortalContainer<R> {}
impl<R: HostRender> HostRender for PortalContainer<R> {}

pub(crate) struct PortalRoot {
  pub(crate) hosts: Vec<UiNode>,
  pub(crate) subscriptions: Vec<UiEventSubscription>,
}

pub(crate) struct PortalLayout {
  pub(crate) attachments: HashMap<PortalTarget, ObjectId>,
  pub(crate) roots: Vec<PortalRoot>,
}

pub(crate) fn layout(runtime_id: u64, trees: &[RenderTree]) -> PortalLayout {
  let mut catalog = PortalCatalog::default();
  for tree in trees {
    self::collect_portals(runtime_id, tree, &mut catalog);
  }
  for target in &catalog.referenced {
    assert!(
      catalog.attachments.contains_key(target),
      "a referenced Reactant portal target is not attached"
    );
  }
  let roots = trees
    .iter()
    .map(|tree| self::physical_hosts(tree, &catalog.ranges, &mut Vec::new()))
    .collect::<Vec<_>>();
  let mut physical_hosts = HashSet::new();
  for root in &roots {
    self::collect_unique_host_ids(root, &mut physical_hosts);
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
    roots,
  }
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
  ranges: HashMap<PortalTarget, Vec<&'a RenderTree>>,
  referenced: HashSet<PortalTarget>,
}

fn collect_portals<'a>(runtime_id: u64, tree: &'a RenderTree, catalog: &mut PortalCatalog<'a>) {
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
      catalog
        .ranges
        .entry(target.clone())
        .or_default()
        .push(&position.children);
    }
    self::collect_portals(runtime_id, &position.children, catalog);
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
  ranges: &HashMap<PortalTarget, Vec<&RenderTree>>,
  expanding: &mut Vec<PortalTarget>,
) -> Vec<UiNode> {
  let mut hosts = Vec::new();
  for position in &tree.positions {
    if position.portal.is_some() {
      continue;
    }
    if let Some(host) = &position.host {
      let mut host = host.clone();
      host.children = self::physical_hosts(&position.children, ranges, expanding);
      if let Some(target) = &position.portal_target {
        assert!(
          !expanding.contains(target),
          "Reactant portal targets form a physical cycle"
        );
        expanding.push(target.clone());
        for range in ranges.get(target).into_iter().flatten() {
          host
            .children
            .extend(self::physical_hosts(range, ranges, expanding));
        }
        expanding.pop();
      }
      hosts.push(host);
    } else {
      hosts.extend(self::physical_hosts(&position.children, ranges, expanding));
    }
  }
  hosts
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

mod private {
  pub trait Sealed {}
}
