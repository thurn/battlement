//! Render values supported by the Reactant tree builder.

use std::{
  any::{Any, TypeId},
  collections::HashSet,
  rc::Rc,
};

use battlement::{ObjectId, Prop, UiEventSubscription, UiNode, VisualElementProperties};

use self::private::Sealed;
use crate::{
  context::ProviderValue,
  effect::EffectOperation,
  event_handler::Handler,
  hook_storage::{HookComponent, HookOwner},
  hooks,
  key::ErasedKey,
  portal::PortalTarget,
  reconcile,
};

/// A value Reactant can lower into native host descriptions.
///
/// Text and other raw scalar values deliberately do not implement this trait.
///
/// ```compile_fail
/// use battlement_reactant::render::Render;
///
/// fn accepts_render(_value: impl Render) {}
/// accepts_render("explicit controls are required");
/// ```
///
/// Arbitrary iterators must be collected into a supported structural value.
///
/// ```compile_fail
/// use battlement::Label;
/// use battlement_reactant::render::Render;
///
/// fn accepts_render(_value: impl Render) {}
/// accepts_render([Label::new("one")].into_iter());
/// ```
pub trait Render: private::Sealed + 'static {}

/// Groups children without introducing a native host.
pub struct Fragment<R> {
  children: R,
}

/// Selects one of two heterogeneous render values.
pub enum Either<L, R> {
  /// The left branch.
  Left(L),
  /// The right branch.
  Right(R),
}

/// Stores an immutable type-erased render value.
#[derive(Clone)]
pub struct Node {
  render: Rc<dyn ErasedRender>,
  descriptor: TypeId,
}

impl<R> Fragment<R> {
  /// Creates a hostless group around `children`.
  pub const fn new(children: R) -> Self {
    Self { children }
  }
}

impl<L, R> Either<L, R> {
  /// Selects the left render value.
  pub const fn left(value: L) -> Self {
    Self::Left(value)
  }

  /// Selects the right render value.
  pub const fn right(value: R) -> Self {
    Self::Right(value)
  }
}

impl Node {
  /// Erases an owned render value while retaining its concrete descriptor.
  pub fn new<R: Render>(render: R) -> Self {
    let descriptor = render.descriptor();
    Self {
      render: Rc::new(render),
      descriptor,
    }
  }
}

impl Render for () {}
impl<R: Render> Render for Option<R> {}
impl<R: Render, const N: usize> Render for [R; N] {}
impl<R: Render> Render for Vec<R> {}
impl<R: Render> Render for Rc<R> {}
impl<R: Render> Render for Fragment<R> {}
impl<L: Render, R: Render> Render for Either<L, R> {}
impl Render for Node {}

pub(crate) fn lower<R: Render>(value: R, committed: &RenderTree) -> RenderTree {
  let mut sink = RenderSink::new(committed);
  value.render_into(&mut sink);
  sink.finish()
}

#[derive(Clone, Default)]
pub(crate) struct RenderTree {
  pub(crate) positions: Vec<RenderPosition>,
}

#[derive(Clone)]
pub(crate) struct EventNode {
  pub(crate) object_id: ObjectId,
  pub(crate) handlers: Vec<Handler>,
}

impl RenderTree {
  pub(crate) fn hosts(&self) -> Vec<UiNode> {
    let mut hosts = Vec::new();
    self.append_hosts(&mut hosts);
    hosts
  }

  pub(crate) fn event_path(&self, target_id: ObjectId) -> Option<Vec<EventNode>> {
    let mut path = Vec::new();
    self.find_event_path(target_id, &mut path).then_some(path)
  }

  pub(crate) fn validate_model(&self, model: TypeId) {
    for position in &self.positions {
      assert!(
        position
          .handlers
          .iter()
          .all(|handler| handler.model() == model),
        "Reactant handler model type does not match its runtime"
      );
      position.children.validate_model(model);
    }
  }

