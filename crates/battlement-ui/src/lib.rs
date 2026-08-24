//! Typed, serializable UI documents authored by Battlement rules engines.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod commands;
mod documents;
mod elements;
mod events;
mod validation;

pub use commands::*;
pub use documents::*;
pub use elements::*;
pub use events::*;
pub use validation::*;

fn is_default<T>(value: &T) -> bool
where
    T: Default + PartialEq,
{
    value == &T::default()
}
