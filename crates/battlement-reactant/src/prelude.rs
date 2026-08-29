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
//!         VisualElement::new()
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
  Box, Button, GroupBox, Image, Label, PopupWindow, Prop, ScrollView, Style, Tab, TabView,
  TextElement, TextField, Toggle, VisualElement,
};

pub use crate::{
  component::{Component, Memo, RenderCallback, memo},
  context::{Context, ContextProvider, Provided, RequiredContext, RequiredContextProvider},
  element_ref::{ElementRef, Referenced, use_element_ref},
  error_boundary::{ErrorBoundary, NoErrorHandler, NoReset},
  event::{
    ChangeEventRenderExt, ElementTarget, EventPhase, EventRenderExt, ReactantEvent,
    ScrollEventRenderExt, TabEventRenderExt, TextEventRenderExt, ValueChangingRenderExt,
    ValueCommittedRenderExt,
  },
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
  key::{KeyRenderExt, Keyed},
  portal::{HostRender, Portal, PortalContainer, PortalTarget, ReactantHostExt, create_portal},
  primitive::{Children, ContainerRenderExt},
  props::Missing,
  render::{Either, Fragment, Node, Render},
  required_props,
  resource::{Resource, ResourceRead, ResourceStatus, use_resource},
  runtime::RenderError,
  suspense::Suspense,
};
