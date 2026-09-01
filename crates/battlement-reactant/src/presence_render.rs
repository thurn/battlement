use std::any::TypeId;

use battlement::{MotionLayer, Prop, UiVisualElementProperties};

use crate::{
  motion_lifecycle::{self, MotionCallbacks},
  presence::{
    self, AutomaticExit, PresenceBoundaryState, PresenceConfig, PresenceExit, PresenceMode,
    PresenceRenderState,
  },
  render::{RenderPosition, RenderSink, RenderTree, sink_with_scope},
  render_value::Sealed,
};

pub(crate) fn push<R: 'static>(
  sink: &mut RenderSink<'_>,
  config: PresenceConfig,
  render: impl FnOnce(&mut RenderSink<'_>),
) {
  if sink.error.is_some() {
    return;
  }
  assert!(
    config.mode != PresenceMode::PopLayout,
    "PresenceMode::PopLayout requires layout projection"
  );
  let descriptor = TypeId::of::<R>();
  let previous = sink
    .committed
    .positions
    .get(sink.positions.len())
    .filter(|position| position.key.is_none() && position.descriptor == descriptor);
  let previous_children = previous.map_or_else(RenderTree::default, |value| value.children.clone());
  let previous_state = previous.and_then(|value| value.presence.clone());
  let generation = previous_state
    .as_ref()
    .map_or(1, |value| value.generation.saturating_add(1));
  let mut children = sink_with_scope(&previous_children, sink.variant_scope.clone());
  presence::with_state(
    PresenceRenderState {
      present: true,
      generation,
    },
    || render(&mut children),
  );
  let (mut current, pending) = match RenderSink::finish_child(children) {
    Ok(value) => value,
    Err(error) => {
      sink.fail(error);
      return;
    }
  };
  flatten_structural_children(&mut current);
  validate_keyed(&current);
  stabilize_transparent_children(
    &mut current,
    &previous_children,
    generation,
    &sink.variant_scope,
  );
  sink.pending.extend(pending);
  if previous.is_none() && !config.initial {
    suppress_initial(&mut current);
  }

  let mut state = previous_state
    .unwrap_or_else(|| PresenceBoundaryState::new(generation, config.on_exit_complete.clone()));
  let completed_wave = state.notified;
  if completed_wave {
    state.exits.clear();
  }
  state.generation = generation;
  state.handler = config.on_exit_complete;
  let current_keys = current
    .positions
    .iter()
    .filter_map(|position| position.key.clone())
    .collect::<Vec<_>>();
  state
    .exits
    .retain(|exit| !current_keys.iter().any(|key| key == &exit.key));

  let mut retained = Vec::new();
  for prior in &previous_children.positions {
    let key = prior
      .key
      .as_ref()
      .expect("presence child key was validated");
    if current_keys.iter().any(|current| current == key) {
      continue;
    }
    if completed_wave {
      continue;
    }
    let existing = state.exits.iter().find(|exit| &exit.key == key).cloned();
    let exit_generation = existing.as_ref().map_or(generation, |exit| exit.generation);
    let mut exiting = rerender_retained(prior, exit_generation, &sink.variant_scope);
    let exit = existing.unwrap_or_else(|| {
      let (automatic, holds) = start_exit(&mut exiting, config.custom.as_ref());
      PresenceExit {
        key: key.clone(),
        generation: exit_generation,
        automatic,
        holds,
      }
    });
    if !state
      .exits
      .iter()
      .any(|candidate| candidate.key == exit.key)
    {
      state.exits.push(exit.clone());
      state.notified = false;
    } else {
      freeze_exit_motion(&mut exiting, prior);
    }
    if !exit.ready() {
      retained.push(exiting);
    }
  }

  let entering = current
    .positions
    .iter()
    .filter(|position| {
      !previous_children
        .positions
        .iter()
        .any(|previous| previous.key == position.key)
    })
    .count();
  let has_active_exits = state.exits.iter().any(|exit| !exit.ready());
  if config.mode == PresenceMode::Wait {
    assert!(
      entering <= 1,
      "PresenceMode::Wait supports one entering logical child"
    );
  }
  if config.mode == PresenceMode::Wait && has_active_exits {
    current.positions.retain(|position| {
      previous_children
        .positions
        .iter()
        .any(|previous| previous.key == position.key)
    });
  }
  current.positions.extend(retained);
  sink.positions.push(RenderPosition {
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
    suspense: None,
    retained_render: None,
    exit_blueprint: None,
    presence: Some(state),
    children: current,
  });
}

fn validate_keyed(tree: &RenderTree) {
  assert!(
    tree.positions.iter().all(|position| position.key.is_some()),
    "AnimatePresence children must be directly keyed"
  );
}

fn stabilize_transparent_children(
  current: &mut RenderTree,
  previous: &RenderTree,
  generation: u64,
  variant_scope: &crate::motion_variants::VariantScope,
) {
  for position in &mut current.positions {
    let Some(prior) = previous
      .positions
      .iter()
      .find(|prior| prior.key == position.key && prior.descriptor == position.descriptor)
    else {
      continue;
    };
    if first_host_id(position) != first_host_id(prior) {
      *position = rerender_position(position, prior, true, generation, variant_scope);
    }
  }
}

