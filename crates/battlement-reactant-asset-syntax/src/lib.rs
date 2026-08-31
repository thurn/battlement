//! Shared syntax and identity contracts for Reactant generated assets.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod canonical;
mod metadata;
mod model;
mod parser;
mod token;
mod value;

pub use model::{
  AssetRequest, ClipEdge, Compression, DependencyKind, FilterMode, GeneratorMetadata, Insets,
  LocalDependency, LogicalRect, LogicalSize, PaintDeclaration, WrapMode,
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

/// Canonicalizes one standalone scalar CSS value.
pub fn canonicalize_value(source: &str) -> Result<Vec<u8>, Diagnostic> {
  value::canonicalize(source).map_err(value::standalone_diagnostic)
}

/// Serializes a scalar CSS value using shortest round-tripping decimals.
pub fn serialize_value(source: &str) -> Result<String, Diagnostic> {
  value::serialize(source).map_err(value::standalone_diagnostic)
}

/// Returns the SHA-256 identity of one standalone scalar CSS value.
pub fn value_identity(source: &str) -> Result<[u8; 32], Diagnostic> {
  canonicalize_value(source).map(|bytes| canonical::identity(&bytes))
}