  pub(crate) fn remount_changed_portals(&mut self, targets: &HashSet<PortalTarget>) {
    for position in &mut self.positions {
      if position
        .portal
        .as_ref()
        .is_some_and(|target| targets.contains(target))
      {
        position.children.remount_hosts();
      } else {
        position.children.remount_changed_portals(targets);
      }
    }
  }

  pub(crate) fn commit_hooks(&mut self) {
    for position in &mut self.positions {
      if let Some(component) = &mut position.component {
        component.commit();
      }
      position.children.commit_hooks();
    }
  }

  pub(crate) fn freeze_store_wakes(&mut self) {
    for position in &mut self.positions {
      if let Some(component) = &mut position.component {
        component.freeze_store_wakes();
      }
      position.children.freeze_store_wakes();
    }
  }

  pub(crate) fn hook_owners(&self, owners: &mut Vec<Rc<HookOwner>>) {
    for position in &self.positions {
      if let Some(component) = &position.component {
        owners.push(component.owner());
      }
      position.children.hook_owners(owners);
    }
  }

  pub(crate) fn take_effect_operations(&mut self, operations: &mut Vec<EffectOperation>) {
    for position in &mut self.positions {
      position.children.take_effect_operations(operations);
      if let Some(component) = &mut position.component {
        component.take_effect_operations(operations);
      }
    }
  }

  pub(crate) fn unmount_effects(
    &mut self,
    mounted: &[Rc<HookOwner>],
    operations: &mut Vec<EffectOperation>,
  ) {
    for position in &mut self.positions {
      position.children.unmount_effects(mounted, operations);
      let Some(component) = &mut position.component else {
        continue;
      };
      if !mounted
        .iter()
        .any(|candidate| component.owner.same(candidate))
      {
        component.unmount(operations);
      }
    }
  }

  pub(crate) fn unmount_all_effects(&mut self, operations: &mut Vec<EffectOperation>) {
    self.unmount_effects(&[], operations);
  }

  pub(crate) fn has_pending_hooks(&self) -> bool {
    self.positions.iter().any(|position| {
      position
        .component
        .as_ref()
        .is_some_and(HookComponent::has_pending)
        || position.children.has_pending_hooks()
    })
  }

  fn has_dirty_work(&self) -> bool {
    self.positions.iter().any(RenderPosition::has_dirty_work)
  }

  pub(crate) fn has_changed_hooks(&self) -> bool {
    self.positions.iter().any(|position| {
      position
        .component
        .as_ref()
        .is_some_and(HookComponent::has_pending_change)
        || position.children.has_changed_hooks()
    })
  }

  pub(crate) fn discard_pending_hooks(&mut self) {
    for position in &mut self.positions {
      if let Some(component) = &mut position.component {
        component.discard_pending();
      }
      position.children.discard_pending_hooks();
    }
  }

  fn append_hosts(&self, hosts: &mut Vec<UiNode>) {
    for position in &self.positions {
      if let Some(host) = &position.host {
        hosts.push(host.clone());
      } else {
        position.children.append_hosts(hosts);
      }
    }
  }

  fn remount_hosts(&mut self) {
    for position in &mut self.positions {
      if let Some(host) = &mut position.host {
        host.object_id = ObjectId::new_v4();
      }
      position.children.remount_hosts();
    }
  }

  fn find_event_path(&self, target_id: ObjectId, path: &mut Vec<EventNode>) -> bool {
    for position in &self.positions {
      if let Some(host) = &position.host {
        path.push(EventNode {
          object_id: host.object_id,
          handlers: position.handlers.clone(),
        });
        if host.object_id == target_id {
          return true;
        }
        if position.children.find_event_path(target_id, path) {
          return true;
        }
        path.pop();
      } else if position.children.find_event_path(target_id, path) {
        return true;
      }
    }
    false
  }
}

