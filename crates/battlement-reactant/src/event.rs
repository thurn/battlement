//! Typed application event handlers and event views.

#![allow(private_bounds, private_interfaces)]

use std::{any::TypeId, cell::Cell, rc::Rc};

use battlement::{
  Box as UiBox, Button, ClickEvent, DropdownField, FocusEvent, GroupBox, Image, KeyEvent, Label,
  LifecycleEvent, LinkEvent, MinMaxSlider, NavigationEvent, NavigationMoveEvent, ObjectId,
  PointerButtonEvent, PointerCancelEvent, PointerCaptureEvent, PointerCrossingEvent,
  PointerMoveEvent, PopupWindow, ProgressBar, RadioButton, RadioButtonGroup, RepeatButton,
  ScrollEvent, ScrollView, Scroller, SelectionEvent, Slider, SliderInt, Tab, TabCloseEvent,
  TabReorderEvent, TabSelectionEvent, TabView, TextElement, TextField, TextInputEvent, Toggle,
  ToggleButtonGroup, TransitionEvent, UiEventBody, UiEventKind, ValueChangingEvent,
  ValueCommitEvent, VisualElement, WheelEvent,
};

use crate::{
  event_control::{
    ChangeHost, ScrollEventHost, TabEventHost, TextEventHost, ValueChangingHost, ValueCommittedHost,
  },
  event_handler::{Handler, HandlerPhase},
  primitive::Children,
  render::{Render, RenderSink, private::Sealed},
  runtime::Root,
};

macro_rules! event_methods {
  ($(($brief:ident, $aware:ident, $slot:literal, $kind:ident, $variant:ident, $payload:ty)),+ $(,)?) => {
    $(
      #[doc = concat!("Replaces the `", stringify!($brief), "` handler.")]
      fn $brief<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> EventHandler<Self> {
        EventHandler::new(
          self,
          Handler::brief(
            $slot,
            UiEventKind::$kind,
            HandlerPhase::Default,
            |body| match body {
              UiEventBody::$variant(value) => value,
              _ => panic!(concat!("Reactant ", stringify!($kind), " handler received another event kind")),
            },
            callback,
          ),
        )
      }

      #[doc = concat!("Replaces the typed `", stringify!($brief), "` handler.")]
      fn $aware<G: 'static>(
        self,
        callback: impl Fn(&mut G, ReactantEvent<$payload>) + 'static,
      ) -> EventHandler<Self> {
        EventHandler::new(
          self,
          Handler::event(
            $slot,
            UiEventKind::$kind,
            HandlerPhase::Default,
            |body| match body {
              UiEventBody::$variant(value) => value,
              _ => panic!(concat!("Reactant ", stringify!($kind), " handler received another event kind")),
            },
            callback,
          ),
        )
      }
    )+
  };
}

macro_rules! propagating_event_methods {
  ($(($brief:ident, $aware:ident, $capture:ident, $capture_aware:ident, $slot:literal, $kind:ident, $variant:ident, $payload:ty)),+ $(,)?) => {
    event_methods!($(($brief, $aware, $slot, $kind, $variant, $payload)),+);
    $(
      #[doc = concat!("Replaces the capture-phase `", stringify!($brief), "` handler.")]
      fn $capture<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> EventHandler<Self> {
        EventHandler::new(
          self,
          Handler::brief(
            $slot,
            UiEventKind::$kind,
            HandlerPhase::Capture,
            |body| match body {
              UiEventBody::$variant(value) => value,
              _ => panic!(concat!("Reactant ", stringify!($kind), " handler received another event kind")),
            },
            callback,
          ),
        )
      }

      #[doc = concat!("Replaces the typed capture-phase `", stringify!($brief), "` handler.")]
      fn $capture_aware<G: 'static>(
        self,
        callback: impl Fn(&mut G, ReactantEvent<$payload>) + 'static,
      ) -> EventHandler<Self> {
        EventHandler::new(
          self,
          Handler::event(
            $slot,
            UiEventKind::$kind,
            HandlerPhase::Capture,
            |body| match body {
              UiEventBody::$variant(value) => value,
              _ => panic!(concat!("Reactant ", stringify!($kind), " handler received another event kind")),
            },
            callback,
          ),
        )
      }
    )+
  };
}

