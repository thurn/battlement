//! Shared syntax and identity contracts for Reactant generated assets.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod metadata;
mod model;
mod parser;
mod token;

pub use model::{
  AssetRequest, ClipEdge, Compression, FilterMode, GeneratorMetadata, Insets, LogicalRect,
  LogicalSize, PaintDeclaration, WrapMode,
};
pub use parser::{
  DeclarationEnvelope, DeclarationKind, Diagnostic, DiagnosticCategory, RawStatement, StatementName,
};
pub use token::SourceSpan;

/// Default number of raster pixels generated for each logical canvas unit.
pub const DEFAULT_RASTER_SCALE: u8 = 2;

/// Parses and validates one complete generated-asset request.
pub fn parse(source: &str) -> Result<AssetRequest, Diagnostic> {
  metadata::validate(parser::parse(source)?)
}

/// Parses only the token-level declaration envelope.
pub fn parse_envelope(source: &str) -> Result<DeclarationEnvelope, Diagnostic> {
  parser::parse(source)
}
