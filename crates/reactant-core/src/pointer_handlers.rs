//! Logical pointer callbacks shared by native world hosts and UI ancestors.
use crate::{
  callback::IntoCallback,
  event::ReactantEvent,
  event_handler::{self, Handler, HandlerPhase},
};
use battlement::{
  ClickEvent, PointerBoundaryEvent, PointerButtonEvent, PointerCancelEvent, PointerCaptureEvent,
  PointerCrossingEvent, PointerMoveEvent, UiEventBody, UiEventKind,
};

/// Typed pointer callbacks propagated along the committed logical ancestry.
#[derive(Clone, Default)]
pub struct PointerHandlers {
  pub(crate) handlers: Vec<Handler>,
}

impl PointerHandlers {
  /// Creates an empty callback set.
  pub fn new() -> Self {
    Self::default()
  }
  /// Whether any callback is installed.
  pub fn is_empty(&self) -> bool {
    self.handlers.is_empty()
  }
  /// Replaces the pointer down default callback.
  pub fn on_pointer_down<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerButtonEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerDown,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerDown(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer down capture callback.
  pub fn on_pointer_down_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerButtonEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerDown,
      HandlerPhase::Capture,
      |body| match body {
        UiEventBody::PointerDown(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer move default callback.
  pub fn on_pointer_move<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerMoveEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerMove,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerMove(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer move capture callback.
  pub fn on_pointer_move_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerMoveEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerMove,
      HandlerPhase::Capture,
      |body| match body {
        UiEventBody::PointerMove(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer up default callback.
  pub fn on_pointer_up<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerButtonEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerUp,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerUp(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer up capture callback.
  pub fn on_pointer_up_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerButtonEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerUp,
      HandlerPhase::Capture,
      |body| match body {
        UiEventBody::PointerUp(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer cancel default callback.
  pub fn on_pointer_cancel<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCancelEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerCancel,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerCancel(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer cancel capture callback.
  pub fn on_pointer_cancel_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCancelEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerCancel,
      HandlerPhase::Capture,
      |body| match body {
        UiEventBody::PointerCancel(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer enter default callback.
  pub fn on_pointer_enter<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerBoundaryEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerEnter,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerEnter(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer leave default callback.
  pub fn on_pointer_leave<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerBoundaryEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerLeave,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerLeave(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer over default callback.
  pub fn on_pointer_over<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCrossingEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerOver,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerOver(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer over capture callback.
  pub fn on_pointer_over_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCrossingEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerOver,
      HandlerPhase::Capture,
      |body| match body {
        UiEventBody::PointerOver(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer out default callback.
  pub fn on_pointer_out<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCrossingEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerOut,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerOut(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer out capture callback.
  pub fn on_pointer_out_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCrossingEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerOut,
      HandlerPhase::Capture,
      |body| match body {
        UiEventBody::PointerOut(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer capture default callback.
  pub fn on_pointer_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCaptureEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerCapture,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerCapture(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer capture capture callback.
  pub fn on_pointer_capture_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCaptureEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerCapture,
      HandlerPhase::Capture,
      |body| match body {
        UiEventBody::PointerCapture(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer capture out default callback.
  pub fn on_pointer_capture_out<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCaptureEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerCaptureOut,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::PointerCaptureOut(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the pointer capture out capture callback.
  pub fn on_pointer_capture_out_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<PointerCaptureEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::PointerCaptureOut,
      HandlerPhase::Capture,
      |body| match body {
        UiEventBody::PointerCaptureOut(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the click default callback.
  pub fn on_click<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<ClickEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::Click,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::Click(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  /// Replaces the click capture callback.
  pub fn on_click_capture<G: 'static>(
    self,
    callback: impl IntoCallback<ReactantEvent<ClickEvent>, G>,
  ) -> Self {
    self.with(
      UiEventKind::Click,
      HandlerPhase::Capture,
      |body| match body {
        UiEventBody::Click(value) => value,
        _ => unreachable!("pointer callback kind"),
      },
      callback,
    )
  }
  fn with<E: 'static, G: 'static>(
    mut self,
    kind: UiEventKind,
    phase: HandlerPhase,
    extract: fn(&UiEventBody) -> &E,
    callback: impl IntoCallback<ReactantEvent<E>, G>,
  ) -> Self {
    self
      .handlers
      .retain(|h| h.native_kind() != kind || h.phase() != phase);
    self.handlers.push(Handler::event_callback(
      event_handler::native_slot(kind),
      kind,
      phase,
      extract,
      callback.into_callback(),
    ));
    self
  }
}