/// Adds common typed event handlers to a host render value.
///
/// Target-only events deliberately have no capture builder.
///
/// ```compile_fail
/// use battlement::VisualElement;
/// use battlement_reactant::event::EventRenderExt;
///
/// let _ = VisualElement::new().on_pointer_enter_capture(|_: &mut ()| {});
/// ```
pub trait EventRenderExt: EventHost + Sized {
  propagating_event_methods!(
    (
      on_pointer_down,
      on_pointer_down_event,
      on_pointer_down_capture,
      on_pointer_down_capture_event,
      "pointer_down",
      PointerDown,
      PointerDown,
      PointerButtonEvent
    ),
    (
      on_pointer_move,
      on_pointer_move_event,
      on_pointer_move_capture,
      on_pointer_move_capture_event,
      "pointer_move",
      PointerMove,
      PointerMove,
      PointerMoveEvent
    ),
    (
      on_pointer_up,
      on_pointer_up_event,
      on_pointer_up_capture,
      on_pointer_up_capture_event,
      "pointer_up",
      PointerUp,
      PointerUp,
      PointerButtonEvent
    ),
    (
      on_pointer_cancel,
      on_pointer_cancel_event,
      on_pointer_cancel_capture,
      on_pointer_cancel_capture_event,
      "pointer_cancel",
      PointerCancel,
      PointerCancel,
      PointerCancelEvent
    ),
    (
      on_click,
      on_click_event,
      on_click_capture,
      on_click_capture_event,
      "click",
      Click,
      Click,
      ClickEvent
    ),
    (
      on_pointer_over,
      on_pointer_over_event,
      on_pointer_over_capture,
      on_pointer_over_capture_event,
      "pointer_over",
      PointerOver,
      PointerOver,
      PointerCrossingEvent
    ),
    (
      on_pointer_out,
      on_pointer_out_event,
      on_pointer_out_capture,
      on_pointer_out_capture_event,
      "pointer_out",
      PointerOut,
      PointerOut,
      PointerCrossingEvent
    ),
    (
      on_wheel,
      on_wheel_event,
      on_wheel_capture,
      on_wheel_capture_event,
      "wheel",
      Wheel,
      Wheel,
      WheelEvent
    ),
    (
      on_pointer_capture,
      on_pointer_capture_event,
      on_pointer_capture_capture,
      on_pointer_capture_capture_event,
      "pointer_capture",
      PointerCapture,
      PointerCapture,
      PointerCaptureEvent
    ),
    (
      on_pointer_capture_out,
      on_pointer_capture_out_event,
      on_pointer_capture_out_capture,
      on_pointer_capture_out_capture_event,
      "pointer_capture_out",
      PointerCaptureOut,
      PointerCaptureOut,
      PointerCaptureEvent
    ),
    (
      on_key_down,
      on_key_down_event,
      on_key_down_capture,
      on_key_down_capture_event,
      "key_down",
      KeyDown,
      KeyDown,
      KeyEvent
    ),
    (
      on_key_up,
      on_key_up_event,
      on_key_up_capture,
      on_key_up_capture_event,
      "key_up",
      KeyUp,
      KeyUp,
      KeyEvent
    ),
    (
      on_navigation_move,
      on_navigation_move_event,
      on_navigation_move_capture,
      on_navigation_move_capture_event,
      "navigation_move",
      NavigationMove,
      NavigationMove,
      NavigationMoveEvent
    ),
    (
      on_navigation_cancel,
      on_navigation_cancel_event,
      on_navigation_cancel_capture,
      on_navigation_cancel_capture_event,
      "navigation_cancel",
      NavigationCancel,
      NavigationCancel,
      NavigationEvent
    ),
    (
      on_focus_in,
      on_focus_in_event,
      on_focus_in_capture,
      on_focus_in_capture_event,
      "focus_in",
      FocusIn,
      FocusIn,
      FocusEvent
    ),
    (
      on_focus_out,
      on_focus_out_event,
      on_focus_out_capture,
      on_focus_out_capture_event,
      "focus_out",
      FocusOut,
      FocusOut,
      FocusEvent
    ),
    (
      on_focus,
      on_focus_event,
      on_focus_capture,
      on_focus_capture_event,
      "focus",
      FocusIn,
      FocusIn,
      FocusEvent
    ),
    (
      on_blur,
      on_blur_event,
      on_blur_capture,
      on_blur_capture_event,
      "blur",
      FocusOut,
      FocusOut,
      FocusEvent
    ),
    (
      on_link_enter,
      on_link_enter_event,
      on_link_enter_capture,
      on_link_enter_capture_event,
      "link_enter",
      LinkEnter,
      LinkEnter,
      LinkEvent
    ),
    (
      on_link_leave,
      on_link_leave_event,
      on_link_leave_capture,
      on_link_leave_capture_event,
      "link_leave",
      LinkLeave,
      LinkLeave,
      LinkEvent
    ),
    (
      on_link_down,
      on_link_down_event,
      on_link_down_capture,
      on_link_down_capture_event,
      "link_down",
      LinkDown,
      LinkDown,
      LinkEvent
    ),
    (
      on_link_up,
      on_link_up_event,
      on_link_up_capture,
      on_link_up_capture_event,
      "link_up",
      LinkUp,
      LinkUp,
      LinkEvent
    ),
  );
  event_methods!(
    (
      on_pointer_enter,
      on_pointer_enter_event,
      "pointer_enter",
      PointerOver,
      PointerOver,
      PointerCrossingEvent
    ),
    (
      on_pointer_leave,
      on_pointer_leave_event,
      "pointer_leave",
      PointerOut,
      PointerOut,
      PointerCrossingEvent
    ),
    (
      on_attach_to_panel,
      on_attach_to_panel_event,
      "attach_to_panel",
      AttachToPanel,
      AttachToPanel,
      LifecycleEvent
    ),
    (
      on_detach_from_panel,
      on_detach_from_panel_event,
      "detach_from_panel",
      DetachFromPanel,
      DetachFromPanel,
      LifecycleEvent
    ),
    (
      on_transition_start,
      on_transition_start_event,
      "transition_start",
      TransitionStart,
      TransitionStart,
      TransitionEvent
    ),
    (
      on_transition_end,
      on_transition_end_event,
      "transition_end",
      TransitionEnd,
      TransitionEnd,
      TransitionEvent
    ),
    (
      on_transition_cancel,
      on_transition_cancel_event,
      "transition_cancel",
      TransitionCancel,
      TransitionCancel,
      TransitionEvent
    ),
  );
}

