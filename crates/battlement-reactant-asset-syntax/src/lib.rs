//! Shared syntax and identity contracts for Reactant generated assets.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod parser;
mod token;

pub use parser::{
  DeclarationEnvelope, DeclarationKind, Diagnostic, DiagnosticCategory, RawStatement, StatementName,
};
pub use token::SourceSpan;

/// Default number of raster pixels generated for each logical canvas unit.
pub const DEFAULT_RASTER_SCALE: u8 = 2;

/// Parses one complete generated-asset declaration envelope.
pub fn parse(source: &str) -> Result<DeclarationEnvelope, Diagnostic> {
  parser::parse(source)
}
