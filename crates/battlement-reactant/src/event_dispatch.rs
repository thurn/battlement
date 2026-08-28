use std::{cell::Cell, rc::Rc};

use battlement::{ObjectId, PointerCrossingEvent, UiEvent, UiEventBody, UiEventKind};

use crate::{
  event::{ElementTarget, EventInner, EventPhase},
  event_handler::{Handler, HandlerPhase, SyntheticHandler},
  render::{EventNode, RenderTree},
  runtime::Root,
};

#[derive(Clone, Copy)]
pub(crate) struct CrossingCandidate {
  kind: UiEventKind,
  pointer_id: i32,
  target_id: ObjectId,
  related_target_id: Option<ObjectId>,
}

pub(crate) fn dispatch<G: 'static>(
  runtime_id: u64,
  roots: &[&RenderTree],
  crossing_candidate: &mut Option<CrossingCandidate>,
  game: &mut G,
  event: UiEvent,
) -> bool {
  let crossing = Crossing::from_event(&event);
  let previous = crossing_candidate.take();
  let duplicate =
    crossing.is_some_and(|value| previous.is_some_and(|candidate| candidate.is_complement(value)));
  let mut invoked = false;
  if let Some(crossing) = crossing {
    if !duplicate {
      invoked |= self::invoke_crossing(runtime_id, roots, game, crossing);
      *crossing_candidate = Some(CrossingCandidate::new(crossing));
    }
  }
  let raw_invoked = self::invoke_raw(runtime_id, roots, game, event);
  invoked || raw_invoked
}

#[derive(Clone)]
struct LogicalNode {
  target: ElementTarget,
  handlers: Vec<Handler>,
}

#[derive(Clone, Copy)]
struct Crossing {
  kind: UiEventKind,
  payload: PointerCrossingEvent,
  target_id: ObjectId,
}

impl CrossingCandidate {
  fn new(crossing: Crossing) -> Self {
    Self {
      kind: crossing.kind,
      pointer_id: crossing.payload.pointer_id,
      target_id: crossing.target_id,
      related_target_id: crossing.payload.related_target_id,
    }
  }

  fn is_complement(self, crossing: Crossing) -> bool {
    if self.kind == crossing.kind || self.pointer_id != crossing.payload.pointer_id {
      return false;
    }
    if Some(self.target_id) != crossing.payload.related_target_id {
      return false;
    }
    self.related_target_id == Some(crossing.target_id)
  }
}

impl Crossing {
  fn from_event(event: &UiEvent) -> Option<Self> {
    match event.body {
      UiEventBody::PointerOver(payload) => Some(Self {
        kind: UiEventKind::PointerOver,
        payload,
        target_id: event.target_id,
      }),
      UiEventBody::PointerOut(payload) => Some(Self {
        kind: UiEventKind::PointerOut,
        payload,
        target_id: event.target_id,
      }),
      _ => None,
    }
  }

  fn old_id(self) -> Option<ObjectId> {
    match self.kind {
      UiEventKind::PointerOut => Some(self.target_id),
      UiEventKind::PointerOver => self.payload.related_target_id,
      _ => unreachable!("crossing kind is pointer over or out"),
    }
  }

  fn new_id(self) -> Option<ObjectId> {
    match self.kind {
      UiEventKind::PointerOut => self.payload.related_target_id,
      UiEventKind::PointerOver => Some(self.target_id),
      _ => unreachable!("crossing kind is pointer over or out"),
    }
  }
}

fn invoke_raw<G: 'static>(
  runtime_id: u64,
  roots: &[&RenderTree],
  game: &mut G,
  event: UiEvent,
) -> bool {
  let Some(path) = self::logical_path(runtime_id, roots, event.target_id) else {
    return false;
  };
  let stopped = Rc::new(Cell::new(false));
  let target_node = path.last().expect("event path has a target");
  let kind = event.kind();
  let body = Rc::new(event.body);
  let shared = Rc::new(EventInner::new(target_node.target, stopped));
  if !kind.propagates() {
    return self::invoke_raw_handlers(
      game,
      target_node,
      EventPhase::Target,
      HandlerPhase::Default,
      kind,
      shared,
      body,
    );
  }
  let mut invoked = false;
  for node in &path[..path.len() - 1] {
    invoked |= self::invoke_raw_handlers(
      game,
      node,
      EventPhase::Capture,
      HandlerPhase::Capture,
      kind,
      Rc::clone(&shared),
      Rc::clone(&body),
    );
  }
  invoked |= self::invoke_raw_handlers(
    game,
    target_node,
    EventPhase::Target,
    HandlerPhase::Capture,
    kind,
    Rc::clone(&shared),
    Rc::clone(&body),
  );
  invoked |= self::invoke_raw_handlers(
    game,
    target_node,
    EventPhase::Target,
    HandlerPhase::Default,
    kind,
    Rc::clone(&shared),
    Rc::clone(&body),
  );
  for node in path[..path.len() - 1].iter().rev() {
    invoked |= self::invoke_raw_handlers(
      game,
      node,
      EventPhase::Bubble,
      HandlerPhase::Default,
      kind,
      Rc::clone(&shared),
      Rc::clone(&body),
    );
  }
  invoked
}