/// Adds text-input-specific event handlers.
pub trait TextEventRenderExt: TextEventHost + Sized {
  event_methods!(
    (
      on_input,
      on_input_event,
      "input",
      Input,
      Input,
      TextInputEvent
    ),
    (
      on_selection_changed,
      on_selection_changed_event,
      "selection_changed",
      SelectionChanged,
      SelectionChanged,
      SelectionEvent
    ),
  );
}

/// Adds scroll-view-specific event handlers.
pub trait ScrollEventRenderExt: ScrollEventHost + Sized {
  event_methods!(
    (
      on_scroll_settled,
      on_scroll_settled_event,
      "scroll_settled",
      ScrollSettled,
      ScrollSettled,
      ScrollEvent
    ),
    (
      on_scroll_changed,
      on_scroll_changed_event,
      "scroll_changed",
      ScrollChanged,
      ScrollChanged,
      ScrollEvent
    ),
  );
}

/// Adds tab-view-specific proposal handlers.
pub trait TabEventRenderExt: TabEventHost + Sized {
  event_methods!(
    (
      on_tab_selection_requested,
      on_tab_selection_requested_event,
      "tab_selection_requested",
      TabSelectionRequested,
      TabSelectionRequested,
      TabSelectionEvent
    ),
    (
      on_tab_close_requested,
      on_tab_close_requested_event,
      "tab_close_requested",
      TabCloseRequested,
      TabCloseRequested,
      TabCloseEvent
    ),
    (
      on_tab_reorder_requested,
      on_tab_reorder_requested_event,
      "tab_reorder_requested",
      TabReorderRequested,
      TabReorderRequested,
      TabReorderEvent
    ),
  );
}

/// Adds native live-value handlers to continuous controls.
pub trait ValueChangingRenderExt: ValueChangingHost + Sized {
  event_methods!((
    on_value_changing,
    on_value_changing_event,
    "value_changing",
    ValueChanging,
    ValueChanging,
    ValueChangingEvent
  ));
}

