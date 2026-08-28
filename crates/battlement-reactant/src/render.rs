//! Render values supported by the Reactant tree builder.

use std::{
  any::{Any, TypeId},
  collections::HashSet,
  rc::Rc,
};

use battlement::{ObjectId, Prop, UiEventSubscription, UiNode, VisualElementProperties};

use crate::{
  context::ProviderValue,
  effect::EffectOperation,
  error_boundary::{BoundaryMarker, BoundaryState, ErasedDependencies, ErrorHandler, ErrorReport},
  event_handler::Handler,
  hook_storage::{HookComponent, HookOwner},
  hooks,
  key::ErasedKey,
  portal::PortalTarget,
  reconcile,
  render_value::Sealed,
  runtime::RenderError,
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
pub trait Render: Sealed + 'static {}

/// Groups children without introducing a native host.
pub struct Fragment<R> {
  pub(crate) children: R,
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
  pub(crate) render: Rc<dyn ErasedRender>,
  pub(crate) descriptor: TypeId,
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
impl<R: Render, E: std::error::Error + 'static> Render for Result<R, E> {}
impl<R: Render> Render for Fragment<R> {}
impl<L: Render, R: Render> Render for Either<L, R> {}
impl Render for Node {}

pub(crate) fn lower<R: Render>(
  value: R,
  committed: &RenderTree,
) -> Result<RenderTree, RenderError> {
  let mut sink = RenderSink::new(committed);
  value.render_owned(&mut sink);
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
      if let Some(boundary) = &position.error_boundary {
        assert!(
          boundary
            .report
            .as_ref()
            .is_none_or(|report| report.model() == model),
          "Reactant error handler model type does not match its runtime"
        );
      }
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

  pub(crate) fn take_error_reports(&mut self, reports: &mut Vec<ErrorReport>) {
    for position in &mut self.positions {
      if let Some(report) = position
        .error_boundary
        .as_mut()
        .and_then(|boundary| boundary.report.take())
      {
        reports.push(report);
      }
      position.children.take_error_reports(reports);
    }
  }

  pub(crate) fn pending_hook_lengths(&self, lengths: &mut Vec<usize>) {
    for position in &self.positions {
      if let Some(component) = &position.component {
        component.pending_lengths(lengths);
      }
      position.children.pending_hook_lengths(lengths);
    }
  }

  pub(crate) fn truncate_pending_hooks(&self, lengths: &[usize], cursor: &mut usize) {
    for position in &self.positions {
      if let Some(component) = &position.component {
        component.truncate_pending(lengths, cursor);
      }
      position.children.truncate_pending_hooks(lengths, cursor);
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
  pub(crate) error_boundary: Option<BoundaryState>,
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
  error: Option<RenderError>,
  pending_hook_lengths: Vec<usize>,
}

impl<'a> RenderSink<'a> {
  fn new(committed: &'a RenderTree) -> Self {
    let mut pending_hook_lengths = Vec::new();
    committed.pending_hook_lengths(&mut pending_hook_lengths);
    Self {
      committed,
      positions: Vec::new(),
      error: None,
      pending_hook_lengths,
    }
  }

  pub(crate) fn push_keyed<R: 'static>(
    &mut self,
    key: ErasedKey,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    if self.error.is_some() {
      return;
    }
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
    let children = match children.finish() {
      Ok(children) => children,
      Err(error) => {
        self.fail(error);
        return;
      }
    };
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
      error_boundary: None,
      children,
    });
  }

  pub(crate) fn push_component<C: 'static>(&mut self, mut render: impl FnMut(&mut RenderSink<'_>)) {
    if self.error.is_some() {
      return;
    }
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
      let children = match children.finish() {
        Ok(children) => children,
        Err(error) => {
          self.fail(error);
          return;
        }
      };
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
        error_boundary: None,
        children,
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
    if self.error.is_some() {
      return;
    }
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
        error_boundary: None,
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
      let children = match children.finish() {
        Ok(children) => children,
        Err(error) => {
          self.fail(error);
          return;
        }
      };
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
        error_boundary: None,
        children,
      });
      break;
    }
  }

  pub(crate) fn push_empty<R: 'static>(&mut self) {
    if self.error.is_some() {
      return;
    }
    self.push(TypeId::of::<R>(), None, RenderTree::default());
  }

  pub(crate) fn push_host<R: 'static>(&mut self, host: impl FnOnce(ObjectId) -> UiNode) {
    if self.error.is_some() {
      return;
    }
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
    if self.error.is_some() {
      return;
    }
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
    let children = match children.finish() {
      Ok(children) => children,
      Err(error) => {
        self.fail(error);
        return;
      }
    };
    node.children = children.hosts();
    self.push(descriptor, Some(node), children);
  }

  pub(crate) fn with_handler(
    &mut self,
    handler: Handler,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    if self.error.is_some() {
      return;
    }
    let index = self.positions.len();
    render(self);
    if self.error.is_some() {
      return;
    }
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
    if self.error.is_some() {
      return;
    }
    let descriptor = TypeId::of::<R>();
    let empty = RenderTree::default();
    let committed = self
      .matching_position(descriptor)
      .filter(|position| position.portal.as_ref() == Some(&target))
      .map_or(&empty, |position| &position.children);
    let mut children = RenderSink::new(committed);
    render(&mut children);
    let children = match children.finish() {
      Ok(children) => children,
      Err(error) => {
        self.fail(error);
        return;
      }
    };
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
      error_boundary: None,
      children,
    });
  }

  pub(crate) fn with_portal_target(
    &mut self,
    target: PortalTarget,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    if self.error.is_some() {
      return;
    }
    let index = self.positions.len();
    render(self);
    if self.error.is_some() {
      return;
    }
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
    if self.error.is_some() {
      return;
    }
    let descriptor = TypeId::of::<R>();
    let empty = RenderTree::default();
    let committed = self
      .matching_position(descriptor)
      .map_or(&empty, |position| &position.children);
    let mut children = RenderSink::new(committed);
    provider.enter(|| render(&mut children));
    let children = match children.finish() {
      Ok(children) => children,
      Err(error) => {
        self.fail(error);
        return;
      }
    };
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
      error_boundary: None,
      children,
    });
  }

  fn push_nested_descriptor(
    &mut self,
    descriptor: TypeId,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    if self.error.is_some() {
      return;
    }
    let empty = RenderTree::default();
    let committed = self
      .matching_position(descriptor)
      .map_or(&empty, |position| &position.children);
    let mut children = RenderSink::new(committed);
    render(&mut children);
    match children.finish() {
      Ok(children) => self.push(descriptor, None, children),
      Err(error) => self.fail(error),
    }
  }

  pub(crate) fn push_error_boundary(
    &mut self,
    reset: Option<ErasedDependencies>,
    handler: Option<ErrorHandler>,
    primary: impl FnOnce(&mut RenderSink<'_>),
    fallback: impl FnOnce(&RenderError, &mut RenderSink<'_>),
  ) {
    if self.error.is_some() {
      return;
    }
    let descriptor = TypeId::of::<BoundaryMarker>();
    let empty = RenderTree::default();
    let matching = self.matching_position(descriptor);
    let previous = matching.and_then(|position| position.error_boundary.as_ref());
    let reset_changed = previous.is_some_and(|previous| previous.reset != reset);
    let previous_latched = previous.is_some_and(|previous| previous.error.is_some());
    let latched = (!reset_changed)
      .then(|| previous.and_then(|previous| previous.error.clone()))
      .flatten();
    let primary_committed = if previous_latched {
      &empty
    } else {
      matching.map_or(&empty, |position| &position.children)
    };
    let fallback_committed = if previous_latched {
      matching.map_or(&empty, |position| &position.children)
    } else {
      &empty
    };
    let (children, error, report) = if let Some(error) = latched {
      let mut children = RenderSink::new(fallback_committed);
      fallback(&error, &mut children);
      let children = match children.finish() {
        Ok(children) => children,
        Err(error) => {
          self.fail(error);
          return;
        }
      };
      (children, Some(error), None)
    } else {
      let mut children = RenderSink::new(primary_committed);
      primary(&mut children);
      match children.finish() {
        Ok(children) => (children, None, None),
        Err(error) => {
          let mut children = RenderSink::new(fallback_committed);
          fallback(&error, &mut children);
          let children = match children.finish() {
            Ok(children) => children,
            Err(fallback_error) => {
              self.fail(fallback_error);
              return;
            }
          };
          let report = handler
            .as_ref()
            .map(|handler| handler.report(error.clone()));
          (children, Some(error), report)
        }
      }
    };
    self.positions.push(RenderPosition {
      descriptor,
      key: None,
      host: None,
      handlers: Vec::new(),
      component: None,
      memo_value: None,
      provider: None,
      portal: None,
      portal_target: None,
      error_boundary: Some(BoundaryState {
        error,
        reset,
        report,
      }),
      children,
    });
  }

  pub(crate) fn fail(&mut self, error: RenderError) {
    if self.error.is_none() {
      self.error = Some(error);
    }
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
      error_boundary: None,
      children,
    });
  }

  fn finish(self) -> Result<RenderTree, RenderError> {
    match self.error {
      Some(error) => {
        let mut cursor = 0;
        self
          .committed
          .truncate_pending_hooks(&self.pending_hook_lengths, &mut cursor);
        assert_eq!(cursor, self.pending_hook_lengths.len());
        Err(error)
      }
      None => Ok(RenderTree {
        positions: self.positions,
      }),
    }
  }
}

pub(crate) trait ErasedRender {
  fn descriptor(&self) -> TypeId;
  fn render_into(self: Rc<Self>, sink: &mut RenderSink<'_>);
}

impl<R: Render> ErasedRender for R {
  fn descriptor(&self) -> TypeId {
    Sealed::descriptor(self)
  }

  fn render_into(self: Rc<Self>, sink: &mut RenderSink<'_>) {
    Sealed::render_shared(self, sink);
  }
}
