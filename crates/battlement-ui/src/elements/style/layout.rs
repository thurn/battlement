use serde::{Deserialize, Serialize};

/// A finite UI Toolkit length preserving pixel and percentage components.
///
/// Percentages are not clamped to `0..=100`; oversize dimensions and offsets
/// are useful layout inputs. Property-specific validation can still reject
/// negative values where Unity expects a nonnegative size or spacing value.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum Length {
  /// A device-independent UI Toolkit pixel length.
  Px(f32),
  /// A percentage of the property's containing dimension.
  Percent(f32),
  /// A typed `calc(px + percent)` length.
  Calc {
    /// Absolute UI Toolkit panel pixels.
    px: f32,
    /// Parent- or self-relative percentage points.
    percent: f32,
  },
}

impl Length {
  /// Creates a pure pixel length.
  #[must_use]
  pub const fn px(value: f32) -> Self {
    Self::Px(value)
  }

  /// Creates a pure percentage length.
  #[must_use]
  pub const fn percent(value: f32) -> Self {
    Self::Percent(value)
  }

  /// Creates a typed `calc(px + percent)` length.
  #[must_use]
  pub const fn calc(px: f32, percent: f32) -> Self {
    if px == 0.0 {
      Self::Percent(percent)
    } else if percent == 0.0 {
      Self::Px(px)
    } else {
      Self::Calc { px, percent }
    }
  }

  /// Returns the independent pixel and percentage components.
  #[must_use]
  pub const fn components(self) -> [f32; 2] {
    match self {
      Self::Px(px) => [px, 0.0],
      Self::Percent(percent) => [0.0, percent],
      Self::Calc { px, percent } => [px, percent],
    }
  }

  /// Resolves this length against the property-specific reference dimension.
  #[must_use]
  pub fn resolve(self, reference: f64) -> f64 {
    let [px, percent] = self.components();
    f64::from(px) + f64::from(percent) * reference / 100.0
  }

  pub(crate) fn is_finite(self) -> bool {
    self.components().into_iter().all(f32::is_finite)
  }
}

impl From<i32> for Length {
  fn from(value: i32) -> Self {
    Self::Px(value as f32)
  }
}

impl From<u32> for Length {
  fn from(value: u32) -> Self {
    Self::Px(value as f32)
  }
}

impl From<f32> for Length {
  fn from(value: f32) -> Self {
    Self::Px(value)
  }
}

/// A finite UI Toolkit length that can also request automatic layout sizing.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum LengthOrAuto {
  /// A device-independent UI Toolkit pixel length.
  Px(f32),
  /// A percentage of the property's containing dimension.
  Percent(f32),
  /// Lets the UI Toolkit layout engine derive the value.
  Auto,
}

impl From<i32> for LengthOrAuto {
  fn from(value: i32) -> Self {
    Self::Px(value as f32)
  }
}

impl From<u32> for LengthOrAuto {
  fn from(value: u32) -> Self {
    Self::Px(value as f32)
  }
}

impl From<f32> for LengthOrAuto {
  fn from(value: f32) -> Self {
    Self::Px(value)
  }
}

impl From<Length> for LengthOrAuto {
  fn from(value: Length) -> Self {
    match value {
      Length::Px(value) => Self::Px(value),
      Length::Percent(value) => Self::Percent(value),
      Length::Calc { .. } => panic!("calc lengths are unsupported by automatic layout values"),
    }
  }
}

/// Extension methods for explicitly authored pixel and percentage lengths.
pub trait LengthUnits {
  /// Converts this number to a pixel [`Length`].
  fn px(self) -> Length;
  /// Converts this number to a percentage [`Length`].
  fn pct(self) -> Length;
}

impl LengthUnits for i32 {
  fn px(self) -> Length {
    Length::Px(self as f32)
  }

  fn pct(self) -> Length {
    Length::Percent(self as f32)
  }
}

