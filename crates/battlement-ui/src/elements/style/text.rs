use battlement_types::Color;
use serde::{Deserialize, Serialize};

/// Font face style and weight selected from the active UI font.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FontStyle {
  /// Uses the font's regular face.
  Normal,
  /// Uses the font's bold face or synthesized bold weight.
  Bold,
  /// Uses the font's italic face or synthesized slant.
  Italic,
  /// Combines bold weight and italic slant.
  BoldAndItalic,
}

/// Alignment of text within the element's content rectangle.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextAnchor {
  /// Aligns to the top-left corner.
  UpperLeft,
  /// Centers horizontally at the top edge.
  UpperCenter,
  /// Aligns to the top-right corner.
  UpperRight,
  /// Centers vertically at the left edge.
  MiddleLeft,
  /// Centers on both axes.
  MiddleCenter,
  /// Centers vertically at the right edge.
  MiddleRight,
  /// Aligns to the bottom-left corner.
  LowerLeft,
  /// Centers horizontally at the bottom edge.
  LowerCenter,
  /// Aligns to the bottom-right corner.
  LowerRight,
}

/// Whether UI Toolkit automatically chooses a font size that fits the box.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum TextAutoSize {
  /// Uses the separately authored font size.
  None,
  /// Chooses a fitting size between the inclusive pixel bounds.
  BestFit {
    /// Smallest font size Unity may choose, in pixels.
    min_size: f32,
    /// Largest font size Unity may choose, in pixels.
    max_size: f32,
  },
}

impl TextAutoSize {
  /// Creates ordered positive pixel bounds for best-fit text.
  #[must_use]
  pub const fn best_fit(min_size: f32, max_size: f32) -> Self {
    Self::BestFit { min_size, max_size }
  }
}

/// Text layout behavior when content exceeds the available width.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextOverflow {
  /// Cuts glyphs at the element's overflow boundary.
  Clip,
  /// Replaces hidden text with an ellipsis.
  Ellipsis,
}

/// Which portion of an elided string UI Toolkit preserves.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextOverflowPosition {
  /// Preserves the end of the string and elides its start.
  Start,
  /// Preserves both ends and elides the middle.
  Middle,
  /// Preserves the start and elides the end.
  End,
}

/// Whitespace preservation and wrapping behavior for rendered text.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum WhiteSpace {
  /// Collapses spaces and wraps lines to fit.
  Normal,
  /// Collapses spaces without automatic line wrapping.
  NoWrap,
  /// Preserves spaces and newlines without automatic wrapping.
  Pre,
  /// Preserves spaces and newlines while allowing wrapping.
  PreWrap,
}

/// Text rendering backend selected for the element.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextGenerator {
  /// Uses Unity's standard text generator.
  Standard,
  /// Uses Unity's advanced generator for complex scripts when available.
  Advanced,
}

/// Glyph rasterization mode used by Unity's editor text renderer.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum EditorTextRenderingMode {
  /// Uses signed-distance-field glyph rendering.
  Sdf,
  /// Uses bitmap glyph rendering.
  Bitmap,
}

/// Shadow painted behind every rendered text glyph.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TextShadow {
  /// Horizontal offset in panel pixels.
  pub x: f32,
  /// Vertical offset in panel pixels.
  pub y: f32,
  /// Nonnegative blur radius in panel pixels.
  pub blur_radius: f32,
  /// Shadow color multiplied with glyph coverage.
  pub color: Color,
}

impl TextShadow {
  /// Creates a text shadow from its pixel offset, blur radius, and color.
  #[must_use]
  pub const fn new(x: f32, y: f32, blur_radius: f32, color: Color) -> Self {
    Self {
      x,
      y,
      blur_radius,
      color,
    }
  }
}