#[derive(Clone)]
pub(crate) struct RenderPosition {
  pub(crate) descriptor: TypeId,
  pub(crate) key: Option<ErasedKey>,
  pub(crate) host: Option<UiNode>,
  pub(crate) handlers: Vec<Handler>,
  pub(crate) component: Option<HookComponent>,
  pub(crate) memo_value: Option<Rc<dyn Any>>,
  pub(crate) provider: Option<ProviderValue>,
  pub(crate) portal: Option<PortalTarget>,
  pub(crate) portal_target: Option<PortalTarget>,
  pub(crate) children: RenderTree,
}

impl RenderPosition {
  pub(crate) fn host_id(&self) -> ObjectId {
    self
      .host
      .as_ref()
      .expect("Reactant portal targets require a host render value")
      .object_id
  }

  fn has_dirty_work(&self) -> bool {
    let component_dirty = self
      .component
      .as_ref()
      .is_some_and(|component| component.has_pending() || component.context_changed());
    if component_dirty {
      return true;
    }
    self.provider.as_ref().map_or_else(
      || self.children.has_dirty_work(),
      |provider| provider.enter(|| self.children.has_dirty_work()),
    )
  }

  fn adapter_host_mut(&mut self) -> &mut Self {
    if self.host.is_some() {
      return self;
    }
    assert_eq!(
      self.children.positions.len(),
      1,
      "Reactant host adapters require one host render position"
    );
    self.children.positions[0].adapter_host_mut()
  }
}

pub(crate) struct RenderSink<'a> {
  committed: &'a RenderTree,
  positions: Vec<RenderPosition>,
}

impl<'a> RenderSink<'a> {
  fn new(committed: &'a RenderTree) -> Self {
    Self {
      committed,
      positions: Vec::new(),
    }
  }