impl LengthUnits for u32 {
  fn px(self) -> Length {
    Length::Px(self as f32)
  }

  fn pct(self) -> Length {
    Length::Percent(self as f32)
  }
}

impl LengthUnits for f32 {
  fn px(self) -> Length {
    Length::Px(self)
  }

  fn pct(self) -> Length {
    Length::Percent(self)
  }
}

/// A finite scalar used by numeric UI Toolkit style properties.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct FloatValue(pub f32);

impl From<i32> for FloatValue {
  fn from(value: i32) -> Self {
    Self(value as f32)
  }
}

impl From<u32> for FloatValue {
  fn from(value: u32) -> Self {
    Self(value as f32)
  }
}

impl From<f32> for FloatValue {
  fn from(value: f32) -> Self {
    Self(value)
  }
}

/// Preferred width-to-height relationship used while resolving automatic size.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum AspectRatio {
  /// Leaves the preferred ratio automatic.
  Auto,
  /// Uses the finite positive quotient `width / height` during layout.
  Ratio {
    /// Relative width component.
    width: f32,
    /// Relative height component.
    height: f32,
  },
}

impl AspectRatio {
  /// Creates a preferred ratio from positive width and height components.
  #[must_use]
  pub const fn new(width: f32, height: f32) -> Self {
    Self::Ratio { width, height }
  }
}

/// Cross-axis alignment for a flex container or item.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Align {
  /// Defers item alignment to its container's alignment behavior.
  Auto,
  /// Packs content at the cross-axis start.
  FlexStart,
  /// Centers content on the cross axis.
  Center,
  /// Packs content at the cross-axis end.
  FlexEnd,
  /// Expands auto-sized content across the available cross axis.
  Stretch,
}

/// Main-axis direction used by a flex container to lay out its children.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FlexDirection {
  /// Lays out children from top to bottom.
  Column,
  /// Lays out children from bottom to top.
  ColumnReverse,
  /// Lays out children from left to right.
  Row,
  /// Lays out children from right to left.
  RowReverse,
}

/// Multi-line placement behavior for a flex container.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FlexWrap {
  /// Keeps children on one line even when they exceed available space.
  NoWrap,
  /// Moves overflowing children onto additional lines.
  Wrap,
  /// Wraps onto additional lines in the reverse cross-axis direction.
  WrapReverse,
}

/// Main-axis distribution of children inside a flex container.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Justify {
  /// Packs children at the main-axis start.
  FlexStart,
  /// Centers children along the main axis.
  Center,
  /// Packs children at the main-axis end.
  FlexEnd,
  /// Distributes free space between adjacent children.
  SpaceBetween,
  /// Distributes free space around every child.
  SpaceAround,
  /// Distributes equal free space between children and container edges.
  SpaceEvenly,
}

/// Whether an element participates in flex flow or is positioned independently.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Position {
  /// Keeps the element in normal flex layout and applies offsets relative to it.
  Relative,
  /// Removes the element from flex flow and resolves offsets against its parent.
  Absolute,
}

/// Whether an element participates in layout and rendering.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Display {
  /// Keeps the element in UI Toolkit's flex layout and renders it.
  Flex,
  /// Removes the element and its descendants from layout and rendering.
  None,
}

/// Whether an element is drawn while retaining its layout space.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Visibility {
  /// Draws the element normally.
  Visible,
  /// Suppresses drawing while preserving the element's layout contribution.
  Hidden,
}

/// Whether descendants may paint outside an element's clipping boundary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Overflow {
  /// Allows descendant content to render beyond the element's bounds.
  Visible,
  /// Clips descendant content to the selected [`OverflowClipBox`].
  Hidden,
}

/// Box edge used when [`Overflow::Hidden`] clips descendant content.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OverflowClipBox {
  /// Clips at the outer edge of the padding box.
  PaddingBox,
  /// Clips at the inner content box, excluding padding.
  ContentBox,
}
