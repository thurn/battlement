use std::{
  any::{Any, TypeId},
  rc::Rc,
};

use battlement::{UiEventBody, UiEventKind};

use crate::{
  app_runtime,
  callback::{Callback, Invalidation},
  event::{ElementTarget, EventInner, EventPhase, ReactantEvent, ReactantNativeEvent},
  semantics,
};

#[derive(Clone)]
pub(crate) struct Handler {
  model: Option<TypeId>,
  invalidation: Invalidation,
  slot: &'static str,
  native_kind: UiEventKind,
  phase: HandlerPhase,
  callback: Rc<ErasedHandler>,
  native_callback: Option<Rc<NativeErasedHandler>>,
}

impl Handler {
  pub(crate) fn world_activation(callback: Callback<()>) -> Self {
    Self::brief_callback(
      "world_activation",
      UiEventKind::Click,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::Click(value) => value,
        _ => unreachable!("click handler payload"),
      },
      callback,
    )
  }

  pub(crate) fn native_view_callback<G: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    callback: impl for<'a> Fn(&mut G, ReactantNativeEvent<'a>) + 'static,
  ) -> Self {
    Self {
      model: Some(TypeId::of::<G>()),
      invalidation: Invalidation::Full,
      slot,
      native_kind,
      phase,
      callback: Rc::new(|_, _, _, _, _| {
        panic!("a native-view event handler cannot receive an owned event")
      }),
      native_callback: Some(Rc::new(move |game, target, phase, event, action| {
        callback(
          game
            .downcast_mut::<G>()
            .expect("Reactant native handler model type was validated"),
          ReactantNativeEvent::new(event, action, target, phase),
        );
      })),
    }
  }

  pub(crate) fn brief_callback<E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(&UiEventBody) -> &E,
    callback: Callback<()>,
  ) -> Self {
    let native_callback = callback.clone();
    Self {
      model: callback.model,
      invalidation: callback.invalidation,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, _, _, _, body| {
        let _payload = extract(body.as_ref());
        callback.call(game, ());
      }),
      native_callback: Some(Rc::new(move |game, _, _, _, _| {
        native_callback.call(game, ());
      })),
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
      invalidation: callback.invalidation,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, target, phase, event, body| {
        callback.call(
          game,
          ReactantEvent::new(event, body, extract, target, phase),
        );
      }),
      native_callback: None,
    }
  }

  pub(crate) fn brief_owned_callback<E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(UiEventBody) -> E,
    callback: Callback<()>,
  ) -> Self {
    let native_callback = callback.clone();
    Self {
      model: callback.model,
      invalidation: callback.invalidation,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, _, _, _, body| {
        let _payload = extract(body.as_ref().clone());
        callback.call(game, ());
      }),
      native_callback: Some(Rc::new(move |game, _, _, _, _| {
        native_callback.call(game, ());
      })),
    }
  }

  pub(crate) fn owned_value_callback<E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(UiEventBody) -> E,
    callback: Callback<E>,
  ) -> Self {
    Self {
      model: callback.model,
      invalidation: callback.invalidation,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, _, _, _, body| {
        callback.call(game, extract(body.as_ref().clone()));
      }),
      native_callback: None,
    }
  }

  pub(crate) fn native_value_callback<E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(UiEventBody) -> E,
    extract_native: for<'a> fn(battlement_native::UiEventActionView<'a>) -> E,
    callback: Callback<E>,
  ) -> Self {
    let native_callback = callback.clone();
    Self {
      model: callback.model,
      invalidation: callback.invalidation,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, _, _, _, body| {
        callback.call(game, extract(body.as_ref().clone()));
      }),
      native_callback: Some(Rc::new(move |game, _, _, _, action| {
        native_callback.call(game, extract_native(action));
      })),
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
      invalidation: callback.invalidation,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, target, phase, event, body| {
        callback.call(
          game,
          ReactantEvent::new_owned(event, extract(body.as_ref().clone()), target, phase),
        );
      }),
      native_callback: None,
    }
  }

  pub(crate) fn native_event_owned_callback<E: 'static>(
    slot: &'static str,
    native_kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(UiEventBody) -> E,
    extract_native: for<'a> fn(battlement_native::UiEventActionView<'a>) -> E,
    callback: Callback<ReactantEvent<E>>,
  ) -> Self {
    let native_callback = callback.clone();
    Self {
      model: callback.model,
      invalidation: callback.invalidation,
      slot,
      native_kind,
      phase,
      callback: Rc::new(move |game, target, phase, event, body| {
        callback.call(
          game,
          ReactantEvent::new_owned(event, extract(body.as_ref().clone()), target, phase),
        );
      }),
      native_callback: Some(Rc::new(move |game, target, phase, event, action| {
        native_callback.call(
          game,
          ReactantEvent::new_owned(event, extract_native(action), target, phase),
        );
      })),
    }
  }

  pub(crate) fn accessibility_callback(
    slot: &'static str,
    callback: Callback<battlement::AccessibilityAction>,
  ) -> Self {
    Self {
      model: callback.model,
      invalidation: callback.invalidation,
      slot,
      native_kind: UiEventKind::AccessibilityAction,
      phase: HandlerPhase::Default,
      callback: Rc::new(move |game, target, phase, event, body| {
        let event = ReactantEvent::new(
          event,
          body,
          |body| match body {
            UiEventBody::AccessibilityAction(value) => value,
            _ => panic!("ControlBehavior callback received another event"),
          },
          target,
          phase,
        );
        if callback.call(game, semantics::to_ui_action(event.payload().action)) {
          event.prevent_default();
        }
      }),
      native_callback: None,
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
    app_runtime::callback(|| (self.callback)(game, current_target, phase, event, body));
  }

  pub(crate) fn supports_native_view(&self) -> bool {
    self.native_callback.is_some()
  }

  pub(crate) fn invoke_native_view(
    &self,
    game: &mut dyn Any,
    current_target: ElementTarget,
    phase: EventPhase,
    event: Rc<EventInner>,
    action: battlement_native::UiEventActionView<'_>,
  ) {
    app_runtime::callback(|| {
      self
        .native_callback
        .as_ref()
        .expect("native event callback support was checked")(
        game,
        current_target,
        phase,
        event,
        action,
      );
    });
  }

  pub(crate) fn model(&self) -> Option<TypeId> {
    self.model
  }

  pub(crate) fn has_local_invalidation(&self) -> bool {
    self.invalidation == Invalidation::LocalState
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

pub(crate) const fn native_slot(kind: UiEventKind) -> &'static str {
  match kind {
    UiEventKind::AccessibilityAction => "accessibility_action",
    UiEventKind::PointerDown => "pointer_down",
    UiEventKind::PointerMove => "pointer_move",
    UiEventKind::PointerUp => "pointer_up",
    UiEventKind::PointerCancel => "pointer_cancel",
    UiEventKind::Click => "click",
    UiEventKind::PointerEnter => "pointer_enter",
    UiEventKind::PointerLeave => "pointer_leave",
    UiEventKind::PointerOver => "pointer_over",
    UiEventKind::PointerOut => "pointer_out",
    UiEventKind::Wheel => "wheel",
    UiEventKind::PointerCapture => "pointer_capture",
    UiEventKind::PointerCaptureOut => "pointer_capture_out",
    UiEventKind::KeyDown => "key_down",
    UiEventKind::KeyUp => "key_up",
    UiEventKind::NavigationMove => "navigation_move",
    UiEventKind::NavigationCancel => "navigation_cancel",
    UiEventKind::FocusIn => "focus_in",
    UiEventKind::Focus => "focus",
    UiEventKind::FocusOut => "focus_out",
    UiEventKind::Blur => "blur",
    UiEventKind::GeometryChanged => "geometry_changed",
    UiEventKind::AttachToPanel => "attach_to_panel",
    UiEventKind::DetachFromPanel => "detach_from_panel",
    UiEventKind::TransitionStart => "transition_start",
    UiEventKind::TransitionEnd => "transition_end",
    UiEventKind::TransitionCancel => "transition_cancel",
    UiEventKind::ValueChanging => "value_changing",
    UiEventKind::ValueCommitted => "value_committed",
    UiEventKind::Input => "input",
    UiEventKind::SelectionChanged => "selection_changed",
    UiEventKind::LinkEnter => "link_enter",
    UiEventKind::LinkLeave => "link_leave",
    UiEventKind::LinkDown => "link_down",
    UiEventKind::LinkUp => "link_up",
    UiEventKind::ScrollSettled => "scroll_settled",
    UiEventKind::ScrollChanged => "scroll_changed",
    UiEventKind::TabSelectionRequested => "tab_selection_requested",
    UiEventKind::TabCloseRequested => "tab_close_requested",
    UiEventKind::TabReorderRequested => "tab_reorder_requested",
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum HandlerPhase {
  Capture,
  Default,
}

type ErasedHandler =
  dyn Fn(&mut dyn Any, ElementTarget, EventPhase, Rc<EventInner>, Rc<UiEventBody>);

type NativeErasedHandler = dyn for<'a> Fn(
  &mut dyn Any,
  ElementTarget,
  EventPhase,
  Rc<EventInner>,
  battlement_native::UiEventActionView<'a>,
);
