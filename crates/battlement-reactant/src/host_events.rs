//! Event builders for Reactant host façades.

use battlement::{
  Choice, ClickEvent, F32Range, FocusEvent, GeometryEvent, KeyEvent, LifecycleEvent, LinkEvent,
  NavigationEvent, NavigationMoveEvent, PointerButtonEvent, PointerCancelEvent,
  PointerCaptureEvent, PointerCrossingEvent, PointerMoveEvent, ScrollEvent, SelectionEvent,
  TabCloseEvent, TabReorderEvent, TabSelectionEvent, TextInputEvent, TransitionEvent, UiEventBody,
  UiEventKind, UiValue, ValueChangingEvent, ValueCommitEvent, WheelEvent,
};

use crate::{
  event::ReactantEvent,
  event_handler::{Handler, HandlerPhase},
  host::{
    Box, Button, DropdownField, Flex, GroupBox, Image, Label, MinMaxSlider, PopupWindow,
    ProgressBar, RadioButton, RadioButtonGroup, RepeatButton, ScrollView, Scroller, Slider,
    SliderInt, Tab, TabView, TextElement, TextField, Toggle, ToggleButtonGroup, View,
  },
};

macro_rules! event_methods {
  ($(($brief:ident, $aware:ident, $slot:literal, $kind:ident, $variant:ident, $payload:ty)),+ $(,)?) => {
    $(
      #[doc = concat!("Replaces the `", stringify!($brief), "` handler.")]
      pub fn $brief<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
        self.with_handler(Handler::brief(
          $slot, UiEventKind::$kind, HandlerPhase::Default,
          |body| match body {
            UiEventBody::$variant(value) => value,
            _ => panic!(concat!("Reactant ", stringify!($kind), " handler received another event kind")),
          },
          callback,
        ))
      }

      #[doc = concat!("Replaces the typed `", stringify!($brief), "` handler.")]
      pub fn $aware<G: 'static>(
        self,
        callback: impl Fn(&mut G, ReactantEvent<$payload>) + 'static,
      ) -> Self {
        self.with_handler(Handler::event(
          $slot, UiEventKind::$kind, HandlerPhase::Default,
          |body| match body {
            UiEventBody::$variant(value) => value,
            _ => panic!(concat!("Reactant ", stringify!($kind), " handler received another event kind")),
          },
          callback,
        ))
      }
    )+
  };
}

macro_rules! propagating_event_methods {
  ($(($brief:ident, $aware:ident, $capture:ident, $capture_aware:ident, $slot:literal, $kind:ident, $variant:ident, $payload:ty)),+ $(,)?) => {
    event_methods!($(($brief, $aware, $slot, $kind, $variant, $payload)),+);
    $(
      #[doc = concat!("Replaces the capture-phase `", stringify!($brief), "` handler.")]
      pub fn $capture<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
        self.with_handler(Handler::brief(
          $slot, UiEventKind::$kind, HandlerPhase::Capture,
          |body| match body {
            UiEventBody::$variant(value) => value,
            _ => panic!(concat!("Reactant ", stringify!($kind), " handler received another event kind")),
          },
          callback,
        ))
      }

      #[doc = concat!("Replaces the typed capture-phase `", stringify!($brief), "` handler.")]
      pub fn $capture_aware<G: 'static>(
        self,
        callback: impl Fn(&mut G, ReactantEvent<$payload>) + 'static,
      ) -> Self {
        self.with_handler(Handler::event(
          $slot, UiEventKind::$kind, HandlerPhase::Capture,
          |body| match body {
            UiEventBody::$variant(value) => value,
            _ => panic!(concat!("Reactant ", stringify!($kind), " handler received another event kind")),
          },
          callback,
        ))
      }
    )+
  };
}

macro_rules! common_event_methods {
  () => {
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
        on_geometry_changed,
        on_geometry_changed_event,
        "geometry_changed",
        GeometryChanged,
        GeometryChanged,
        GeometryEvent
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
  };
}

macro_rules! text_event_methods {
  () => {
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
  };
}

macro_rules! scroll_event_methods {
  () => {
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
  };
}

macro_rules! tab_event_methods {
  () => {
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
  };
}

macro_rules! value_changing_methods {
  () => {
    event_methods!((
      on_value_changing,
      on_value_changing_event,
      "value_changing",
      ValueChanging,
      ValueChanging,
      ValueChangingEvent
    ));
  };
}

macro_rules! value_committed_methods {
  () => {
    event_methods!((
      on_value_committed,
      on_value_committed_event,
      "value_committed",
      ValueCommitted,
      ValueCommitted,
      ValueCommitEvent
    ));
  };
}

macro_rules! implement_event_methods {
  ($methods:ident: $($host:ty),+ $(,)?) => {
    $(
      impl $host {
        $methods!();
      }
    )+
  };
}

