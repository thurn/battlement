//! Public display driving and deterministic worker controls for Reactant scenarios.
//!
//! [`Display`] runs a real public [`battlement_native::Engine`] through the
//! Battlement fake host. Presentation time, rendered frames, and worker
//! scheduling remain independent controls.

#![warn(missing_docs)]

mod display;
mod worker;

pub use display::Display;
pub use worker::{WorkerDisplay, WorkerDisplayBuilder, WorkerWaitError};
