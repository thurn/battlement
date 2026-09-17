//! An in-memory Battlement client for fast rules-engine tests.
//!
//! The fake applies validated protocol snapshots and commands synchronously,
//! exposing the resulting world and an execution journal without Unity or
//! wall-clock time. Presentation time advances only through explicit controls.
//! Requests and responses cross the same verified FlatBuffer boundary as a
//! native client.

#![warn(missing_docs)]

pub mod assets;
pub mod client;
pub mod effects;
pub mod journal;
mod response_command;
mod response_motion_descriptor_reader;
mod response_motion_reader;
mod response_paint_reader;
mod response_reader;
mod response_ui_reader;
pub mod time;
pub mod world;

pub use battlement_ui_fake;
pub use response_reader::read as read_response;

mod assertions;
mod executor;
mod interpolation;
mod material;
mod operation;
mod pointer_geometry;
mod presentation;
mod transform;
mod tween;
mod world_validation;
