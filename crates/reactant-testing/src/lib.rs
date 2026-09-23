//! Public display driving and deterministic worker controls for Reactant scenarios.
//!
//! [`Display`] runs a real public [`battlement_native::Engine`] through the
//! Battlement fake host. Presentation time, rendered frames, and worker
//! scheduling remain independent controls.

#![warn(missing_docs)]

pub mod assets;
pub mod benchmark;
mod display;
mod driving;
mod input;
mod observations;
mod publications;
mod storage;
pub mod temporal;
#[cfg(feature = "worker-fixture")]
mod worker;

pub use display::Display;
pub use driving::ActionResult;
pub use publications::PublicationDisplay;
pub use storage::MemoryPersistence;
#[cfg(feature = "worker-fixture")]
pub use worker::{WorkerDisplay, WorkerDisplayBuilder, WorkerWaitError};
