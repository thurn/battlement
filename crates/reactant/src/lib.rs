//! Ergonomic Reactant application facade.
//!
//! This crate combines the shared component runtime, Battlement UI authoring,
//! and typed rules-session machinery while keeping those lower layers acyclic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use reactant_core::*;

/// Typed rules execution and worker lifecycle support.
pub use reactant_rules as rules;

/// Battlement UI controls and host adapters.
pub use reactant_ui as ui;
