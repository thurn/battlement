//! Typed application event handlers and event views.

#![allow(private_bounds, private_interfaces)]

use std::{
  any::{Any, TypeId},
  cell::Cell,
  rc::Rc,
};

use battlement::{
  Box as UiBox, Button, ClickEvent, DropdownField, GroupBox, Image, Label, MinMaxSlider, ObjectId,
  PopupWindow, ProgressBar, RadioButton, RadioButtonGroup, RepeatButton, ScrollView, Scroller,
  Slider, SliderInt, Tab, TabView, TextElement, TextField, Toggle, ToggleButtonGroup, UiEventBody,
  UiEventKind, VisualElement,
};

use crate::{
  primitive::Children,
  render::{Render, RenderSink, private::Sealed},
  runtime::Root,
};

/// Adds typed event handlers to a host render value.
pub trait EventRenderExt: EventHost + Sized {
  /// Replaces the click handler with a payload-free callback.
  fn on_click<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> EventHandler<Self> {
    EventHandler::new(self, Handler::click(callback))
  }

  /// Replaces the click handler with a typed event-aware callback.
  fn on_click_event<G: 'static>(
    self,
    callback: impl Fn(&mut G, ReactantEvent<ClickEvent>) + 'static,
  ) -> EventHandler<Self> {
    EventHandler::new(self, Handler::click_event(callback))
  }
}

/// The logical phase observed by one Reactant handler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventPhase {
  /// Logical capture traversal.
  Capture,
  /// The originating target.
  Target,
  /// Logical bubble traversal.
  Bubble,
}

/// Identifies a logical host at the time an event was dispatched.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElementTarget {
  root: Root,
  object_id: ObjectId,
}

/// A typed view of one shared event dispatch.
pub struct ReactantEvent<E> {
  inner: Rc<EventInner<E>>,
  current_target: ElementTarget,
  phase: EventPhase,
}

/// A host render value carrying an event handler slot.
pub struct EventHandler<R> {
  render: R,
  handler: Handler,
}

impl ElementTarget {
  /// Returns the event-time native host identity.
  #[must_use]
  pub const fn object_id(self) -> ObjectId {
    self.object_id
  }

  /// Returns the logical source root.
  #[must_use]
  pub const fn root(self) -> Root {
    self.root
  }

  pub(crate) const fn new(root: Root, object_id: ObjectId) -> Self {
    Self { root, object_id }
  }
}

impl<E> ReactantEvent<E> {
  /// Returns the event-family-specific payload.
  #[must_use]
  pub fn payload(&self) -> &E {
    &self.inner.payload
  }

  /// Returns the original logical target.
  #[must_use]
  pub fn target(&self) -> ElementTarget {
    self.inner.target
  }

  /// Returns the host whose callback is currently running.
  #[must_use]
  pub fn current_target(&self) -> ElementTarget {
    self.current_target
  }

  /// Returns the logical route phase.
  #[must_use]
  pub fn phase(&self) -> EventPhase {
    self.phase
  }

  /// Stops later logical callbacks for this dispatch.
  pub fn stop_propagation(&self) {
    self.inner.propagation_stopped.set(true);
  }
}

impl<E> Clone for ReactantEvent<E> {
  fn clone(&self) -> Self {
    Self {
      inner: Rc::clone(&self.inner),
      current_target: self.current_target,
      phase: self.phase,
    }
  }
}

impl<R> EventHandler<R> {
  fn new(render: R, handler: Handler) -> Self {
    Self { render, handler }
  }
}

impl<R: EventHost> EventRenderExt for R {}
impl<R: EventHost> EventHost for EventHandler<R> {}
impl<R: EventHost> Render for EventHandler<R> {}

impl<R: EventHost> Sealed for EventHandler<R> {
  fn descriptor(&self) -> TypeId {
    self.render.descriptor()
  }

  fn render_into(&self, sink: &mut RenderSink<'_>) {
    sink.with_handler(self.handler.clone(), |sink| self.render.render_into(sink));
  }
}

pub(crate) trait EventHost: Render {}

macro_rules! event_hosts {
  ($($host:ty),+ $(,)?) => {
    $(impl EventHost for $host {})+
  };
}

event_hosts!(
  VisualElement,
  UiBox,
  Label,
  TextElement,
  TextField,
  Toggle,
  RadioButton,
  RadioButtonGroup,
  ToggleButtonGroup,
  DropdownField,
  Button,
  RepeatButton,
  GroupBox,
  PopupWindow,
  ScrollView,
  Scroller,
  Slider,
  SliderInt,
  MinMaxSlider,
  ProgressBar,
  Tab,
  TabView,
  Image,
);

impl<H: crate::primitive::private::Host, C: Render> EventHost for Children<H, C> {}

struct EventInner<E> {
  payload: E,
  target: ElementTarget,
  propagation_stopped: Cell<bool>,
}

#[derive(Clone)]
pub(crate) struct Handler {
  model: TypeId,
  phase: HandlerPhase,
  callback: Rc<ErasedHandler>,
}

impl Handler {
  pub(crate) fn invoke(&self, game: &mut dyn Any, target: ElementTarget, body: UiEventBody) {
    (self.callback)(game, target, body);
  }

  pub(crate) fn model(&self) -> TypeId {
    self.model
  }

  pub(crate) fn phase(&self) -> HandlerPhase {
    self.phase
  }

  fn click<G: 'static>(callback: impl Fn(&mut G) + 'static) -> Self {
    Self {
      model: TypeId::of::<G>(),
      phase: HandlerPhase::Default,
      callback: Rc::new(move |game, _target, body| {
        let UiEventBody::Click(_) = body else {
          panic!("Reactant click handler received another event kind");
        };
        callback(
          game
            .downcast_mut::<G>()
            .expect("Reactant handler model type was not validated"),
        );
      }),
    }
  }

  fn click_event<G: 'static>(
    callback: impl Fn(&mut G, ReactantEvent<ClickEvent>) + 'static,
  ) -> Self {
    Self {
      model: TypeId::of::<G>(),
      phase: HandlerPhase::Default,
      callback: Rc::new(move |game, target, body| {
        let UiEventBody::Click(payload) = body else {
          panic!("Reactant click handler received another event kind");
        };
        callback(
          game
            .downcast_mut::<G>()
            .expect("Reactant handler model type was not validated"),
          ReactantEvent {
            inner: Rc::new(EventInner {
              payload,
              target,
              propagation_stopped: Cell::new(false),
            }),
            current_target: target,
            phase: EventPhase::Target,
          },
        );
      }),
    }
  }

  pub(crate) const fn kind(&self) -> UiEventKind {
    UiEventKind::Click
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum HandlerPhase {
  Capture,
  Default,
}

type ErasedHandler = dyn Fn(&mut dyn Any, ElementTarget, UiEventBody);

const _: HandlerPhase = HandlerPhase::Capture;