#[allow(clippy::too_many_arguments)]
fn invoke_raw_handlers<G: 'static>(
  game: &mut G,
  node: &LogicalNode,
  phase: EventPhase,
  handler_phase: HandlerPhase,
  kind: UiEventKind,
  event: Rc<EventInner>,
  body: Rc<UiEventBody>,
) -> bool {
  let mut invoked = false;
  for handler in &node.handlers {
    if event.propagation_stopped() {
      break;
    }
    if handler.native_kind() != kind || handler.synthetic().is_some() {
      continue;
    }
    if handler.phase() != handler_phase {
      continue;
    }
    handler.invoke(
      game,
      node.target,
      phase,
      Rc::clone(&event),
      Rc::clone(&body),
    );
    invoked = true;
  }
  invoked
}

fn invoke_crossing<G: 'static>(
  runtime_id: u64,
  roots: &[&RenderTree],
  game: &mut G,
  crossing: Crossing,
) -> bool {
  let old_path = crossing
    .old_id()
    .and_then(|target| self::logical_path(runtime_id, roots, target))
    .unwrap_or_default();
  let new_path = crossing
    .new_id()
    .and_then(|target| self::logical_path(runtime_id, roots, target))
    .unwrap_or_default();
  let common = old_path
    .iter()
    .zip(&new_path)
    .take_while(|(old, new)| old.target == new.target)
    .count();
  let stopped = Rc::new(Cell::new(false));
  let mut invoked = false;
  let mut leave_payload = crossing.payload;
  leave_payload.related_target_id = crossing.new_id();
  let leave = Rc::new(UiEventBody::PointerOut(leave_payload));
  for node in old_path[common..].iter().rev() {
    invoked |= self::invoke_synthetic_handlers(
      game,
      node,
      SyntheticHandler::PointerLeave,
      Rc::clone(&leave),
      Rc::clone(&stopped),
    );
  }
  let mut enter_payload = crossing.payload;
  enter_payload.related_target_id = crossing.old_id();
  let enter = Rc::new(UiEventBody::PointerOver(enter_payload));
  for node in &new_path[common..] {
    invoked |= self::invoke_synthetic_handlers(
      game,
      node,
      SyntheticHandler::PointerEnter,
      Rc::clone(&enter),
      Rc::clone(&stopped),
    );
  }
  invoked
}

fn invoke_synthetic_handlers<G: 'static>(
  game: &mut G,
  node: &LogicalNode,
  synthetic: SyntheticHandler,
  body: Rc<UiEventBody>,
  stopped: Rc<Cell<bool>>,
) -> bool {
  let mut invoked = false;
  let event = Rc::new(EventInner::new(node.target, stopped));
  for handler in &node.handlers {
    if event.propagation_stopped() {
      break;
    }
    if handler.synthetic() != Some(synthetic) {
      continue;
    }
    handler.invoke(
      game,
      node.target,
      EventPhase::Target,
      Rc::clone(&event),
      Rc::clone(&body),
    );
    invoked = true;
  }
  invoked
}

fn logical_path(
  runtime_id: u64,
  roots: &[&RenderTree],
  target_id: ObjectId,
) -> Option<Vec<LogicalNode>> {
  roots.iter().enumerate().find_map(|(index, tree)| {
    tree.event_path(target_id).map(|path| {
      let root = Root::new(runtime_id, index);
      path
        .into_iter()
        .map(|node: EventNode| LogicalNode {
          target: ElementTarget::new(root, node.object_id),
          handlers: node.handlers,
        })
        .collect()
    })
  })
}
