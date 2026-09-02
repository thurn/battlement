use std::{
  any::{Any, TypeId},
  rc::Rc,
};

use battlement::{UiEventBody, UiEventKind};

use crate::event::{ElementTarget, EventInner, EventPhase, ReactantEvent};

#[derive(Clone)]
pub(crate) struct Handler {
  model: TypeId,
  slot: &'static str,
  native_kind: UiEventKind,
  phase: HandlerPhase,
  callback: Rc<ErasedHandler>,
}

impl Handler {
  pub(crate) fn accessibility<G: 'static>(
    slot: &'static str,
    callback: impl Fn(&mut G, battlement::AccessibilityAction) -> crate::semantics::ActionDisposition
    + 'static,
  ) -> Self {
    Self::event(
      slot,
      UiEventKind::AccessibilityAction,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::AccessibilityAction(value) => value,
        _ => panic!("Reactant accessibility handler received another event kind"),
      },
      move |game, event| {
        if callback(game, crate::semantics::to_ui_action(event.payload().action))
          == crate::semantics::ActionDisposition::Handled
        {
          event.prevent_default();
        }
      },
    )
  }

  pub(crate) fn brief<G: 'static, E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(&UiEventBody) -> &E,
    callback: impl Fn(&mut G) + 'static,
  ) -> Self {
    Self {
      model: TypeId::of::<G>(),
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, _current_target, _event_phase, _event, body| {
        let _payload = extract(body.as_ref());
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
    extract: fn(&UiEventBody) -> &E,
    callback: impl Fn(&mut G, ReactantEvent<E>) + 'static,
  ) -> Self {
    Self {
      model: TypeId::of::<G>(),
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, current_target, event_phase, event, body| {
        callback(
          game
            .downcast_mut::<G>()
            .expect("Reactant handler model type was not validated"),
          ReactantEvent::new(event, body, extract, current_target, event_phase),
        );
      }),
    }
  }

  pub(crate) fn brief_owned<G: 'static, E: 'static>(
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
      callback: Rc::new(move |game, _current_target, _event_phase, _event, body| {
        let _payload = extract(body.as_ref().clone());
        callback(
          game
            .downcast_mut::<G>()
            .expect("Reactant handler model type was not validated"),
        );
      }),
    }
  }

  pub(crate) fn event_owned<G: 'static, E: 'static>(
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
      callback: Rc::new(move |game, current_target, event_phase, event, body| {
        callback(
          game
            .downcast_mut::<G>()
            .expect("Reactant handler model type was not validated"),
          ReactantEvent::new_owned(
            event,
            extract(body.as_ref().clone()),
            current_target,
            event_phase,
          ),
        );
      }),
    }
  }

  pub(crate) fn invoke(
    &self,
    game: &mut dyn Any,
    current_target: ElementTarget,
    phase: EventPhase,
    event: Rc<EventInner>,
    body: Rc<UiEventBody>,
  ) {
    (self.callback)(game, current_target, phase, event, body);
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

type ErasedHandler =
  dyn Fn(&mut dyn Any, ElementTarget, EventPhase, Rc<EventInner>, Rc<UiEventBody>);
