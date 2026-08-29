//! Strict JSON models shared by the Ditto host and player.

pub mod baseline_state;
pub mod common;
pub mod job;
pub mod lifecycle;
pub mod outcome;
pub mod player_errors;
pub mod result;
pub mod review;
pub mod run_storage;

mod baseline_state_validation;
mod completion_validation;
mod lifecycle_validation;
mod log_validation;
mod result_format;
mod result_nested_validation;
mod result_validation;
mod review_validation;
mod run_retention;
pub(crate) mod run_storage_io;
mod validation;
