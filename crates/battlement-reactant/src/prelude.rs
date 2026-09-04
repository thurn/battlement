//! Focused imports for ordinary Reactant component authoring.
//!
//! This module is the crate's sole exception to the repository rule against
//! public re-exports. It intentionally excludes runtime administration,
//! executors, protocol messages, and command composition.
//!
//! ```
//! use battlement_reactant::prelude::*;
//! use trox::{tx_args, txa};
//!
//! #[builder]
//! struct Greeting {
//!     #[builder(required)]
//!     name: String,
//! }
//!
//! impl Component for Greeting {
//!     fn render(&self) -> impl Render {
//!         View::new()
//!             .class("greeting")
//!             .child(Label::new(txa(
//!                 "Hello, {name}",
//!                 tx_args![name => self.name.clone()],
//!                 "Personalized greeting in the example.",
//!             )))
//!     }
//! }
//!
//! let _view = Fragment::new((Greeting::new().name("Ada"), ()));
//! ```
//!
//! Runtime administration remains an explicit import.
//!
//! ```compile_fail
//! use battlement_reactant::prelude::*;
//!
//! fn administer(_runtime: Reactant<()>) {}
//! ```

#[doc(hidden)]
pub use crate::builder_support as __builder_support;
pub use battlement::application::ReducedMotionPreference;
pub use battlement_builder::builder;

pub use battlement::{
  AccessibilityScrollAxis, AccessibilityScrollDirection, Align, CheckedState, Color, Display,
  FilterFunction, FilterList, FlexDirection, Gradient, GradientStop, Justify, Length, LengthUnits,
  MotionProperty, Overflow, PaintFill, PaintLayer, PaintStyle, PickingMode, PlacementAlign,
  PlacementSide, PopoverPlacement, PopupKind, Position, Prop, SemanticRole, SemanticState, Shadow,
  StepPosition, Style, TextAnchor, TransformList, TransformOperation, Visibility, WhiteSpace,
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
  announcement::{Announce, use_announce},
  app_context::{AppHandle, use_app, use_viewport_size},
  application::use_application_state,
  callback::Callback as EventCallback,
  component::{Component, Memo, RenderCallback, memo},
  components::{
    Button, Checkbox, ColumnHeader, Disclosure, Group, Heading, Image, Link, ListBox,
    ListBoxOption, Navigation, PopupButton, Progress, Radio, RadioGroup, Region, RowHeader,
    ScrollArea, Slider, Switch, Tab, TabPanel, Table, TableCell, TableRow, Tabs, Text,
  },
  context::{Context, ContextProvider, Provided, RequiredContext, RequiredContextProvider},
  element_ref::{ElementRef, use_element_ref},
  error_boundary::{ErrorBoundary, NoErrorHandler, NoReset},
  event::{ElementTarget, EventPhase, ReactantEvent},
  external_store::{ExternalStore, StoreNotify, Subscription},
  focus::FocusProps,
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
    use_context, use_effect, use_effect_always, use_external_store, use_id, use_is_present,
    use_memo, use_presence, use_reducer, use_reducer_with, use_ref, use_ref_with,
    use_required_context, use_state, use_state_with,
  },
  host::{
    Box, DropdownField, Flex, Grid, GroupBox, Label, LocalizedChoice, MinMaxSlider, PopupWindow,
    ProgressBar, RadioButton, RadioButtonGroup, RepeatButton, ScrollView, Scroller, SliderInt,
    Stack, TabView, TextElement, TextField, ToggleButtonGroup, View,
  },
  key::{KeyRenderExt, Keyed},
  layout::{Layout, LayoutGroup, ReorderAxis, reorder_index},
  motion::{
    Easing, InertiaTarget, InitialTarget, Keyframes, MotionProps, MotionTarget, Repeat, RepeatType,
    StyleTarget, Transition,
  },
  motion_component::{MotionComponent, MotionComponentExt},
  motion_config::{MotionConfig, ReducedMotion, use_reduced_motion, use_reduced_motion_preference},
  motion_filter::{MotionFilter, MotionFilterList, PaintDropShadow, PaintFilterList},
  motion_value::{
    AnimationPlayback, AudioPlayback, AudioPlaybackOptions, ControlledMotionClock, InputRange,
    MotionExpression, MotionTimeSource, MotionValue as TypedMotionValue, MotionValueEvent,
    MotionValueType, OutputRange, PlaybackOutcome, SpringOptions, SpringValue,
    use_controlled_motion_clock, use_motion_expression, use_motion_time, use_motion_value,
    use_motion_value_event, use_spring, use_time, use_transform, use_velocity,
  },
  motion_variants::VariantOrchestration,
  overlay::{Overlay, OverlayHost},
  portal::{Portal, PortalTarget, create_portal},
  presence::{AnimatePresence, Presence, PresenceMode},
  props::Missing,
  render::{Child, Children, Either, Fragment, Node, Render},
  required_props,
  resource::{Resource, ResourceRead, ResourceStatus, use_resource},
  resource_control::{ResourceControl, use_resource_control},
  runtime::RenderError,
  semantics::{
    ActionDisposition, ControlBehavior, InteractionProps, SemanticDescription, SemanticName,
    SemanticProps, SemanticRange, SemanticVisibility,
  },
  suspense::Suspense,
  variant_map::{VariantData, VariantKey, VariantName, VariantTarget, Variants},
};
pub use battlement::{StaggerDirection, VariantWhen};
pub use trox::LocalizedString;

pub use crate::{
  element_behavior::{use_focus_on_mount, use_scroll_reveal},
  label_binding::{
    AssociatedControl, AssociatedLabel, ControlLabel, LabelBinding, use_control_label, use_label,
  },
  scale_to_fit::ScaleToFit,
};
