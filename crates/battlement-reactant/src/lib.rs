//! Declarative component rendering for Battlement UI documents.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod commit;
pub mod component;
pub mod context;
mod effect;
pub mod element_ref;
pub mod error_boundary;
pub mod event;
mod event_control;
mod event_dispatch;
mod event_handler;
pub mod executor;
mod external_portal;
pub mod external_store;
pub mod geometry;
mod geometry_runtime;
mod hook_storage;
pub mod hooks;
pub mod key;
mod mutation;
pub mod portal;
pub mod prelude;
pub mod primitive;
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
pub mod runtime;
mod runtime_document;
pub mod suspense;
