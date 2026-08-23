//! Canonical Rust types for the Battlement wire protocol.
//!
//! Battlement is a Unity rendering and input client for turn-based games. This
//! crate models the messages exchanged between that client and an authoritative
//! rules engine.
//!
//! The main entry points are [`Connect`], [`Response`], [`ResponseMessage`],
//! [`ClientMessage`], [`Snapshot`], and [`Batch`]. Rules engines normally build
//! commands with [`Command`] and [`CommandBody`]. Game-specific integrations
//! can use [`CustomAction`] and [`CustomCommand`] without giving up strongly
//! typed IDs or the shared command and action formats.
//! Required values are constructor arguments. Records with useful defaults
//! provide consuming field-named methods for fluent configuration.
//!
//! These types derive [`serde::Serialize`] and [`serde::Deserialize`] without
//! prescribing a particular serialization format.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod assets;
mod command_builders;
pub mod commands;
pub mod json;
mod message_builders;
pub mod messages;
mod object_builders;
pub mod objects;
pub mod validation;

pub use assets::*;
pub use battlement_types::*;
pub use battlement_ui::*;
pub use commands::*;
pub use messages::*;
pub use objects::*;
pub use validation::*;

pub(crate) fn default_one() -> f64 {
    1.0
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    value == &T::default()
}

pub(crate) fn is_one(value: &f64) -> bool {
    *value == 1.0
}

pub(crate) fn is_true(value: &bool) -> bool {
    *value
}
