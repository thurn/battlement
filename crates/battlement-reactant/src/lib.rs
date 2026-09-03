//! Declarative component rendering for Battlement UI documents.
//!
//! Reactant renders Rust component structs into Battlement UI documents,
//! reconciles them with the last committed tree, and emits the host mutation
//! groups required to update Unity. Ordinary component code starts
//! with the focused [`prelude`] and [`app::App`] owns engine integration.
//! Specialized hosts can integrate [`runtime::Reactant`] directly.
//!
//! ```
//! use battlement_reactant::{app::App, prelude::*};
//!
//! struct Greeting;
//!
//! impl Component for Greeting {
//!     fn render(&self) -> impl Render {
//!         View::new().child(Label::new("Hello from Reactant"))
//!     }
//! }
//!
//! fn create_engine() -> App {
//!     App::new("my-game/content").ui(Greeting)
//! }
//!
//! battlement_native::export_engine!(create_engine);
//! ```
//!
//! Reactant uses React-compatible names only where Battlement can preserve the
//! corresponding behavior. The
//! [feature ledger](https://github.com/dthurn/battlement/blob/master/docs/reactant/feature-ledger.md)
//! maps the supported V1 surface to its sample screen and black-box proof and
//! lists the reserved React APIs that remain unsupported.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod accessibility;
pub mod accessibility_collections;
pub mod accessibility_popup;
mod action_context;
mod activation;
pub mod animation_controls;
pub mod announcement;
pub mod app;
pub mod app_context;
mod app_delivery;
mod app_engine;
mod app_root;
pub mod application;
pub mod asset_generator;
pub mod callback;
mod commit;
pub mod component;
pub mod context;
pub mod cooperative_executor;
mod effect;
pub mod element_ref;
pub mod error_boundary;
pub mod event;
mod event_dispatch;
mod event_handler;
pub mod executor;
mod external_portal;
pub mod external_store;
pub mod focus;
pub mod geometry;
mod geometry_effect;
mod geometry_runtime;
pub mod gesture;
mod hook_storage;
pub mod hooks;
pub mod host;
mod host_events;
mod host_facade;
mod host_flex;
mod host_grid;
mod host_properties;
mod host_stack;
pub mod key;
pub mod layout;
mod lifecycle;
pub mod motion;
mod motion_component;
pub mod motion_config;
mod motion_css;
mod motion_lifecycle;
mod motion_transition;
pub mod motion_value;
mod motion_value_runtime;
mod motion_variants;
mod mutation;
pub mod overlay;
pub mod paint;
pub mod portal;
pub mod prelude;
pub mod presence;
mod presence_render;
pub mod props;
mod reconcile;
#[cfg(test)]
mod reconcile_tests;
pub mod render;
mod render_error;
mod render_facade;
mod render_tree;
mod render_value;
pub mod resource;
mod resource_admin;
mod resource_cache;
pub mod resource_control;
mod resource_runtime;
#[cfg(test)]
mod resource_tests;
mod root_view;
pub mod runtime;
mod runtime_document;
mod runtime_motion;
mod semantic_projection;
mod semantic_validation;
pub mod semantics;
pub mod suspense;
mod variant_map;

#[doc(hidden)]
#[macro_export]
macro_rules! __register_generated_asset {
  ($registration:expr) => {
    $crate::asset_generator::__private::submit! { $registration }
  };
}

pub mod element_behavior;

pub mod label_binding;

pub mod scale_to_fit;
