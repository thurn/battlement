use std::{cell::Cell, rc::Rc};

use battlement::{ObjectId, UiEvent, UiEventBody, UiEventDisposition, UiEventKind};

use crate::{
  event::{ElementTarget, EventInner, EventPhase},
  event_handler::{Handler, HandlerPhase},
  render::RenderTree,
  runtime::Root,
};

#[derive(Clone)]
pub(crate) struct EventNode {
  pub(crate) object_id: battlement::ObjectId,
  pub(crate) handlers: Vec<crate::event_handler::Handler>,
}

pub(crate) struct DispatchResult {
  pub(crate) disposition: UiEventDisposition,
  pub(crate) invoked: bool,
  pub(crate) prevented_by_reactant: bool,
}

pub(crate) fn dispatch<G: 'static>(
  runtime_id: u64,
  roots: &[&RenderTree],
  game: &mut G,
  event: UiEvent,
) -> DispatchResult {
  self::invoke_raw(runtime_id, roots, game, event)
}

#[derive(Clone)]
struct LogicalNode {
  target: ElementTarget,
  handlers: Vec<Handler>,
}

fn invoke_raw<G: 'static>(
  runtime_id: u64,
  roots: &[&RenderTree],
  game: &mut G,
  event: UiEvent,
) -> DispatchResult {
  let incoming_prevented = event.default_prevented;
  let Some(path) = self::logical_path(runtime_id, roots, event.target_id) else {
    return DispatchResult {
      disposition: disposition(incoming_prevented),
      invoked: false,
      prevented_by_reactant: false,
    };
  };
  let stopped = Rc::new(Cell::new(false));
  let target_node = path.last().expect("event path has a target");
  let kind = event.kind();
  let body = Rc::new(event.body);
  let shared = Rc::new(EventInner::new(
    target_node.target,
    stopped,
    event.cancelable,
    incoming_prevented,
  ));
  if !kind.propagates() {
    let invoked = self::invoke_raw_handlers(
      game,
      target_node,
      EventPhase::Target,
      HandlerPhase::Default,
      kind,
      Rc::clone(&shared),
      body,
    );
    return DispatchResult {
      disposition: disposition(shared.default_prevented()),
      invoked,
      prevented_by_reactant: shared.prevented_by_reactant(),
    };
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
  DispatchResult {
    disposition: disposition(shared.default_prevented()),
    invoked,
    prevented_by_reactant: shared.prevented_by_reactant(),
  }
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
    if handler.native_kind() != kind {
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

fn disposition(prevented: bool) -> UiEventDisposition {
  if prevented {
    UiEventDisposition::PreventDefault
  } else {
    UiEventDisposition::Continue
  }
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
