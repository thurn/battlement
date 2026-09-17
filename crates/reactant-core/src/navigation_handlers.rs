//! Semantic input callbacks for world controls.
use crate::{
  callback::{Callback, IntoCallback},
  event::ReactantEvent,
  event_handler::{Handler, HandlerPhase},
};
use battlement::{NavigationMoveEvent, UiEventBody, UiEventKind};

/// Focus and navigation callbacks propagated through the committed logical tree.
#[derive(Clone, Default)]
pub struct NavigationHandlers {
  pub(crate) handlers: Vec<Handler>,
}

impl NavigationHandlers {
  /// Creates an empty callback set.
  pub fn new() -> Self {
    Self::default()
  }
  /// Whether any callback is installed.
  pub fn is_empty(&self) -> bool {
    self.handlers.is_empty()
  }
  /// Handles coordinate-free activation, independently of pointer clicks.
  pub fn on_activate(mut self, callback: Callback<()>) -> Self {
    self.handlers.push(Handler::semantic_activation(callback));
    self
  }
  /// Observes focus acquisition; use this to render a visible focus indication.
  pub fn on_focus(mut self, callback: Callback<()>) -> Self {
    self.handlers.push(Handler::brief_callback(
      "world_focus",
      UiEventKind::Focus,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::Focus(v) => v,
        _ => unreachable!(),
      },
      callback,
    ));
    self
  }
  /// Observes focus loss while the logical owner remains mounted.
  pub fn on_blur(mut self, callback: Callback<()>) -> Self {
    self.handlers.push(Handler::brief_callback(
      "world_blur",
      UiEventKind::Blur,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::Blur(v) => v,
        _ => unreachable!(),
      },
      callback,
    ));
    self
  }
  /// Handles semantic cancellation without a pointer event.
  pub fn on_cancel(mut self, callback: Callback<()>) -> Self {
    self.handlers.push(Handler::brief_callback(
      "world_cancel",
      UiEventKind::NavigationCancel,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::NavigationCancel(v) => v,
        _ => unreachable!(),
      },
      callback,
    ));
    self
  }
  /// Handles directional intent before native focus movement; default prevention keeps focus.
  pub fn on_navigate<G: 'static>(
    mut self,
    callback: impl IntoCallback<ReactantEvent<NavigationMoveEvent>, G>,
  ) -> Self {
    self.handlers.push(Handler::event_callback(
      "world_navigate",
      UiEventKind::NavigationMove,
      HandlerPhase::Default,
      |body| match body {
        UiEventBody::NavigationMove(v) => v,
        _ => unreachable!(),
      },
      callback.into_callback(),
    ));
    self
  }
}
