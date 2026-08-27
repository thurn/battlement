//! Shared value types used by Battlement protocol domains.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod assets;
mod ids;
mod value_builders;
mod values;

pub use assets::*;
pub use ids::*;
pub use values::*;

#[doc(hidden)]
pub mod __private {
  pub use uuid::{Uuid, uuid};
}

pub(crate) fn default_one() -> f64 {
  1.0
}

pub(crate) fn is_one(value: &f64) -> bool {
  *value == 1.0
}
