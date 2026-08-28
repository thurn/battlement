//! Declarative component rendering for Battlement UI documents.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod component;
pub mod context;
mod effect;
pub mod error_boundary;
pub mod event;
mod event_control;
mod event_dispatch;
mod event_handler;
pub mod executor;
mod external_portal;
pub mod external_store;
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
mod render_value;
pub mod runtime;
