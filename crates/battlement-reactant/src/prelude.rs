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
  component::{Component, RenderCallback},
  event::{
    ChangeEventRenderExt, ElementTarget, EventPhase, EventRenderExt, ReactantEvent,
    ScrollEventRenderExt, TabEventRenderExt, TextEventRenderExt, ValueChangingRenderExt,
    ValueCommittedRenderExt,
  },
  hooks::{ReducerDispatch, StateSetter, use_reducer, use_reducer_with, use_state, use_state_with},
  key::{KeyRenderExt, Keyed},
  primitive::{Children, ContainerRenderExt},
  props::Missing,
  render::{Either, Fragment, Node, Render},
  required_props,
};
