//! Public display driving and deterministic worker controls for Reactant scenarios.
//!
//! [`Display`] runs a real public [`battlement_native::Engine`] through the
//! Battlement fake host. Presentation time, rendered frames, and worker
//! scheduling remain independent controls.

#![warn(missing_docs)]

mod display;
mod inline;
mod publications;
pub mod temporal;
#[cfg(feature = "worker-fixture")]
mod worker;

pub use display::{Display, GameActionResult};
pub use inline::InlineActionResult;
pub use publications::PublicationDisplay;
#[cfg(feature = "worker-fixture")]
pub use worker::{WorkerDisplay, WorkerDisplayBuilder, WorkerWaitError};