  pub(crate) fn push_keyed<R: 'static>(
    &mut self,
    key: ErasedKey,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    assert!(
      !self
        .positions
        .iter()
        .any(|position| position.key.as_ref() == Some(&key)),
      "duplicate sibling key"
    );
    let descriptor = TypeId::of::<R>();
    let empty = RenderTree::default();
    let committed = self
      .committed
      .positions
      .iter()
      .find(|position| position.key.as_ref() == Some(&key))
      .filter(|position| position.descriptor == descriptor)
      .map_or(&empty, |position| &position.children);
    let mut children = RenderSink::new(committed);
    render(&mut children);
    self.positions.push(RenderPosition {
      descriptor,
      key: Some(key),
      host: None,
      handlers: Vec::new(),
      component: None,
      memo_value: None,
      provider: None,
      portal: None,
      portal_target: None,
      children: children.finish(),
    });
  }

  pub(crate) fn push_component<C: 'static>(&mut self, mut render: impl FnMut(&mut RenderSink<'_>)) {
    let descriptor = TypeId::of::<C>();
    let empty = RenderTree::default();
    let matching = self.matching_position(descriptor);
    let committed = matching.map_or(&empty, |position| &position.children);
    let mut component = matching
      .and_then(|position| position.component.clone())
      .unwrap_or_else(HookComponent::new);
    let mut retries = 0;
    loop {
      let mut children = RenderSink::new(committed);
      let (rendered, render_retry) = hooks::render_component(component, || render(&mut children));
      component = rendered;
      let store_retry = !render_retry && component.stabilize_stores();
      if render_retry || store_retry {
        retries += 1;
        assert!(
          retries <= hooks::retry_limit(),
          "{}",
          if store_retry {
            "Reactant external store did not stabilize"
          } else {
            "Reactant render-phase update retry limit exceeded"
          }
        );
        continue;
      }
      self.positions.push(RenderPosition {
        descriptor,
        key: None,
        host: None,
        handlers: Vec::new(),
        component: Some(component),
        memo_value: None,
        provider: None,
        portal: None,
        portal_target: None,
        children: children.finish(),
      });
      break;
    }
  }

  pub(crate) fn push_memoized<B, C>(
    &mut self,
    component_value: Rc<C>,
    mut render: impl FnMut(&mut RenderSink<'_>),
  ) where
    B: 'static,
    C: PartialEq + 'static,
  {
    let descriptor = TypeId::of::<B>();
    let matching = self.matching_position(descriptor).cloned();
    let same_props = matching
      .as_ref()
      .and_then(|position| position.memo_value.as_ref())
      .and_then(|value| value.downcast_ref::<C>())
      .is_some_and(|value| value == component_value.as_ref());
    if same_props
      && matching
        .as_ref()
        .is_some_and(|position| !position.has_dirty_work())
    {
      let matching = matching.expect("matching memo position exists");
      self.positions.push(RenderPosition {
        descriptor,
        key: None,
        host: None,
        handlers: Vec::new(),
        component: matching.component,
        memo_value: Some(component_value),
        provider: None,
        portal: None,
        portal_target: None,
        children: matching.children,
      });
      return;
    }
    let empty = RenderTree::default();
    let committed = matching
      .as_ref()
      .map_or(&empty, |position| &position.children);
    let mut component = matching
      .as_ref()
      .and_then(|position| position.component.clone())
      .unwrap_or_else(HookComponent::new);
    let mut retries = 0;
    loop {
      let mut children = RenderSink::new(committed);
      let (rendered, render_retry) = hooks::render_component(component, || render(&mut children));
      component = rendered;
      let store_retry = !render_retry && component.stabilize_stores();
      if render_retry || store_retry {
        retries += 1;
        assert!(
          retries <= hooks::retry_limit(),
          "{}",
          if store_retry {
            "Reactant external store did not stabilize"
          } else {
            "Reactant render-phase update retry limit exceeded"
          }
        );
        continue;
      }
      self.positions.push(RenderPosition {
        descriptor,
        key: None,
        host: None,
        handlers: Vec::new(),
        component: Some(component),
        memo_value: Some(component_value),
        provider: None,
        portal: None,
        portal_target: None,
        children: children.finish(),
      });
      break;
    }
  }

  fn push_empty<R: 'static>(&mut self) {
    self.push(TypeId::of::<R>(), None, RenderTree::default());
  }

  pub(crate) fn push_host<R: 'static>(&mut self, host: impl FnOnce(ObjectId) -> UiNode) {
    let descriptor = TypeId::of::<R>();
    let previous = self
      .matching_position(descriptor)
      .and_then(|position| position.host.as_ref());
    let mut node = host(previous.map_or_else(ObjectId::new_v4, |node| node.object_id));
    if previous.is_some_and(|value| reconcile::requires_remount(&value.element, &node.element)) {
      node.object_id = ObjectId::new_v4();
    }
    self.push(descriptor, Some(node), RenderTree::default());
  }

  pub(crate) fn push_host_with_children<R: 'static>(
    &mut self,
    host: impl FnOnce(ObjectId, Vec<UiNode>) -> UiNode,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    let descriptor = TypeId::of::<R>();
    let matching = self.matching_position(descriptor);
    let previous = matching.and_then(|position| position.host.as_ref());
    let mut node = host(
      previous.map_or_else(ObjectId::new_v4, |value| value.object_id),
      Vec::new(),
    );
    let remount =
      previous.is_some_and(|value| reconcile::requires_remount(&value.element, &node.element));
    if remount {
      node.object_id = ObjectId::new_v4();
    }
    let empty = RenderTree::default();
    let committed = if remount {
      &empty
    } else {
      matching.map_or(&empty, |position| &position.children)
    };
    let mut children = RenderSink::new(committed);
    render(&mut children);
    let children = children.finish();
    node.children = children.hosts();
    self.push(descriptor, Some(node), children);
  }

  pub(crate) fn with_handler(
    &mut self,
    handler: Handler,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    let index = self.positions.len();
    render(self);
    assert_eq!(
      self.positions.len(),
      index + 1,
      "Reactant event handlers require one host render position"
    );
    let position = &mut self.positions[index];
    let host = position
      .host
      .as_mut()
      .expect("Reactant event handlers require a host render value");
    position
      .handlers
      .retain(|existing| !existing.same_slot(&handler));
    position.handlers.push(handler);
    let mut kinds = position
      .handlers
      .iter()
      .map(Handler::native_kind)
      .filter(|kind| !kind.propagates())
      .collect::<Vec<_>>();
    kinds.sort_by_key(|kind| *kind as usize);
    kinds.dedup();
    let visual = host.element.visual_element_mut();
    visual.events = Prop::Unset;
    visual.event_subscriptions = if kinds.is_empty() {
      Prop::Unset
    } else {
      Prop::Set(kinds.into_iter().map(UiEventSubscription::target).collect())
    };
  }

  pub(crate) fn push_portal<R: 'static>(
    &mut self,
    target: PortalTarget,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    let descriptor = TypeId::of::<R>();
    let empty = RenderTree::default();
    let committed = self
      .matching_position(descriptor)
      .filter(|position| position.portal.as_ref() == Some(&target))
      .map_or(&empty, |position| &position.children);
    let mut children = RenderSink::new(committed);
    render(&mut children);
    self.positions.push(RenderPosition {
      descriptor,
      key: None,
      host: None,
      handlers: Vec::new(),
      component: None,
      memo_value: None,
      provider: None,
      portal: Some(target),
      portal_target: None,
      children: children.finish(),
    });
  }

  pub(crate) fn with_portal_target(
    &mut self,
    target: PortalTarget,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    let index = self.positions.len();
    render(self);
    assert_eq!(
      self.positions.len(),
      index + 1,
      "Reactant portal targets require one host render position"
    );
    let host = self.positions[index].adapter_host_mut();
    assert!(
      host.portal_target.replace(target).is_none(),
      "a Reactant portal host has more than one target"
    );
  }

  pub(crate) fn push_nested<R: 'static>(&mut self, render: impl FnOnce(&mut RenderSink<'_>)) {
    self.push_nested_descriptor(TypeId::of::<R>(), render);
  }

  pub(crate) fn push_provider<R: 'static>(
    &mut self,
    provider: ProviderValue,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    let descriptor = TypeId::of::<R>();
    let empty = RenderTree::default();
    let committed = self
      .matching_position(descriptor)
      .map_or(&empty, |position| &position.children);
    let mut children = RenderSink::new(committed);
    provider.enter(|| render(&mut children));
    self.positions.push(RenderPosition {
      descriptor,
      key: None,
      host: None,
      handlers: Vec::new(),
      component: None,
      memo_value: None,
      provider: Some(provider),
      portal: None,
      portal_target: None,
      children: children.finish(),
    });
  }

  fn push_nested_descriptor(
    &mut self,
    descriptor: TypeId,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    let empty = RenderTree::default();
    let committed = self
      .matching_position(descriptor)
      .map_or(&empty, |position| &position.children);
    let mut children = RenderSink::new(committed);
    render(&mut children);
    self.push(descriptor, None, children.finish());
  }

  fn matching_position(&self, descriptor: TypeId) -> Option<&RenderPosition> {
    self
      .committed
      .positions
      .get(self.positions.len())
      .filter(|position| position.key.is_none() && position.descriptor == descriptor)
  }

  fn push(&mut self, descriptor: TypeId, host: Option<UiNode>, children: RenderTree) {
    self.positions.push(RenderPosition {
      descriptor,
      key: None,
      host,
      handlers: Vec::new(),
      component: None,
      memo_value: None,
      provider: None,
      portal: None,
      portal_target: None,
      children,
    });
  }

  fn finish(self) -> RenderTree {
    RenderTree {
      positions: self.positions,
    }
  }
}

