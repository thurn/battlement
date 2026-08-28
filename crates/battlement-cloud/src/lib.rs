//! Rust contracts for Unity Cloud services exposed through Battlement.
//!
//! Each supported Unity Cloud service has its own module containing its commands,
//! validation, and serialized protocol values. Applications only need to depend on
//! the modules for the services they use.
//!
//! The [`diagnostics`] module enriches future Unity Diagnostics reports with custom
//! metadata.
//! Additional Unity Cloud integrations can be added alongside it without expanding
//! the Diagnostics API or changing its namespace.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod diagnostics;
