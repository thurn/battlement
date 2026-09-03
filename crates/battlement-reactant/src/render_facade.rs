use std::any::TypeId;

use battlement::{
  MotionDescriptor, MotionGeneration, ObjectId, Prop, UiElement, UiEventSubscription, UiNode,
  UiVisualElementProperties,
};

use crate::{
  element_ref::ElementRef,
  event_handler::Handler,
  host_facade::FacadeMetadata,
  motion::MotionProps,
  motion_lifecycle::{self, MotionCallbackRegistration, MotionCallbacks},
  motion_variants::{ExitBlueprint, ResolvedVariants, VariantScope},
  reconcile,
  render::{RenderPosition, RenderTree},
};

pub(crate) struct PreparedFacade {
  pub(crate) remount: bool,
  pub(crate) resolved_variants: ResolvedVariants,
  descriptor: TypeId,
  metadata: Box<FacadeMetadata>,
  node: UiNode,
  drag_constraint_ref: Option<ElementRef>,
  motion_callbacks: MotionCallbacks,
  exit_blueprint: Option<ExitBlueprint>,
  previous_motion: Option<MotionDescriptor>,
  motion_callback_history: Vec<MotionCallbackRegistration>,
}

pub(crate) fn prepare(
  descriptor: TypeId,
  metadata: Box<FacadeMetadata>,
  element: Box<UiElement>,
  matching: Option<&RenderPosition>,
  scope: &VariantScope,
) -> Box<PreparedFacade> {
  let previous = matching.and_then(|position| position.host.as_ref());
  let mut node = UiNode::new(
    previous.map_or_else(ObjectId::new_v4, |value| value.object_id),
    *element,
  );
  let remount =
    previous.is_some_and(|value| reconcile::requires_remount(&value.element, &node.element));
  if remount {
    node.object_id = ObjectId::new_v4();
  }
  let resolved_variants = scope.resolve(&metadata.motion);
  let drag_constraint_ref = metadata.motion.drag_constraint_ref().cloned();
  let motion_callbacks = metadata.motion.callbacks(&resolved_variants);
  let exit_blueprint = ExitBlueprint::new(metadata.motion.clone(), scope.clone());
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
  Box::new(PreparedFacade {
    remount,
    resolved_variants,
    descriptor,
    metadata,
    node,
    drag_constraint_ref,
    motion_callbacks,
    exit_blueprint,
    previous_motion,
    motion_callback_history,
  })
}

impl PreparedFacade {
  pub(crate) fn finish(
    self: Box<Self>,
    children: RenderTree,
    scope: &VariantScope,
  ) -> RenderPosition {
    let Self {
      descriptor,
      metadata,
      mut node,
      drag_constraint_ref,
      motion_callbacks,
      exit_blueprint,
      previous_motion,
      motion_callback_history,
      resolved_variants,
      ..
    } = *self;
    let duration_micros = metadata.motion.resolved_duration_micros(&resolved_variants);
    resolved_variants.complete(scope, duration_micros);
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
      node.element.visual_element_mut().motion = if previous_motion
        .as_ref()
        .is_some_and(|previous| &same_generation == previous)
      {
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
    RenderPosition {
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
      semantic: metadata.semantic,
      suspense: None,
      retained_render: metadata.retained_render,
      exit_blueprint,
      presence: None,
      children,
    }
  }
}
