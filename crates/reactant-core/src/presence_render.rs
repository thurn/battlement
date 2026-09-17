use std::any::TypeId;

use battlement::{MotionLayer, OverlayLayer, OverlayPlacement, Prop, UiVisualElementProperties};

use crate::{
  motion_lifecycle::{self, MotionCallbacks},
  presence::{
    self, AutomaticExit, PresenceBoundaryState, PresenceConfig, PresenceExit, PresenceHold,
    PresenceMode, PresenceRenderState,
  },
  render::{RenderPosition, RenderSink, RenderTree, sink_with_scope},
  ui_host_adapter,
};

pub(crate) fn push<R: 'static>(
  sink: &mut RenderSink<'_>,
  config: PresenceConfig,
  render: impl FnOnce(&mut RenderSink<'_>),
) {
  if sink.error.is_some() {
    return;
  }
  let descriptor = TypeId::of::<R>();
  let committed = sink.committed;
  let previous = committed
    .positions
    .get(sink.positions.len())
    .filter(|position| position.key.is_none() && position.descriptor == descriptor);
  let empty = RenderTree::default();
  let previous_children = previous.map_or(&empty, |value| &value.children);
  let previous_state = previous.and_then(|value| value.presence.clone());
  let generation = previous_state
    .as_ref()
    .map_or(1, |value| value.generation.saturating_add(1));
  let previous_live = RenderTree {
    positions: previous_children
      .positions
      .iter()
      .filter(|position| !position.terminal_visual)
      .cloned()
      .collect(),
  };
  let mut children = sink_with_scope(&previous_live, sink.variant_scope.clone(), sink.identities);
  children.direct_presence_children = true;
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
  validate_keyed(&current);
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
    let mut exiting = prior.clone();
    mark_inert(&mut exiting);
    if config.mode == PresenceMode::PopLayout {
      mark_pop_layout(&mut exiting);
    }
    let exit = existing.unwrap_or_else(|| {
      let (automatic, holds) = start_exit(&mut exiting, config.custom.as_ref());
      PresenceExit {
        key: key.clone(),
        generation: exit_generation,
        automatic,
        holds,
        resources: crate::retained_visual::resources(&exiting),
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
      crate::retained_visual::freeze(&mut exiting);
      exiting.terminal_visual = true;
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
    presentation_id: None,
    hidden: false,
    terminal_visual: false,
    key: None,
    host: None,
    handlers: Vec::new(),
    motion_callbacks: MotionCallbacks::default(),
    motion_callback_history: Vec::new(),
    component: None,
    component_source: None,
    component_scope: None,
    memo_value: None,
    provider: None,
    portal: None,
    portal_target: None,
    error_boundary: None,
    element_ref: None,
    drag_constraint_ref: None,
    overlay_reference: None,
    semantic: None,
    suspense: None,
    retained_render: None,
    exit_blueprint: None,
    presence: Some(state),
    children: current,
  });
}

fn mark_pop_layout(position: &mut RenderPosition) {
  if let Some(host) = position.host.as_mut().filter(|host| host.is_ui())
    && let battlement::Prop::Set(descriptor) = &mut ui_host_adapter::element_mut(host)
      .visual_element_mut()
      .motion
  {
    let layout = descriptor
      .layout
      .as_mut()
      .expect("PresenceMode::PopLayout requires layout(...) on the exiting host");
    layout.pop_layout = true;
    return;
  }
  for child in &mut position.children.positions {
    mark_pop_layout(child);
  }
}

fn mark_inert(position: &mut RenderPosition) {
  self::mark_inert_descendants(position, false);
}

fn mark_inert_descendants(position: &mut RenderPosition, inherited: bool) {
  let inherited = if position.portal.is_some() {
    false
  } else {
    inherited
  };
  let mut child_inherited = inherited;
  if let Some(host) = position.host.as_mut().filter(|host| host.is_ui()) {
    let visual = ui_host_adapter::element_mut(host).visual_element_mut();
    if !inherited {
      visual.auto_focus = Prop::Set(false);
      visual.inert = Prop::Set(true);
    }
    child_inherited = true;
    if matches!(
      visual.overlay_placement,
      Prop::Set(OverlayPlacement::Modal { .. })
    ) {
      visual.overlay_placement = Prop::Set(OverlayPlacement::Layer(OverlayLayer::Modal));
      position.overlay_reference = None;
    }
  }
  for child in &mut position.children.positions {
    self::mark_inert_descendants(child, child_inherited);
  }
}

fn validate_keyed(tree: &RenderTree) {
  assert!(
    tree.positions.iter().all(|position| position.key.is_some()),
    "AnimatePresence children must be directly keyed"
  );
}

fn start_exit(
  position: &mut RenderPosition,
  custom: Option<&crate::variant_map::ErasedVariantData>,
) -> (Vec<AutomaticExit>, Vec<PresenceHold>) {
  let mut automatic = Vec::new();
  let mut holds = position
    .component
    .as_ref()
    .map_or_else(Vec::new, |component| {
      component
        .presence_holds()
        .into_iter()
        .map(|cell| (cell, std::rc::Rc::clone(&component.owner)))
        .collect()
    });
  if let (Some(host), Some(blueprint)) = (&mut position.host, &position.exit_blueprint) {
    let object_id = host.object_id;
    let visual = ui_host_adapter::element_mut(host).visual_element_mut();
    if let Prop::Set(previous) = &visual.motion
      && let Some(descriptor) = blueprint.descriptor(object_id, previous, custom)
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
  if let Some(presence) = &position.presence {
    for exit in &presence.exits {
      automatic.extend(exit.automatic.iter().cloned());
      holds.extend(exit.holds.iter().cloned());
    }
  }
  if let Some(suspense) = &mut position.suspense {
    for child in &mut suspense.primary.positions {
      let (child_automatic, child_holds) = start_exit(child, custom);
      automatic.extend(child_automatic);
      holds.extend(child_holds);
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
  if let (Some(current_host), Some(previous_host)) = (
    current.host.as_mut().filter(|host| host.is_ui()),
    previous.host.as_ref().filter(|host| host.is_ui()),
  ) {
    let previous_motion = &ui_host_adapter::element(previous_host)
      .visual_element()
      .motion;
    let exiting = matches!(
      previous_motion,
      Prop::Set(value) if value.slots.iter().any(|slot| slot.layer == MotionLayer::Exit)
    );
    if exiting {
      ui_host_adapter::element_mut(current_host)
        .visual_element_mut()
        .motion = previous_motion.clone();
      current.exit_blueprint = previous.exit_blueprint.clone();
      current.motion_callbacks = previous.motion_callbacks.clone();
      current.motion_callback_history = previous.motion_callback_history.clone();
    }
  }
  for (index, child) in current.children.positions.iter_mut().enumerate() {
    let prior = if child.key.is_some() {
      previous
        .children
        .positions
        .iter()
        .find(|prior| prior.key == child.key && prior.descriptor == child.descriptor)
    } else {
      previous
        .children
        .positions
        .get(index)
        .filter(|prior| prior.key.is_none() && prior.descriptor == child.descriptor)
    };
    if let Some(prior) = prior {
      freeze_exit_motion(child, prior);
    }
  }
}

fn suppress_initial(tree: &mut RenderTree) {
  for position in &mut tree.positions {
    if let Some(host) = position.host.as_mut().filter(|host| host.is_ui())
      && let Prop::Set(descriptor) = &mut ui_host_adapter::element_mut(host)
        .visual_element_mut()
        .motion
    {
      descriptor.initial = None;
      descriptor.initial_disabled = true;
    }
    suppress_initial(&mut position.children);
  }
}
