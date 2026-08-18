//! Canonical Rust types for the Masonry wire protocol.
//!
//! Masonry is a Unity rendering and input client for turn-based games. This
//! crate models the messages exchanged between that client and an authoritative
//! rules engine.
//!
//! The main entry points are [`Connect`], [`Response`], [`ResponseMessage`],
//! [`ClientMessage`], [`Snapshot`], and [`Batch`]. Rules engines normally build
//! commands with [`Command`] and [`CommandBody`]. Game-specific integrations
//! can use [`CustomAction`] and [`CustomCommand`] without giving up strongly
//! typed IDs or the shared command and action formats.
//!
//! These types derive [`serde::Serialize`] and [`serde::Deserialize`] without
//! prescribing a particular serialization format.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod assets;
pub mod commands;
pub mod ids;
pub mod messagepack;
pub mod messages;
pub mod objects;
pub mod validation;
pub mod values;

pub use assets::*;
pub use commands::*;
pub use ids::*;
pub use messages::*;
pub use objects::*;
pub use validation::*;
pub use values::*;