fn first_host_id(position: &RenderPosition) -> Option<battlement::ObjectId> {
  position
    .host
    .as_ref()
    .map(|host| host.object_id)
    .or_else(|| position.children.positions.iter().find_map(first_host_id))
}

fn flatten_structural_children(tree: &mut RenderTree) {
  let positions = std::mem::take(&mut tree.positions);
  for mut position in positions {
    if is_structural_wrapper(&position) {
      flatten_structural_children(&mut position.children);
      tree.positions.extend(position.children.positions);
    } else {
      tree.positions.push(position);
    }
  }
}

fn is_structural_wrapper(position: &RenderPosition) -> bool {
  position.key.is_none()
    && position.host.is_none()
    && position.handlers.is_empty()
    && position.component.is_none()
    && position.memo_value.is_none()
    && position.provider.is_none()
    && position.portal.is_none()
    && position.portal_target.is_none()
    && position.error_boundary.is_none()
    && position.element_ref.is_none()
    && position.suspense.is_none()
    && position.retained_render.is_none()
    && position.exit_blueprint.is_none()
    && position.presence.is_none()
}

fn rerender_retained(
  previous: &RenderPosition,
  generation: u64,
  variant_scope: &crate::motion_variants::VariantScope,
) -> RenderPosition {
  rerender_position(previous, previous, false, generation, variant_scope)
}

fn rerender_position(
  source_position: &RenderPosition,
  committed_position: &RenderPosition,
  present: bool,
  generation: u64,
  variant_scope: &crate::motion_variants::VariantScope,
) -> RenderPosition {
  let source = source_position
    .retained_render
    .clone()
    .expect("AnimatePresence keyed children must retain their render value");
  let committed = RenderTree {
    positions: vec![committed_position.clone()],
  };
  let mut sink = sink_with_scope(&committed, variant_scope.clone());
  presence::with_state(
    PresenceRenderState {
      present,
      generation,
    },
    || source.render_owned(&mut sink),
  );
  let (tree, pending) = RenderSink::finish_child(sink).expect("retained presence render failed");
  assert!(
    pending.is_empty(),
    "retained presence cannot suspend without its boundary"
  );
  let mut positions = tree.positions;
  assert_eq!(
    positions.len(),
    1,
    "retained keyed child changed cardinality"
  );
  positions.remove(0)
}

fn start_exit(
  position: &mut RenderPosition,
  custom: Option<&crate::variant_map::ErasedVariantData>,
) -> (
  Vec<AutomaticExit>,
  Vec<std::rc::Rc<crate::presence::PresenceCell>>,
) {
  let mut automatic = Vec::new();
  let mut holds = position
    .component
    .as_ref()
    .map_or_else(Vec::new, |component| component.presence_holds());
  if let (Some(host), Some(blueprint)) = (&mut position.host, &position.exit_blueprint) {
    let visual = host.element.visual_element_mut();
    if let Prop::Set(previous) = &visual.motion
      && let Some(descriptor) = blueprint.descriptor(host.object_id, previous, custom)
    {
      position.motion_callback_history = motion_lifecycle::carry_registrations(
        &position.motion_callback_history,
        Some(previous),
        &position.motion_callbacks,
      );
      position.motion_callbacks = blueprint.callbacks(custom);
      automatic.extend(
        descriptor
          .slots
          .iter()
          .map(|slot| AutomaticExit::new(descriptor.descriptor_id, slot.slot, slot.generation)),
      );
      visual.motion = Prop::Set(descriptor);
    }
  }
  for child in &mut position.children.positions {
    let (child_automatic, child_holds) = start_exit(child, custom);
    automatic.extend(child_automatic);
    holds.extend(child_holds);
  }
  (automatic, holds)
}

fn freeze_exit_motion(current: &mut RenderPosition, previous: &RenderPosition) {
  if let (Some(current_host), Some(previous_host)) = (&mut current.host, &previous.host) {
    let previous_motion = &previous_host.element.visual_element().motion;
    let exiting = matches!(
      previous_motion,
      Prop::Set(value) if value.slots.iter().any(|slot| slot.layer == MotionLayer::Exit)
    );
    if exiting {
      current_host.element.visual_element_mut().motion = previous_motion.clone();
      current.exit_blueprint = previous.exit_blueprint.clone();
      current.motion_callbacks = previous.motion_callbacks.clone();
      current.motion_callback_history = previous.motion_callback_history.clone();
    }
  }
  for child in &mut current.children.positions {
    if let Some(prior) = previous
      .children
      .positions
      .iter()
      .find(|prior| prior.key == child.key && prior.descriptor == child.descriptor)
    {
      freeze_exit_motion(child, prior);
    }
  }
}

fn suppress_initial(tree: &mut RenderTree) {
  for position in &mut tree.positions {
    if let Some(host) = &mut position.host
      && let Prop::Set(descriptor) = &mut host.element.visual_element_mut().motion
    {
      descriptor.initial = None;
      descriptor.initial_disabled = true;
    }
    suppress_initial(&mut position.children);
  }
}
