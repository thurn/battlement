//! Focused imports for ordinary Reactant component authoring.
//!
//! This module is the crate's sole exception to the repository rule against
//! public re-exports. It intentionally excludes runtime administration,
//! executors, protocol messages, and command composition.
//!
//! ```
//! use battlement_reactant::prelude::*;
//!
//! struct Greeting {
//!     name: String,
//! }
//!
//! impl Component for Greeting {
//!     fn render(&self) -> impl Render {
//!         View::new()
//!             .class("greeting")
//!             .child(Label::new(format!("Hello, {}", self.name)))
//!     }
//! }
//!
//! let _view = Fragment::new((Greeting { name: "Ada".to_owned() }, ()));
//! ```
//!
//! Runtime administration remains an explicit import.
//!
//! ```compile_fail
//! use battlement_reactant::prelude::*;
//!
//! fn administer(_runtime: Reactant<()>) {}
//! ```

pub use battlement::{
  MotionColor, MotionLength, MotionProperty, Prop, StepPosition, Style, Visibility,
};

pub use crate::{
  component::{Component, Memo, RenderCallback, memo},
  context::{Context, ContextProvider, Provided, RequiredContext, RequiredContextProvider},
  element_ref::{ElementRef, use_element_ref},
  error_boundary::{ErrorBoundary, NoErrorHandler, NoReset},
  event::{ElementTarget, EventPhase, ReactantEvent},
  external_store::{ExternalStore, StoreNotify, Subscription},
  geometry::{
    GeometrySnapshot, GeometryTargets, IntoGeometryEffectCleanup, Measurement, MeasurementStatus,
    ViewportRef, WorldGeometry, WorldRef, use_geometry, use_geometry_effect,
  },
  hooks::{
    Callback, Dependencies, IntoEffectCleanup, ReducerDispatch, Ref, StateSetter, use_callback,
    use_context, use_effect, use_effect_always, use_external_store, use_memo, use_reducer,
    use_reducer_with, use_ref, use_ref_with, use_required_context, use_state, use_state_with,
  },
  host::{
    Box, Button, DropdownField, GroupBox, Image, Label, MinMaxSlider, PopupWindow, ProgressBar,
    RadioButton, RadioButtonGroup, RepeatButton, ScrollView, Scroller, Slider, SliderInt, Tab,
    TabView, TextElement, TextField, Toggle, ToggleButtonGroup, View,
  },
  key::{KeyRenderExt, Keyed},
  motion::{
    Easing, InitialTarget, Keyframes, MotionComponent, MotionComponentExt, MotionProps,
    MotionStyle, MotionTarget, Repeat, RepeatType, Transition,
  },
  portal::{Portal, PortalTarget, create_portal},
  props::Missing,
  render::{Either, Fragment, Node, Render},
  required_props,
  resource::{Resource, ResourceRead, ResourceStatus, use_resource},
  runtime::RenderError,
  suspense::Suspense,
};