/// Adds native completed-value handlers to controlled inputs.
pub trait ValueCommittedRenderExt: ValueCommittedHost + Sized {
  event_methods!((
    on_value_committed,
    on_value_committed_event,
    "value_committed",
    ValueCommitted,
    ValueCommitted,
    ValueCommitEvent
  ));
}

/// Adds a control-specific typed change handler.
///
/// Hosts without controlled values deliberately have no change builder.
///
/// ```compile_fail
/// use battlement::Button;
/// use battlement_reactant::event::ChangeEventRenderExt;
///
/// let _ = Button::new().on_change(|_: &mut ()| {});
/// ```
pub trait ChangeEventRenderExt: ChangeHost + Sized {
  /// Replaces the payload-free change handler.
  fn on_change<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> EventHandler<Self> {
    EventHandler::new(
      self,
      Handler::brief_owned(
        "change",
        Self::change_kind(),
        HandlerPhase::Default,
        Self::change_payload,
        callback,
      ),
    )
  }

  /// Replaces the typed change handler.
  fn on_change_event<G: 'static>(
    self,
    callback: impl Fn(&mut G, ReactantEvent<<Self as ChangeHost>::Value>) + 'static,
  ) -> EventHandler<Self> {
    EventHandler::new(
      self,
      Handler::event_owned(
        "change",
        Self::change_kind(),
        HandlerPhase::Default,
        Self::change_payload,
        callback,
      ),
    )
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
  inner: Rc<EventInner>,
  payload: EventPayload<E>,
  current_target: ElementTarget,
  phase: EventPhase,
}

/// A host render value carrying an event handler slot.
#[derive(Clone)]
pub struct EventHandler<R> {
  render: R,
  handler: Handler,
}

impl<R: ChangeHost> ChangeEventRenderExt for R {}

impl<R: TextEventHost> TextEventRenderExt for R {}
impl<R: ScrollEventHost> ScrollEventRenderExt for R {}
impl<R: TabEventHost> TabEventRenderExt for R {}
impl<R: ValueChangingHost> ValueChangingRenderExt for R {}
impl<R: ValueCommittedHost> ValueCommittedRenderExt for R {}

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
    match &self.payload {
      EventPayload::Shared { body, extract } => extract(body),
      EventPayload::Owned(payload) => payload,
    }
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

  pub(crate) fn new(
    inner: Rc<EventInner>,
    body: Rc<UiEventBody>,
    extract: fn(&UiEventBody) -> &E,
    current_target: ElementTarget,
    phase: EventPhase,
  ) -> Self {
    Self {
      inner,
      payload: EventPayload::Shared { body, extract },
      current_target,
      phase,
    }
  }

  pub(crate) fn new_owned(
    inner: Rc<EventInner>,
    payload: E,
    current_target: ElementTarget,
    phase: EventPhase,
  ) -> Self {
    Self {
      inner,
      payload: EventPayload::Owned(Rc::new(payload)),
      current_target,
      phase,
    }
  }
}

impl<E> Clone for ReactantEvent<E> {
  fn clone(&self) -> Self {
    Self {
      inner: Rc::clone(&self.inner),
      payload: self.payload.clone(),
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

pub(crate) struct EventInner {
  target: ElementTarget,
  propagation_stopped: Rc<Cell<bool>>,
}

impl EventInner {
  pub(crate) fn new(target: ElementTarget, propagation_stopped: Rc<Cell<bool>>) -> Self {
    Self {
      target,
      propagation_stopped,
    }
  }

  pub(crate) fn propagation_stopped(&self) -> bool {
    self.propagation_stopped.get()
  }
}

enum EventPayload<E> {
  Shared {
    body: Rc<UiEventBody>,
    extract: fn(&UiEventBody) -> &E,
  },
  Owned(Rc<E>),
}

impl<E> Clone for EventPayload<E> {
  fn clone(&self) -> Self {
    match self {
      Self::Shared { body, extract } => Self::Shared {
        body: Rc::clone(body),
        extract: *extract,
      },
      Self::Owned(payload) => Self::Owned(Rc::clone(payload)),
    }
  }
}
