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

pub use crate::motion_css::{
  Animation, AnimationComposition, AnimationDirection, AnimationFill, AnimationIterations,
  AnimationPlayState, Decoration, DecorationOverflow, DecorationPosition, IntoPseudoStyle,
  StyleProperty, StyleTransition,
};
pub use crate::{
  animation_controls::{
    AnimationControls, AnimationScope, AnimationSequence, ControlTarget, MotionSelector,
    SequencePosition, use_animation_controls, use_animation_scope,
  },
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
  gesture::{
    DragAxis, DragConstraints, DragControls, DragElastic, DragStartOptions, DragTransition,
    GestureConfig, use_drag_controls,
  },
  hooks::{
    Callback, Dependencies, IntoEffectCleanup, ReducerDispatch, Ref, StateSetter, use_callback,
    use_context, use_effect, use_effect_always, use_external_store, use_is_present, use_memo,
    use_presence, use_reducer, use_reducer_with, use_ref, use_ref_with, use_required_context,
    use_state, use_state_with,
  },
  host::{
    Box, Button, DropdownField, Flex, GroupBox, Image, Label, MinMaxSlider, PopupWindow,
    ProgressBar, RadioButton, RadioButtonGroup, RepeatButton, ScrollView, Scroller, Slider,
    SliderInt, Tab, TabView, TextElement, TextField, Toggle, ToggleButtonGroup, View,
  },
  key::{KeyRenderExt, Keyed},
  layout::{Layout, LayoutGroup, ReorderAxis, reorder_index},
  motion::{
    Easing, InertiaTarget, InitialTarget, Keyframes, MotionProps, MotionStyle, MotionTarget,
    Repeat, RepeatType, Transition,
  },
  motion_component::{MotionComponent, MotionComponentExt},
  motion_config::{MotionConfig, ReducedMotion, use_reduced_motion},
  motion_value::{
    AnimationPlayback, AudioPlayback, AudioPlaybackOptions, ControlledMotionClock, InputRange,
    MotionExpression, MotionTimeSource, MotionValue as TypedMotionValue, MotionValueEvent,
    MotionValueType, OutputRange, PlaybackOutcome, SpringOptions, SpringValue,
    use_controlled_motion_clock, use_motion_expression, use_motion_time, use_motion_value,
    use_motion_value_event, use_spring, use_time, use_transform, use_velocity,
  },
  motion_variants::VariantOrchestration,
  portal::{Portal, PortalTarget, create_portal},
  presence::{AnimatePresence, Presence, PresenceMode},
  props::Missing,
  render::{Either, Fragment, Node, Render},
  required_props,
  resource::{Resource, ResourceRead, ResourceStatus, use_resource},
  runtime::RenderError,
  suspense::Suspense,
  variant_map::{VariantData, VariantKey, VariantName, VariantTarget, Variants},
};
pub use battlement::{StaggerDirection, VariantWhen};
