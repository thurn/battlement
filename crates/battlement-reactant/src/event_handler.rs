use std::{
  any::{Any, TypeId},
  rc::Rc,
};

use battlement::{UiEventBody, UiEventKind};

use crate::event::{ElementTarget, EventPhase, ReactantEvent};

#[derive(Clone)]
pub(crate) struct Handler {
  model: TypeId,
  slot: &'static str,
  native_kind: UiEventKind,
  phase: HandlerPhase,
  callback: Rc<ErasedHandler>,
}

impl Handler {
  pub(crate) fn brief<G: 'static, E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(UiEventBody) -> E,
    callback: impl Fn(&mut G) + 'static,
  ) -> Self {
    Self {
      model: TypeId::of::<G>(),
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, _target, _event_phase, body| {
        let _payload = extract(body);
        callback(
          game
            .downcast_mut::<G>()
            .expect("Reactant handler model type was not validated"),
        );
      }),
    }
  }

  pub(crate) fn event<G: 'static, E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(UiEventBody) -> E,
    callback: impl Fn(&mut G, ReactantEvent<E>) + 'static,
  ) -> Self {
    Self {
      model: TypeId::of::<G>(),
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, target, event_phase, body| {
        callback(
          game
            .downcast_mut::<G>()
            .expect("Reactant handler model type was not validated"),
          ReactantEvent::new(extract(body), target, target, event_phase),
        );
      }),
    }
  }

  pub(crate) fn invoke(
    &self,
    game: &mut dyn Any,
    target: ElementTarget,
    phase: EventPhase,
    body: UiEventBody,
  ) {
    (self.callback)(game, target, phase, body);
  }

  pub(crate) fn model(&self) -> TypeId {
    self.model
  }

  pub(crate) fn native_kind(&self) -> UiEventKind {
    self.native_kind
  }

  pub(crate) fn phase(&self) -> HandlerPhase {
    self.phase
  }

  pub(crate) fn same_slot(&self, other: &Self) -> bool {
    self.slot == other.slot && self.phase == other.phase
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum HandlerPhase {
  Capture,
  Default,
}

type ErasedHandler = dyn Fn(&mut dyn Any, ElementTarget, EventPhase, UiEventBody);
