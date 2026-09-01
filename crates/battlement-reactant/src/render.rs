//! Render values supported by the Reactant tree builder.

use std::{
  any::{Any, TypeId},
  rc::Rc,
};

use battlement::{
  MotionGeneration, ObjectId, Prop, UiElement, UiEventSubscription, UiNode,
  UiVisualElementProperties,
};

use crate::{
  context::ProviderValue,
  element_ref::ElementRef,
  error_boundary::{BoundaryMarker, BoundaryState, ErasedDependencies, ErrorHandler},
  event_handler::Handler,
  hook_storage::HookComponent,
  hooks,
  host_facade::FacadeMetadata,
  key::ErasedKey,
  motion::MotionProps,
  motion_component::MotionComponent,
  motion_lifecycle::{self, MotionCallbackRegistration, MotionCallbacks},
  motion_variants::{ExitBlueprint, VariantScope},
  overlay::OverlayReference,
  portal::PortalTarget,
  presence::{PresenceBoundaryState, PresenceConfig},
  presence_render, reconcile,
  render_value::{ErasedRender, Sealed},
  resource_runtime::ResourceToken,
  runtime::RenderError,
  suspense::{SuspenseMarker, SuspenseState},
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
/// use battlement::UiLabel;
/// use battlement_reactant::render::Render;
///
/// fn accepts_render(_value: impl Render) {}
/// accepts_render([UiLabel::new("one")].into_iter());
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

#[derive(Clone, Default)]
pub(crate) struct RenderTree {
  pub(crate) positions: Vec<RenderPosition>,
}

#[derive(Clone)]
pub(crate) struct RenderPosition {
  pub(crate) descriptor: TypeId,
  pub(crate) key: Option<ErasedKey>,
  pub(crate) host: Option<UiNode>,
  pub(crate) handlers: Vec<Handler>,
  pub(crate) motion_callbacks: MotionCallbacks,
  pub(crate) motion_callback_history: Vec<MotionCallbackRegistration>,
  pub(crate) component: Option<HookComponent>,
  pub(crate) memo_value: Option<Rc<dyn Any>>,
  pub(crate) provider: Option<ProviderValue>,
  pub(crate) portal: Option<PortalTarget>,
  pub(crate) portal_target: Option<PortalTarget>,
  pub(crate) error_boundary: Option<BoundaryState>,
  pub(crate) element_ref: Option<ElementRef>,
  pub(crate) drag_constraint_ref: Option<ElementRef>,
  pub(crate) overlay_reference: Option<OverlayReference>,
  pub(crate) suspense: Option<SuspenseState>,
  pub(crate) retained_render: Option<Node>,
  pub(crate) exit_blueprint: Option<ExitBlueprint>,
  pub(crate) presence: Option<PresenceBoundaryState>,
  pub(crate) children: RenderTree,
}

pub(crate) struct RenderSink<'a> {
  pub(crate) committed: &'a RenderTree,
  pub(crate) positions: Vec<RenderPosition>,
  pub(crate) error: Option<RenderError>,
  pub(crate) pending: Vec<ResourceToken>,
  pending_hook_lengths: Vec<usize>,
  pub(crate) variant_scope: VariantScope,
}

fn motion_host(tree: &RenderTree) -> Option<ObjectId> {
  let mut result = None;
  for position in &tree.positions {
    if let Some(host) = &position.host
      && matches!(host.element.visual_element().motion, Prop::Set(_))
    {
      assert!(
        result.replace(host.object_id).is_none(),
        "MotionComponent must forward Motion props to exactly one host façade"
      );
    }
    if let Some(host) = motion_host(&position.children) {
      assert!(
        result.replace(host).is_none(),
        "MotionComponent must forward Motion props to exactly one host façade"
      );
    }
  }
  result
}

pub(crate) fn sink_with_scope(
  committed: &RenderTree,
  variant_scope: VariantScope,
) -> RenderSink<'_> {
  let mut pending_hook_lengths = Vec::new();
  committed.pending_hook_lengths(&mut pending_hook_lengths);
  RenderSink {
    committed,
    positions: Vec::new(),
    error: None,
    pending: Vec::new(),
    pending_hook_lengths,
    variant_scope,
  }
}

