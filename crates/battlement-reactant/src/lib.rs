//! Declarative component rendering for Battlement UI documents.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod component;
mod context;
pub mod event;
mod event_control;
mod event_dispatch;
mod event_handler;
pub mod executor;
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