trait ErasedRender {
  fn descriptor(&self) -> TypeId;
  fn render_into(&self, sink: &mut RenderSink<'_>);
}

impl<R: Render> ErasedRender for R {
  fn descriptor(&self) -> TypeId {
    Sealed::descriptor(self)
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    Sealed::render_into(self, sink);
  }
}

#[allow(private_interfaces)]
pub(crate) mod private {
  use std::any::TypeId;
  use std::rc::Rc;

  use crate::{
    component::Component,
    context,
    render::{Either, Fragment, Node, RenderSink},
  };

  pub trait Sealed {
    fn descriptor(&self) -> TypeId;
    fn render_into(&self, sink: &mut RenderSink<'_>);
  }

  impl Sealed for () {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<Self>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      sink.push_empty::<Self>();
    }
  }

  impl<R: super::Render> Sealed for Option<R> {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<OptionMarker>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      sink.push_nested::<OptionMarker>(|children| {
        if let Some(value) = self {
          value.render_into(children);
        }
      });
    }
  }

  impl<R: super::Render, const N: usize> Sealed for [R; N] {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<Self>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      for value in self {
        value.render_into(sink);
      }
    }
  }

  impl<R: super::Render> Sealed for Vec<R> {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<Self>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      for value in self {
        value.render_into(sink);
      }
    }
  }

  impl<R: super::Render> Sealed for Rc<R> {
    fn descriptor(&self) -> TypeId {
      self.as_ref().descriptor()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      self.as_ref().render_into(sink);
    }
  }

  impl<R: super::Render> Sealed for Fragment<R> {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<FragmentMarker>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      sink.push_nested::<FragmentMarker>(|children| self.children.render_into(children));
    }
  }

  impl<L: super::Render, R: super::Render> Sealed for Either<L, R> {
    fn descriptor(&self) -> TypeId {
      match self {
        Either::Left(value) => value.descriptor(),
        Either::Right(value) => value.descriptor(),
      }
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      match self {
        Either::Left(value) => value.render_into(sink),
        Either::Right(value) => value.render_into(sink),
      }
    }
  }

  impl Sealed for Node {
    fn descriptor(&self) -> TypeId {
      self.descriptor
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      debug_assert_eq!(self.descriptor, self.render.descriptor());
      self.render.render_into(sink);
    }
  }

  impl<C: Component> Sealed for C {
    fn descriptor(&self) -> TypeId {
      TypeId::of::<C>()
    }

    fn render_into(&self, sink: &mut RenderSink<'_>) {
      sink.push_component::<C>(|children| {
        debug_assert!(context::hooks_allowed());
        self.render().render_into(children);
      });
    }
  }

  impl<C: Component> super::Render for C {}

  macro_rules! tuple_render {
    ($($name:ident),+) => {
      impl<$($name: super::Render),+> Sealed for ($($name,)+) {
        fn descriptor(&self) -> TypeId {
          TypeId::of::<Self>()
        }

        fn render_into(&self, sink: &mut RenderSink<'_>) {
          #[allow(non_snake_case)]
          let ($($name,)+) = self;
          $($name.render_into(sink);)+
        }
      }

      impl<$($name: super::Render),+> super::Render for ($($name,)+) {}
    };
  }

  struct FragmentMarker;
  struct OptionMarker;

  tuple_render!(A);
  tuple_render!(A, B);
  tuple_render!(A, B, C);
  tuple_render!(A, B, C, D);
  tuple_render!(A, B, C, D, E);
  tuple_render!(A, B, C, D, E, F);
  tuple_render!(A, B, C, D, E, F, G);
  tuple_render!(A, B, C, D, E, F, G, H);
  tuple_render!(A, B, C, D, E, F, G, H, I);
  tuple_render!(A, B, C, D, E, F, G, H, I, J);
  tuple_render!(A, B, C, D, E, F, G, H, I, J, K);
  tuple_render!(A, B, C, D, E, F, G, H, I, J, K, L);
}
