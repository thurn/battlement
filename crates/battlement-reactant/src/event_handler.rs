use std::{
  any::{Any, TypeId},
  rc::Rc,
};

use battlement::{UiEventBody, UiEventKind};

use crate::{
  callback::Callback,
  event::{ElementTarget, EventInner, EventPhase, ReactantEvent},
  semantics,
};

#[derive(Clone)]
pub(crate) struct Handler {
  model: Option<TypeId>,
  slot: &'static str,
  native_kind: UiEventKind,
  phase: HandlerPhase,
  callback: Rc<ErasedHandler>,
}

impl Handler {
  pub(crate) fn brief_callback<E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(&UiEventBody) -> &E,
    callback: Callback<()>,
  ) -> Self {
    Self {
      model: callback.model,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, _, _, _, body| {
        let _payload = extract(body.as_ref());
        callback.call(game, ());
      }),
    }
  }

  pub(crate) fn event_callback<E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(&UiEventBody) -> &E,
    callback: Callback<ReactantEvent<E>>,
  ) -> Self {
    Self {
      model: callback.model,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, target, phase, event, body| {
        callback.call(
          game,
          ReactantEvent::new(event, body, extract, target, phase),
        );
      }),
    }
  }

  pub(crate) fn brief_owned_callback<E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(UiEventBody) -> E,
    callback: Callback<()>,
  ) -> Self {
    Self {
      model: callback.model,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, _, _, _, body| {
        let _payload = extract(body.as_ref().clone());
        callback.call(game, ());
      }),
    }
  }

  pub(crate) fn event_owned_callback<E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(UiEventBody) -> E,
    callback: Callback<ReactantEvent<E>>,
  ) -> Self {
    Self {
      model: callback.model,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, target, phase, event, body| {
        callback.call(
          game,
          ReactantEvent::new_owned(event, extract(body.as_ref().clone()), target, phase),
        );
      }),
    }
  }

  pub(crate) fn accessibility_callback(
    slot: &'static str,
    callback: Callback<battlement::AccessibilityAction>,
  ) -> Self {
    Self {
      model: callback.model,
      slot,
      native_kind: UiEventKind::AccessibilityAction,
      phase: HandlerPhase::Default,
      callback: Rc::new(move |game, target, phase, event, body| {
        let event = ReactantEvent::new(
          event,
          body,
          |body| match body {
            UiEventBody::AccessibilityAction(value) => value,
            _ => panic!("accessibility callback received another event"),
          },
          target,
          phase,
        );
        if callback.call(game, semantics::to_ui_action(event.payload().action)) {
          event.prevent_default();
        }
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

  pub(crate) fn model(&self) -> Option<TypeId> {
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
