//! Strict JSON models shared by the Ditto host and player.

pub mod baseline_state;
pub mod common;
pub mod job;
pub mod lifecycle;
pub mod result;
pub mod review;

mod baseline_state_validation;
mod completion_validation;
mod lifecycle_validation;
mod log_validation;
mod result_format;
mod result_nested_validation;
mod result_validation;
mod review_validation;
mod validation;
