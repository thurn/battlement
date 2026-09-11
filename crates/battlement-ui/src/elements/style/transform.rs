use serde::{Deserialize, Serialize};

use super::Length;

/// A rotation in degrees around a finite three-dimensional axis.
///
/// UI Toolkit applies this after scale and before translation without changing
/// layout. Positive angles rotate clockwise in panel space. The axis may point
/// in any direction but must not be the zero vector.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Rotate {
  /// Horizontal axis component.
  pub x: f32,
  /// Vertical axis component.
  pub y: f32,
  /// Depth axis component.
  pub z: f32,
  /// Clockwise rotation angle in degrees.
  pub degrees: f32,
}

impl Rotate {
  /// Creates a rotation around the supplied axis in degrees.
  #[must_use]
  pub const fn new(x: f32, y: f32, z: f32, degrees: f32) -> Self {
    Self { x, y, z, degrees }
  }

  /// Creates a panel-plane rotation around the positive z axis.
  #[must_use]
  pub const fn degrees(value: f32) -> Self {
    Self::new(0.0, 0.0, 1.0, value)
  }
}

/// Apparent horizontal and vertical size multipliers for an element.
///
/// Scale affects painting rather than flex layout. Negative values mirror the
/// element on that axis, and descendants are transformed with their parent.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Scale {
  /// Horizontal size multiplier.
  pub x: f32,
  /// Vertical size multiplier.
  pub y: f32,
}

impl Scale {
  /// Creates independent horizontal and vertical scale factors.
  #[must_use]
  pub const fn new(x: f32, y: f32) -> Self {
    Self { x, y }
  }

  /// Creates a uniform scale factor for both axes.
  #[must_use]
  pub const fn uniform(value: f32) -> Self {
    Self::new(value, value)
  }
}

/// A paint-time offset relative to an element's own size and position.
///
/// Percentage x and y values resolve against the element itself, not its
/// parent. The z component is measured in panel pixels. Translation does not
/// reserve layout space and is applied after scale and rotation.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct Translate {
  /// Horizontal pixel or self-relative percentage offset.
  pub x: Length,
  /// Vertical pixel or self-relative percentage offset.
  pub y: Length,
  /// Depth offset in panel pixels.
  pub z: f32,
}

impl Translate {
  /// Creates a translation with an explicit depth offset.
  #[must_use]
  pub const fn new(x: Length, y: Length, z: f32) -> Self {
    Self { x, y, z }
  }

  /// Creates a two-dimensional translation at zero depth.
  #[must_use]
  pub const fn two_dimensional(x: Length, y: Length) -> Self {
    Self::new(x, y, 0.0)
  }
}

/// Pivot used by an element's scale and rotation transforms.
///
/// Percentage x and y values resolve against the element bounds. Values may
/// lie outside those bounds, allowing an element to orbit an external pivot.
/// The z component is measured in panel pixels.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TransformOrigin {
  /// Horizontal pixel or self-relative percentage pivot.
  pub x: Length,
  /// Vertical pixel or self-relative percentage pivot.
  pub y: Length,
  /// Depth pivot in panel pixels.
  pub z: f32,
}

impl TransformOrigin {
  /// Creates a transform pivot with an explicit depth coordinate.
  #[must_use]
  pub const fn new(x: Length, y: Length, z: f32) -> Self {
    Self { x, y, z }
  }

  /// Creates a two-dimensional transform pivot at zero depth.
  #[must_use]
  pub const fn two_dimensional(x: Length, y: Length) -> Self {
    Self::new(x, y, 0.0)
  }
}
