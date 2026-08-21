//! An in-memory Masonry client for fast rules-engine tests.
//!
//! The fake applies validated protocol snapshots and commands synchronously,
//! exposing the resulting world and an execution journal without Unity,
//! serialization, wall-clock time, or background work.

#![warn(missing_docs)]

pub mod assets;
pub mod client;
pub mod journal;
pub mod world;

mod executor;
mod transform;
mod tween;
mod world_validation;
