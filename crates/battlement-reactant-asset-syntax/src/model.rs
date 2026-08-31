use crate::{DeclarationKind, SourceSpan};

/// Logical canvas size in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalSize {
  /// Width in logical pixels.
  pub width: f64,
  /// Height in logical pixels.
  pub height: f64,
}

/// Logical subject rectangle in canvas coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalRect {
  /// Horizontal origin.
  pub x: f64,
  /// Vertical origin.
  pub y: f64,
  /// Subject width.
  pub width: f64,
  /// Subject height.
  pub height: f64,
}

/// Nine-slice insets in CSS order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Insets {
  /// Top inset.
  pub top: f64,
  /// Right inset.
  pub right: f64,
  /// Bottom inset.
  pub bottom: f64,
  /// Left inset.
  pub left: f64,
}

/// Canvas edge where rendered paint may be intentionally clipped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipEdge {
  /// Top edge.
  Top,
  /// Right edge.
  Right,
  /// Bottom edge.
  Bottom,
  /// Left edge.
  Left,
}

/// Unity texture filtering mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterMode {
  /// Bilinear interpolation.
  Bilinear,
  /// Nearest-neighbor sampling.
  Nearest,
}

/// Unity texture wrapping mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WrapMode {
  /// Clamp sampling at texture edges.
  Clamp,
  /// Repeat texture coordinates.
  Repeat,
}

/// Unity texture compression selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compression {
  /// Preserve lossless source pixels.
  Lossless,
  /// Use low-quality lossy compression.
  LossyLow,
  /// Use normal-quality lossy compression.
  LossyNormal,
  /// Use high-quality lossy compression.
  LossyHigh,
}

/// Validated generator metadata with all defaults applied.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratorMetadata {
  /// Logical output canvas.
  pub canvas: LogicalSize,
  /// Logical subject rectangle.
  pub subject: LogicalRect,
  /// Optional nine-slice geometry.
  pub slices: Option<Insets>,
  /// Ordered set of permitted clipping edges.
  pub allowed_clipping: Vec<ClipEdge>,
  /// Effective project or declaration raster scale.
  pub raster_scale: u8,
  /// Texture filtering mode.
  pub filter_mode: FilterMode,
  /// Texture wrapping mode.
  pub wrap_mode: WrapMode,
  /// Texture compression selection.
  pub compression: Compression,
  /// Unity-relative font dependency for text images.
  pub font_file: Option<String>,
}

/// One recognized CSS property whose typed value is parsed by later grammar stages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaintDeclaration {
  /// ASCII-lowercase property name.
  pub property: String,
  /// Rust-token representation of the authored value.
  pub value: String,
  /// Property-name source span.
  pub span: SourceSpan,
  pub(crate) canonical_value: Vec<u8>,
}

impl PaintDeclaration {
  /// Deterministic tagged encoding of the parsed property value.
  pub fn canonical_value(&self) -> &[u8] {
    &self.canonical_value
  }
}

/// One parsed asset request shared by macro expansion and host discovery.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetRequest {
  /// Generated static name.
  pub symbol: String,
  /// Generated handle family.
  pub kind: DeclarationKind,
  /// Complete validated generator metadata.
  pub metadata: GeneratorMetadata,
  /// Paint declarations sorted by property name.
  pub paint: Vec<PaintDeclaration>,
  /// Complete declaration source span.
  pub span: SourceSpan,
}
