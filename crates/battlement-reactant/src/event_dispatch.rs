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
  pub(crate) local_invalidation: bool,
  pub(crate) prevented_by_reactant: bool,
}

#[derive(Default)]
struct HandlerInvocations {
  invoked: bool,
  local_only: bool,
}

impl HandlerInvocations {
  fn merge(&mut self, other: Self) {
    if other.invoked {
      self.local_only = (!self.invoked || self.local_only) && other.local_only;
      self.invoked = true;
    }
  }
}

pub(crate) fn dispatch<G: 'static>(
  runtime_id: u64,
  roots: &[&RenderTree],
  game: &mut G,
  event: UiEvent,
) -> DispatchResult {
  self::invoke_raw(runtime_id, roots, game, event)
}

pub(crate) fn dispatch_view<G: 'static>(
  runtime_id: u64,
  roots: &[&RenderTree],
  game: &mut G,
  action: battlement_native::UiEventActionView<'_>,
) -> DispatchResult {
  let incoming_prevented = action.default_prevented();
  let target_id = ObjectId::from_uuid(uuid::Uuid::from_bytes(action.target_id()))
    .expect("UI event view validates nonzero target UUIDs");
  let Some(path) = self::logical_path(runtime_id, roots, target_id) else {
    return DispatchResult {
      disposition: disposition(incoming_prevented),
      invoked: false,
      local_invalidation: false,
      prevented_by_reactant: false,
    };
  };
  let kind = action.event_kind();
  let requires_owned_body = path.iter().any(|node| {
    node
      .handlers
      .iter()
      .any(|handler| handler.native_kind() == kind && !handler.supports_native_view())
  });
  let owned_body = requires_owned_body.then(|| {
    Rc::new(
      action
        .to_owned_event()
        .expect("verified UI event view must produce a legacy callback body")
        .body,
    )
  });
  let stopped = Rc::new(Cell::new(false));
  let target_node = path.last().expect("event path has a target");
  let shared = Rc::new(EventInner::new(
    target_node.target,
    stopped,
    action.cancelable(),
    incoming_prevented,
  ));
  if !kind.propagates() {
    let invoked = self::invoke_view_handlers(
      game,
      target_node,
      EventPhase::Target,
      HandlerPhase::Default,
      kind,
      Rc::clone(&shared),
      action,
      owned_body,
    );
    return DispatchResult {
      disposition: disposition(shared.default_prevented()),
      invoked: invoked.invoked,
      local_invalidation: invoked.invoked && invoked.local_only,
      prevented_by_reactant: shared.prevented_by_reactant(),
    };
  }
  let mut invoked = HandlerInvocations::default();
  for node in &path[..path.len() - 1] {
    invoked.merge(self::invoke_view_handlers(
      game,
      node,
      EventPhase::Capture,
      HandlerPhase::Capture,
      kind,
      Rc::clone(&shared),
      action,
      owned_body.as_ref().map(Rc::clone),
    ));
  }
  invoked.merge(self::invoke_view_handlers(
    game,
    target_node,
    EventPhase::Target,
    HandlerPhase::Capture,
    kind,
    Rc::clone(&shared),
    action,
    owned_body.as_ref().map(Rc::clone),
  ));
  invoked.merge(self::invoke_view_handlers(
    game,
    target_node,
    EventPhase::Target,
    HandlerPhase::Default,
    kind,
    Rc::clone(&shared),
    action,
    owned_body.as_ref().map(Rc::clone),
  ));
  for node in path[..path.len() - 1].iter().rev() {
    invoked.merge(self::invoke_view_handlers(
      game,
      node,
      EventPhase::Bubble,
      HandlerPhase::Default,
      kind,
      Rc::clone(&shared),
      action,
      owned_body.as_ref().map(Rc::clone),
    ));
  }
  DispatchResult {
    disposition: disposition(shared.default_prevented()),
    invoked: invoked.invoked,
    local_invalidation: invoked.invoked && invoked.local_only,
    prevented_by_reactant: shared.prevented_by_reactant(),
  }
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
      local_invalidation: false,
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
      invoked: invoked.invoked,
      local_invalidation: invoked.invoked && invoked.local_only,
      prevented_by_reactant: shared.prevented_by_reactant(),
    };
  }
  let mut invoked = HandlerInvocations::default();
  for node in &path[..path.len() - 1] {
    invoked.merge(self::invoke_raw_handlers(
      game,
      node,
      EventPhase::Capture,
      HandlerPhase::Capture,
      kind,
      Rc::clone(&shared),
      Rc::clone(&body),
    ));
  }
  invoked.merge(self::invoke_raw_handlers(
    game,
    target_node,
    EventPhase::Target,
    HandlerPhase::Capture,
    kind,
    Rc::clone(&shared),
    Rc::clone(&body),
  ));
  invoked.merge(self::invoke_raw_handlers(
    game,
    target_node,
    EventPhase::Target,
    HandlerPhase::Default,
    kind,
    Rc::clone(&shared),
    Rc::clone(&body),
  ));
  for node in path[..path.len() - 1].iter().rev() {
    invoked.merge(self::invoke_raw_handlers(
      game,
      node,
      EventPhase::Bubble,
      HandlerPhase::Default,
      kind,
      Rc::clone(&shared),
      Rc::clone(&body),
    ));
  }
  DispatchResult {
    disposition: disposition(shared.default_prevented()),
    invoked: invoked.invoked,
    local_invalidation: invoked.invoked && invoked.local_only,
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
) -> HandlerInvocations {
  let mut invoked = HandlerInvocations::default();
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
    invoked.local_only =
      (!invoked.invoked || invoked.local_only) && handler.has_local_invalidation();
    invoked.invoked = true;
  }
  invoked
}

#[allow(clippy::too_many_arguments)]
fn invoke_view_handlers<G: 'static>(
  game: &mut G,
  node: &LogicalNode,
  phase: EventPhase,
  handler_phase: HandlerPhase,
  kind: UiEventKind,
  event: Rc<EventInner>,
  action: battlement_native::UiEventActionView<'_>,
  owned_body: Option<Rc<UiEventBody>>,
) -> HandlerInvocations {
  let mut invoked = HandlerInvocations::default();
  for handler in &node.handlers {
    if event.propagation_stopped() {
      break;
    }
    if handler.native_kind() != kind || handler.phase() != handler_phase {
      continue;
    }
    if handler.supports_native_view() {
      handler.invoke_native_view(game, node.target, phase, Rc::clone(&event), action);
    } else {
      handler.invoke(
        game,
        node.target,
        phase,
        Rc::clone(&event),
        owned_body
          .as_ref()
          .map(Rc::clone)
          .expect("legacy native handler requires an owned callback body"),
      );
    }
    invoked.local_only =
      (!invoked.invoked || invoked.local_only) && handler.has_local_invalidation();
    invoked.invoked = true;
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
