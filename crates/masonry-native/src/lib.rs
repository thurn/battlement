//! Native adapter primitives for typed Masonry rules engines.
//!
//! This crate owns the raw-buffer boundary and MessagePack conversion. A game
//! supplies an [`Engine`] and [`EngineFactory`]; the export macro and panic
//! containment are added separately by the final ABI layer.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

mod adapter;
mod engine;

pub use adapter::*;
pub use engine::*;
