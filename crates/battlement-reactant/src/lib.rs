//! Declarative component rendering for Battlement UI documents.
//!
//! Reactant renders Rust component structs into Battlement UI documents,
//! reconciles them with the last committed tree, and emits the host mutation
//! groups required to update Unity. Ordinary component code starts
//! with the focused [`prelude`] while engine integration uses [`runtime`].
//!
//! ```
//! use battlement_reactant::prelude::*;
//!
//! struct Greeting;
//!
//! impl Component for Greeting {
//!     fn render(&self) -> impl Render {
//!         View::new().child(Label::new("Hello from Reactant"))
//!     }
//! }
//!
//! let _view = Fragment::new((Greeting, ()));
//! ```
//!
//! Reactant uses React-compatible names only where Battlement can preserve the
//! corresponding behavior. The
//! [feature ledger](https://github.com/dthurn/battlement/blob/master/docs/reactant/feature-ledger.md)
//! maps the supported V1 surface to its sample screen and black-box proof and
//! lists the reserved React APIs that remain unsupported.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod animation_controls;
pub mod asset_generator;
mod commit;
pub mod component;
pub mod context;
mod effect;
pub mod element_ref;
pub mod error_boundary;
pub mod event;
mod event_dispatch;
mod event_handler;
pub mod executor;
mod external_portal;
pub mod external_store;
pub mod geometry;
mod geometry_effect;
mod geometry_runtime;
mod hook_storage;
pub mod hooks;
pub mod host;
mod host_events;
mod host_properties;
pub mod key;
mod lifecycle;
pub mod motion;
mod motion_component;
mod motion_css;
mod motion_lifecycle;
mod motion_transition;
pub mod motion_value;
mod motion_value_runtime;
mod motion_variants;
mod mutation;
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
mod render_tree;
mod render_value;
pub mod resource;
mod resource_admin;
mod resource_cache;
mod resource_runtime;
#[cfg(test)]
mod resource_tests;
mod root_view;
pub mod runtime;
mod runtime_document;
mod runtime_motion;
pub mod suspense;
mod variant_map;

#[doc(hidden)]
#[macro_export]
macro_rules! __register_generated_asset {
  ($registration:expr) => {
    $crate::asset_generator::__private::submit! { $registration }
  };
}
