//! Typed, serializable UI documents authored by Battlement rules engines.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod documents;
mod elements;
mod validation;

pub use documents::*;
pub use elements::*;
pub use validation::*;

fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    value == &T::default()
}
