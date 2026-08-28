//! Deterministic test implementations of Battlement Unity Cloud services.
//!
//! Each service fake lives in the module matching its production contract. The
//! [`diagnostics`] module simulates local Unity Diagnostics behavior without
//! starting Unity or contacting the Unity Dashboard.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod diagnostics;
