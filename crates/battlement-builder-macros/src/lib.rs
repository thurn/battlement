//! Attribute expansion for same-struct property builders.

#![forbid(unsafe_code)]

mod conversion;
mod expand;
mod input;
mod names;
mod projection;

use proc_macro::TokenStream;

/// Generates `new()` and consuming property setters on the annotated struct.
///
/// Fields use `Default` unless marked `#[builder(required)]` or given
/// `#[builder(default = expression)]`. Required setters may be called in any
/// order, once each. The struct's unqualified type names its complete state.
/// Place this attribute before derives. Use `support = ::module` when importing
/// outside the Reactant prelude's default support path.
#[proc_macro_attribute]
pub fn builder(arguments: TokenStream, item: TokenStream) -> TokenStream {
  match input::parse(arguments.into(), item.into()).and_then(expand::expand) {
    Ok(tokens) => tokens.into(),
    Err(error) => error.into_compile_error().into(),
  }
}