impl<'a> RenderSink<'a> {
  pub(crate) fn new(committed: &'a RenderTree) -> Self {
    sink_with_scope(committed, VariantScope::default())
  }

  pub(crate) fn push_keyed<R: 'static>(
    &mut self,
    key: ErasedKey,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    self.push_keyed_source::<R>(key, None, render);
  }

  pub(crate) fn push_keyed_source<R: 'static>(
    &mut self,
    key: ErasedKey,
    retained_render: Option<Node>,
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
    let mut children = sink_with_scope(committed, self.variant_scope.clone());
    render(&mut children);
    let (children, pending) = match Self::finish_child(children) {
      Ok(attempt) => attempt,
      Err(error) => {
        self.fail(error);
        return;
      }
    };
    self.pending.extend(pending);
    self.positions.push(RenderPosition {
      descriptor,
      key: Some(key),
      host: None,
      handlers: Vec::new(),
      motion_callbacks: MotionCallbacks::default(),
      motion_callback_history: Vec::new(),
      component: None,
      memo_value: None,
      provider: None,
      portal: None,
      portal_target: None,
      error_boundary: None,
      element_ref: None,
      drag_constraint_ref: None,
      overlay_reference: None,
      suspense: None,
      retained_render,
      exit_blueprint: None,
      presence: None,
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
      let mut children = sink_with_scope(committed, self.variant_scope.clone());
      let (rendered, render_retry) = hooks::render_component(component, || render(&mut children));
      component = rendered;
      let (children, pending) = match children.finish_attempt() {
        Ok(attempt) => attempt,
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
      self.pending.extend(pending);
      self.positions.push(RenderPosition {
        descriptor,
        key: None,
        host: None,
        handlers: Vec::new(),
        motion_callbacks: MotionCallbacks::default(),
        motion_callback_history: Vec::new(),
        component: Some(component),
        memo_value: None,
        provider: None,
        portal: None,
        portal_target: None,
        error_boundary: None,
        element_ref: None,
        drag_constraint_ref: None,
        overlay_reference: None,
        suspense: None,
        retained_render: None,
        exit_blueprint: None,
        presence: None,
        children,
      });
      break;
    }
  }

  pub(crate) fn push_motion_component<B, C>(&mut self, component: C, motion: MotionProps)
  where
    B: 'static,
    C: MotionComponent + Clone,
  {
    let descriptor = TypeId::of::<B>();
    let previous = self
      .matching_position(descriptor)
      .and_then(|position| motion_host(&position.children));
    self.push_component::<B>(|children| {
      component
        .clone()
        .with_motion(motion.clone())
        .render()
        .render_owned(children);
    });
    if self.error.is_some() {
      return;
    }
    let current = motion_host(
      &self
        .positions
        .last()
        .expect("forwarded Motion component position is missing")
        .children,
    )
    .expect("MotionComponent must forward Motion props to exactly one host façade");
    assert!(
      previous.is_none_or(|value| value == current),
      "MotionComponent changed its forwarded host without changing component identity"
    );
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
        motion_callbacks: MotionCallbacks::default(),
        motion_callback_history: Vec::new(),
        component: matching.component,
        memo_value: Some(component_value),
        provider: None,
        portal: None,
        portal_target: None,
        error_boundary: None,
        element_ref: None,
        drag_constraint_ref: None,
        overlay_reference: None,
        suspense: None,
        retained_render: None,
        exit_blueprint: None,
        presence: None,
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
      let mut children = sink_with_scope(committed, self.variant_scope.clone());
      let (rendered, render_retry) = hooks::render_component(component, || render(&mut children));
      component = rendered;
      let (children, pending) = match children.finish_attempt() {
        Ok(attempt) => attempt,
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
      self.pending.extend(pending);
      self.positions.push(RenderPosition {
        descriptor,
        key: None,
        host: None,
        handlers: Vec::new(),
        motion_callbacks: MotionCallbacks::default(),
        motion_callback_history: Vec::new(),
        component: Some(component),
        memo_value: Some(component_value),
        provider: None,
        portal: None,
        portal_target: None,
        error_boundary: None,
        element_ref: None,
        drag_constraint_ref: None,
        overlay_reference: None,
        suspense: None,
        retained_render: None,
        exit_blueprint: None,
        presence: None,
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

  pub(crate) fn push_facade<R: 'static>(
    &mut self,
    metadata: FacadeMetadata,
    element: UiElement,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    if self.error.is_some() {
      return;
    }
    if let Some(key) = &metadata.key {
      assert!(
        !self
          .positions
          .iter()
          .any(|position| position.key.as_ref() == Some(key)),
        "duplicate sibling key"
      );
    }
    let descriptor = TypeId::of::<R>();
    let matching = match &metadata.key {
      Some(key) => self
        .committed
        .positions
        .iter()
        .find(|position| position.key.as_ref() == Some(key))
        .filter(|position| position.descriptor == descriptor),
      None => self.matching_position(descriptor),
    };
    let previous = matching.and_then(|position| position.host.as_ref());
    let mut node = UiNode::new(
      previous.map_or_else(ObjectId::new_v4, |value| value.object_id),
      element,
    );
    let remount =
      previous.is_some_and(|value| reconcile::requires_remount(&value.element, &node.element));
    if remount {
      node.object_id = ObjectId::new_v4();
    }
    let resolved_variants = self.variant_scope.resolve(&metadata.motion);
    let drag_constraint_ref = metadata.motion.drag_constraint_ref().cloned();
    let motion_callbacks = metadata.motion.callbacks(&resolved_variants);
    let exit_blueprint = ExitBlueprint::new(metadata.motion.clone(), self.variant_scope.clone());
    let previous_motion = previous.and_then(|value| match &value.element.visual_element().motion {
      Prop::Set(value) => Some(value.clone()),
      Prop::Unset | Prop::Reset => None,
    });
    let motion_callback_history = matching.map_or_else(Vec::new, |position| {
      motion_lifecycle::carry_registrations(
        &position.motion_callback_history,
        previous_motion.as_ref(),
        &position.motion_callbacks,
      )
    });
    let empty = RenderTree::default();
    let committed = if remount {
      &empty
    } else {
      matching.map_or(&empty, |position| &position.children)
    };
    let mut children = sink_with_scope(committed, resolved_variants.child_scope.clone());
    render(&mut children);
    let (children, pending) = match Self::finish_child(children) {
      Ok(attempt) => attempt,
      Err(error) => {
        self.fail(error);
        return;
      }
    };
    self.pending.extend(pending);
    let duration_micros = metadata.motion.resolved_duration_micros(&resolved_variants);
    resolved_variants.complete(&self.variant_scope, duration_micros);
    if metadata.motion != MotionProps::new() || resolved_variants.descriptor.is_some() {
      let prior_generation = previous_motion
        .as_ref()
        .map_or(MotionGeneration(1), |value| value.generation);
      let same_generation = metadata.motion.descriptor(
        node.object_id,
        prior_generation,
        &resolved_variants,
        previous_motion.as_ref(),
      );
      node.element.visual_element_mut().motion =
        if previous_motion.as_ref() == Some(&same_generation) {
          Prop::Set(same_generation)
        } else {
          let generation = previous_motion
            .as_ref()
            .map_or(MotionGeneration(1), |value| {
              MotionGeneration(
                value
                  .generation
                  .0
                  .checked_add(1)
                  .expect("motion generation exhausted"),
              )
            });
          Prop::Set(metadata.motion.descriptor(
            node.object_id,
            generation,
            &resolved_variants,
            previous_motion.as_ref(),
          ))
        };
    } else if previous_motion.is_some() {
      node.element.visual_element_mut().motion = Prop::Reset;
    }
    node.children = children.hosts();
    let mut kinds = metadata
      .handlers
      .iter()
      .map(Handler::native_kind)
      .filter(|kind| !kind.propagates())
      .collect::<Vec<_>>();
    kinds.sort_by_key(|kind| *kind as usize);
    kinds.dedup();
    let visual = node.element.visual_element_mut();
    visual.events = Prop::Unset;
    visual.event_subscriptions = if kinds.is_empty() {
      Prop::Unset
    } else {
      Prop::Set(kinds.into_iter().map(UiEventSubscription::target).collect())
    };
    self.positions.push(RenderPosition {
      descriptor,
      key: metadata.key,
      host: Some(node),
      handlers: metadata.handlers,
      motion_callbacks,
      motion_callback_history,
      component: None,
      memo_value: None,
      provider: None,
      portal: None,
      portal_target: metadata.portal_target,
      error_boundary: None,
      element_ref: metadata.element_ref,
      drag_constraint_ref,
      overlay_reference: metadata.overlay_reference,
      suspense: None,
      retained_render: metadata.retained_render,
      exit_blueprint,
      presence: None,
      children,
    });
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
    let mut children = sink_with_scope(committed, self.variant_scope.clone());
    render(&mut children);
    let (children, pending) = match Self::finish_child(children) {
      Ok(attempt) => attempt,
      Err(error) => {
        self.fail(error);
        return;
      }
    };
    self.pending.extend(pending);
    self.positions.push(RenderPosition {
      descriptor,
      key: None,
      host: None,
      handlers: Vec::new(),
      motion_callbacks: MotionCallbacks::default(),
      motion_callback_history: Vec::new(),
      component: None,
      memo_value: None,
      provider: None,
      portal: Some(target),
      portal_target: None,
      error_boundary: None,
      element_ref: None,
      drag_constraint_ref: None,
      overlay_reference: None,
      suspense: None,
      retained_render: None,
      exit_blueprint: None,
      presence: None,
      children,
    });
  }

  pub(crate) fn push_presence<R: 'static>(
    &mut self,
    config: PresenceConfig,
    render: impl FnOnce(&mut RenderSink<'_>),
  ) {
    presence_render::push::<R>(self, config, render);
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
    let mut children = sink_with_scope(committed, self.variant_scope.clone());
    provider.enter(|| render(&mut children));
    let (children, pending) = match Self::finish_child(children) {
      Ok(attempt) => attempt,
      Err(error) => {
        self.fail(error);
        return;
      }
    };
    self.pending.extend(pending);
    self.positions.push(RenderPosition {
      descriptor,
      key: None,
      host: None,
      handlers: Vec::new(),
      motion_callbacks: MotionCallbacks::default(),
      motion_callback_history: Vec::new(),
      component: None,
      memo_value: None,
      provider: Some(provider),
      portal: None,
      portal_target: None,
      error_boundary: None,
      element_ref: None,
      drag_constraint_ref: None,
      overlay_reference: None,
      suspense: None,
      retained_render: None,
      exit_blueprint: None,
      presence: None,
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
    let mut children = sink_with_scope(committed, self.variant_scope.clone());
    render(&mut children);
    match Self::finish_child(children) {
      Ok((children, pending)) => {
        self.pending.extend(pending);
        self.push(descriptor, None, children);
      }
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
      let mut children = sink_with_scope(fallback_committed, self.variant_scope.clone());
      fallback(&error, &mut children);
      let (children, pending) = match Self::finish_child(children) {
        Ok(attempt) => attempt,
        Err(error) => {
          self.fail(error);
          return;
        }
      };
      self.pending.extend(pending);
      (children, Some(error), None)
    } else {
      let mut children = sink_with_scope(primary_committed, self.variant_scope.clone());
      primary(&mut children);
      match children.finish_attempt() {
        Ok((children, pending)) => {
          self.pending.extend(pending);
          (children, None, None)
        }
        Err(error) => {
          let mut children = sink_with_scope(fallback_committed, self.variant_scope.clone());
          fallback(&error, &mut children);
          let (children, pending) = match Self::finish_child(children) {
            Ok(attempt) => attempt,
            Err(fallback_error) => {
              self.fail(fallback_error);
              return;
            }
          };
          self.pending.extend(pending);
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
      motion_callbacks: MotionCallbacks::default(),
      motion_callback_history: Vec::new(),
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
      element_ref: None,
      drag_constraint_ref: None,
      overlay_reference: None,
      suspense: None,
      retained_render: None,
      exit_blueprint: None,
      presence: None,
      children,
    });
  }

  pub(crate) fn push_suspense(
    &mut self,
    primary: impl FnOnce(&mut RenderSink<'_>),
    fallback: impl FnOnce(&mut RenderSink<'_>),
  ) {
    if self.error.is_some() {
      return;
    }
    let descriptor = TypeId::of::<SuspenseMarker>();
    let empty = RenderTree::default();
    let matching = self.matching_position(descriptor);
    let previous = matching.and_then(|position| position.suspense.clone());
    let previous_fallback = previous
      .as_ref()
      .is_some_and(|state| state.showing_fallback);
    let primary_committed = if previous_fallback {
      &previous.as_ref().expect("fallback state exists").primary
    } else {
      matching.map_or(&empty, |position| &position.children)
    };
    let fallback_committed = if previous_fallback {
      matching.map_or(&empty, |position| &position.children)
    } else {
      &empty
    };
    let mut primary_children = sink_with_scope(primary_committed, self.variant_scope.clone());
    primary(&mut primary_children);
    if primary_children.error.is_none() && !primary_children.pending.is_empty() {
      primary_children.rollback_pending_hooks();
    }
    let (primary_children, pending) = match primary_children.finish_attempt() {
      Ok(attempt) => attempt,
      Err(error) => {
        self.fail(error);
        return;
      }
    };
    let showing_fallback = !pending.is_empty();
    let retained_primary = if showing_fallback {
      if previous_fallback {
        previous
          .as_ref()
          .expect("fallback state exists")
          .primary
          .clone()
      } else {
        matching.map_or_else(RenderTree::default, |position| position.children.clone())
      }
    } else {
      RenderTree::default()
    };
    let children = if showing_fallback {
      let mut fallback_children = sink_with_scope(fallback_committed, self.variant_scope.clone());
      fallback(&mut fallback_children);
      let (children, fallback_pending) = match Self::finish_child(fallback_children) {
        Ok(attempt) => attempt,
        Err(error) => {
          self.fail(error);
          return;
        }
      };
      self.pending.extend(fallback_pending);
      children
    } else {
      primary_children
    };
    let suspense = match previous {
      Some(state) => state.prepare(showing_fallback, pending, retained_primary),
      None => SuspenseState::new(showing_fallback, pending, retained_primary),
    };
    self.positions.push(RenderPosition {
      descriptor,
      key: None,
      host: None,
      handlers: Vec::new(),
      motion_callbacks: MotionCallbacks::default(),
      motion_callback_history: Vec::new(),
      component: None,
      memo_value: None,
      provider: None,
      portal: None,
      portal_target: None,
      error_boundary: None,
      element_ref: None,
      drag_constraint_ref: None,
      overlay_reference: None,
      suspense: Some(suspense),
      retained_render: None,
      exit_blueprint: None,
      presence: None,
      children,
    });
  }

  pub(crate) fn fail(&mut self, error: RenderError) {
    if self.error.is_none() {
      self.error = Some(error);
    }
  }

  pub(crate) fn suspend(&mut self, token: ResourceToken) {
    if self.error.is_none() {
      self.pending.push(token);
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
      motion_callbacks: MotionCallbacks::default(),
      motion_callback_history: Vec::new(),
      component: None,
      memo_value: None,
      provider: None,
      portal: None,
      portal_target: None,
      error_boundary: None,
      element_ref: None,
      drag_constraint_ref: None,
      overlay_reference: None,
      suspense: None,
      retained_render: None,
      exit_blueprint: None,
      presence: None,
      children,
    });
  }

  pub(crate) fn finish_child(
    children: RenderSink<'_>,
  ) -> Result<(RenderTree, Vec<ResourceToken>), RenderError> {
    children.finish_attempt()
  }

  pub(crate) fn finish(mut self) -> Result<RenderTree, RenderError> {
    if self.error.is_none() && !self.pending.is_empty() {
      self.rollback_pending_hooks();
      panic!("pending Reactant resource read requires a Suspense boundary");
    }
    self.finish_attempt().map(|(tree, _)| tree)
  }

  fn finish_attempt(mut self) -> Result<(RenderTree, Vec<ResourceToken>), RenderError> {
    match self.error.take() {
      Some(error) => {
        self.rollback_pending_hooks();
        Err(error)
      }
      None => Ok((
        RenderTree {
          positions: self.positions,
        },
        self.pending,
      )),
    }
  }

  fn rollback_pending_hooks(&mut self) {
    let mut cursor = 0;
    self
      .committed
      .truncate_pending_hooks(&self.pending_hook_lengths, &mut cursor);
    assert_eq!(cursor, self.pending_hook_lengths.len());
  }
}
