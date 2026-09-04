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
  LocalDependency, LogicalRect, LogicalSize, NativeSupport, PaintDeclaration, WrapMode,
};
pub use parser::{
  DeclarationEnvelope, DeclarationKind, Diagnostic, DiagnosticCategory, RawStatement, StatementName,
};
pub use token::SourceSpan;

/// Default number of raster pixels generated for each logical canvas unit.
pub const DEFAULT_RASTER_SCALE: u8 = 2;

/// Parses and validates one complete generated-asset request.
pub fn parse(source: &str) -> Result<AssetRequest, Diagnostic> {
  let (request, support) = metadata::validate(parser::parse(source)?)?;
  match support {
    NativeSupport::GeneratorRequired => Ok(request),
    NativeSupport::NativeOnly { replacements } => Err(Diagnostic {
      category: DiagnosticCategory::NativeOnly,
      symbol: Some(request.symbol),
      property: None,
      replacement: Some(replacements.join(", ")),
      span: request.span,
    }),
  }
}

/// Expands one asset-family declaration into complete ordinary declarations.
///
/// Statements in each named member replace common statements with the same
/// name and append member-only statements in authored order.
pub fn expand_family(source: &str) -> Result<Vec<String>, Diagnostic> {
  parser::expand_family(source)
}

/// Classifies a complete declaration against Battlement's native UI surface.
pub fn classify_native_support(source: &str) -> Result<NativeSupport, Diagnostic> {
  metadata::validate(parser::parse(source)?).map(|(_, support)| support)
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

/// Serializes one already supported property value as browser CSS.
pub fn serialize_property_value(property: &str, source: &str) -> Result<String, Diagnostic> {
  value::parse_property(property, source)
    .and_then(|_| value::serialize_css(source))
    .map_err(value::standalone_diagnostic)
}

/// Returns the SHA-256 identity of one standalone scalar CSS value.
pub fn value_identity(source: &str) -> Result<[u8; 32], Diagnostic> {
  canonicalize_value(source).map(|bytes| canonical::identity(&bytes))
}
