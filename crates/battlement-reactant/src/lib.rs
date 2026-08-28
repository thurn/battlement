//! Declarative component rendering for Battlement UI documents.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod component;
pub mod context;
mod effect;
pub mod event;
mod event_control;
mod event_dispatch;
mod event_handler;
pub mod executor;
pub mod external_store;
mod hook_storage;
pub mod hooks;
pub mod key;
mod mutation;
pub mod prelude;
pub mod primitive;
pub mod props;
mod reconcile;
#[cfg(test)]
mod reconcile_tests;
pub mod render;
pub mod runtime;