implement_event_methods!(common_event_methods:
  View,
  Flex,
  Box,
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

implement_event_methods!(text_event_methods: TextField);
implement_event_methods!(scroll_event_methods: ScrollView);
implement_event_methods!(tab_event_methods: TabView);
implement_event_methods!(value_changing_methods: Scroller, Slider, SliderInt, MinMaxSlider);
implement_event_methods!(value_committed_methods:
  TextField,
  Toggle,
  RadioButton,
  RadioButtonGroup,
  ToggleButtonGroup,
  DropdownField,
  Scroller,
  Slider,
  SliderInt,
  MinMaxSlider,
);

macro_rules! change_host {
  ($host:ty, $value:ty, $slot:literal, $kind:ident, $body:ident => $payload:expr) => {
    impl $host {
      /// Replaces the payload-free change handler.
      #[must_use]
      pub fn on_change<G: 'static>(self, callback: impl Fn(&mut G) + 'static) -> Self {
        self.with_handler(Handler::brief_owned(
          $slot,
          UiEventKind::$kind,
          HandlerPhase::Default,
          |$body| $payload,
          callback,
        ))
      }

      /// Replaces the typed change handler.
      #[must_use]
      pub fn on_change_event<G: 'static>(
        self,
        callback: impl Fn(&mut G, ReactantEvent<$value>) + 'static,
      ) -> Self {
        self.with_handler(Handler::event_owned(
          $slot,
          UiEventKind::$kind,
          HandlerPhase::Default,
          |$body| $payload,
          callback,
        ))
      }
    }
  };
}

change_host!(TextField, String, "input", Input, body => match body { UiEventBody::Input(value) => value.value, _ => panic!("Reactant Input change handler received another event kind") });
change_host!(Scroller, f32, "value_changing", ValueChanging, body => changing_f32(body));
change_host!(Slider, f32, "value_changing", ValueChanging, body => changing_f32(body));
change_host!(SliderInt, i32, "value_changing", ValueChanging, body => match body { UiEventBody::ValueChanging(value) => match value.proposed { UiValue::I32(value) => value, _ => panic!("Reactant SliderInt change handler received another value type") }, _ => panic!("Reactant SliderInt change handler received another event kind") });
change_host!(MinMaxSlider, F32Range, "value_changing", ValueChanging, body => match body { UiEventBody::ValueChanging(value) => match value.proposed { UiValue::F32Range(value) => value, _ => panic!("Reactant MinMaxSlider change handler received another value type") }, _ => panic!("Reactant MinMaxSlider change handler received another event kind") });
change_host!(Toggle, bool, "value_committed", ValueCommitted, body => committed_bool(body));
change_host!(RadioButton, bool, "value_committed", ValueCommitted, body => committed_bool(body));
change_host!(RadioButtonGroup, Option<u32>, "value_committed", ValueCommitted, body => match body { UiEventBody::ValueCommitted(value) => match value.proposed { UiValue::Index(value) => value, _ => panic!("Reactant RadioButtonGroup change handler received another value type") }, _ => panic!("Reactant RadioButtonGroup change handler received another event kind") });
change_host!(ToggleButtonGroup, Vec<u32>, "value_committed", ValueCommitted, body => match body { UiEventBody::ValueCommitted(value) => match value.proposed { UiValue::Indices(value) => value, _ => panic!("Reactant ToggleButtonGroup change handler received another value type") }, _ => panic!("Reactant ToggleButtonGroup change handler received another event kind") });
change_host!(DropdownField, Choice, "value_committed", ValueCommitted, body => match body { UiEventBody::ValueCommitted(value) => match value.proposed { UiValue::Choice(value) => value, _ => panic!("Reactant DropdownField change handler received another value type") }, _ => panic!("Reactant DropdownField change handler received another event kind") });
change_host!(TabView, u32, "tab_selection_requested", TabSelectionRequested, body => match body { UiEventBody::TabSelectionRequested(value) => value.proposed_index, _ => panic!("Reactant TabView change handler received another event kind") });

fn changing_f32(body: UiEventBody) -> f32 {
  match body {
    UiEventBody::ValueChanging(value) => match value.proposed {
      UiValue::F32(value) => value,
      _ => panic!("Reactant floating-point change handler received another value type"),
    },
    _ => panic!("Reactant floating-point change handler received another event kind"),
  }
}

fn committed_bool(body: UiEventBody) -> bool {
  match body {
    UiEventBody::ValueCommitted(value) => match value.proposed {
      UiValue::Bool(value) => value,
      _ => panic!("Reactant Boolean change handler received another value type"),
    },
    _ => panic!("Reactant Boolean change handler received another event kind"),
  }
}
