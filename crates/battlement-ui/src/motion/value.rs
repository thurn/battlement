use serde::{Deserialize, Serialize};

/// A length preserving pixel and percentage components during interpolation.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MotionLength {
  /// Absolute UI Toolkit panel pixels.
  pub px: f32,
  /// Parent- or self-relative percentage points.
  pub percent: f32,
}

impl MotionLength {
  /// Creates a pure pixel length.
  #[must_use]
  pub const fn px(value: f32) -> Self {
    Self {
      px: value,
      percent: 0.0,
    }
  }

  /// Creates a pure percentage length.
  #[must_use]
  pub const fn percent(value: f32) -> Self {
    Self {
      px: 0.0,
      percent: value,
    }
  }

  /// Creates a typed `calc(px + percent)` length.
  #[must_use]
  pub const fn calc(px: f32, percent: f32) -> Self {
    Self { px, percent }
  }

  pub(crate) fn is_finite(self) -> bool {
    self.px.is_finite() && self.percent.is_finite()
  }

  /// Resolves the length against its property-specific reference dimension.
  #[must_use]
  pub fn resolve(self, reference: f64) -> f64 {
    f64::from(self.px) + f64::from(self.percent) * reference / 100.0
  }
}

impl From<f32> for MotionLength {
  fn from(value: f32) -> Self {
    Self::px(value)
  }
}

/// A Motion-compatible linear RGBA color value.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionColor {
  /// Red channel in `0..=1` for ordinary colors.
  pub red: f32,
  /// Green channel in `0..=1` for ordinary colors.
  pub green: f32,
  /// Blue channel in `0..=1` for ordinary colors.
  pub blue: f32,
  /// Alpha channel in `0..=1` for ordinary colors.
  pub alpha: f32,
}

impl MotionColor {
  /// Creates a color from finite channels.
  #[must_use]
  pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> Self {
    Self {
      red,
      green,
      blue,
      alpha,
    }
  }

  pub(crate) fn channels(self) -> [f32; 4] {
    [self.red, self.green, self.blue, self.alpha]
  }
}

/// One typed operation in an authored transform list.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionTransform {
  /// Three-axis translation.
  Translate([MotionLength; 3]),
  /// Three-axis rotation in degrees.
  Rotate([f32; 3]),
  /// Two-axis skew in degrees.
  Skew([f32; 2]),
  /// Three-axis scale.
  Scale([f32; 3]),
}

/// One supported filter operation.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionFilter {
  /// Gaussian blur radius in pixels.
  Blur(f32),
  /// Brightness multiplier.
  Brightness(f32),
  /// Saturation multiplier.
  Saturate(f32),
  /// Contrast multiplier.
  Contrast(f32),
  /// Hue rotation in degrees.
  HueRotate(f32),
  /// Opacity multiplier.
  Opacity(f32),
  /// Motion-compatible drop shadow.
  DropShadow(MotionShadow),
}

/// One outer or inset shadow.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionShadow {
  /// Horizontal offset in pixels.
  pub x: f32,
  /// Vertical offset in pixels.
  pub y: f32,
  /// Blur radius in pixels.
  pub blur: f32,
  /// Spread radius in pixels.
  pub spread: f32,
  /// Shadow color.
  pub color: MotionColor,
  /// Whether the shadow paints inward.
  pub inset: bool,
}

/// One gradient color stop.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MotionGradientStop {
  /// Stop color.
  pub color: MotionColor,
  /// Normalized stop position.
  pub position: f32,
}

/// A compatible linear or radial gradient.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionGradient {
  /// Linear gradient with an angle in degrees.
  Linear {
    /// Gradient direction.
    angle: f32,
    /// Ordered normalized color stops.
    stops: Vec<MotionGradientStop>,
  },
  /// Radial gradient with normalized center and radii.
  Radial {
    /// Normalized center point.
    center: [f32; 2],
    /// Normalized horizontal and vertical radii.
    radius: [f32; 2],
    /// Ordered normalized color stops.
    stops: Vec<MotionGradientStop>,
  },
}

/// Every normalized value shape accepted by [`MotionProperty`](crate::MotionProperty).
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionValue {
  /// One finite scalar.
  Scalar(f32),
  /// One typed length.
  Length(MotionLength),
  /// One RGBA color.
  Color(MotionColor),
  /// One two-channel vector.
  Vector2([f32; 2]),
  /// One three-channel vector.
  Vector3([f32; 3]),
  /// One angle in degrees.
  Angle(f32),
  /// One ordered transform list.
  TransformList(Vec<MotionTransform>),
  /// One ordered filter list.
  FilterList(Vec<MotionFilter>),
  /// One compatible shadow list.
  ShadowList(Vec<MotionShadow>),
  /// One compatible gradient.
  Gradient(MotionGradient),
  /// Top, right, bottom, and left clip insets.
  ClipInset([MotionLength; 4]),
  /// Ordered polygon vertices.
  ClipPolygon(Vec<[MotionLength; 2]>),
  /// A catalog-declared discrete protocol value.
  Discrete(serde_json::Value),
}

impl MotionValue {
  pub(crate) fn validate(&self) -> Result<(), &'static str> {
    match self {
      Self::Scalar(value) | Self::Angle(value) if !value.is_finite() => {
        Err("motion scalar must be finite")
      }
      Self::Length(value) if !value.is_finite() => Err("motion length must be finite"),
      Self::Color(value) if value.channels().into_iter().any(|value| !value.is_finite()) => {
        Err("motion color must be finite")
      }
      Self::Vector2(values) if values.iter().any(|value| !value.is_finite()) => {
        Err("motion vector must be finite")
      }
      Self::Vector3(values) if values.iter().any(|value| !value.is_finite()) => {
        Err("motion vector must be finite")
      }
      Self::TransformList(values) => validate_transforms(values),
      Self::FilterList(values) => validate_filters(values),
      Self::ShadowList(values) if values.iter().any(|value| !shadow_is_finite(*value)) => {
        Err("motion shadow must be finite")
      }
      Self::Gradient(value) => validate_gradient(value),
      Self::ClipInset(values) if values.iter().any(|value| !value.is_finite()) => {
        Err("motion clip inset must be finite")
      }
      Self::ClipPolygon(values)
        if values.is_empty() || values.iter().flatten().any(|value| !value.is_finite()) =>
      {
        Err("motion clip polygon must contain finite vertices")
      }
      _ => Ok(()),
    }
  }
}

fn validate_transforms(values: &[MotionTransform]) -> Result<(), &'static str> {
  for value in values {
    let finite = match value {
      MotionTransform::Translate(values) => values.iter().all(|value| value.is_finite()),
      MotionTransform::Rotate(values) | MotionTransform::Scale(values) => {
        values.iter().all(|value| value.is_finite())
      }
      MotionTransform::Skew(values) => values.iter().all(|value| value.is_finite()),
    };
    if !finite {
      return Err("motion transform must be finite");
    }
  }
  Ok(())
}

fn validate_filters(values: &[MotionFilter]) -> Result<(), &'static str> {
  for value in values {
    let finite = match value {
      MotionFilter::Blur(value)
      | MotionFilter::Brightness(value)
      | MotionFilter::Saturate(value)
      | MotionFilter::Contrast(value)
      | MotionFilter::HueRotate(value)
      | MotionFilter::Opacity(value) => value.is_finite(),
      MotionFilter::DropShadow(value) => shadow_is_finite(*value),
    };
    if !finite {
      return Err("motion filter must be finite");
    }
  }
  Ok(())
}

fn validate_gradient(value: &MotionGradient) -> Result<(), &'static str> {
  let (geometry, stops): (&[f32], &[MotionGradientStop]) = match value {
    MotionGradient::Linear { angle, stops } => (std::slice::from_ref(angle), stops),
    MotionGradient::Radial {
      center,
      radius,
      stops,
    } => (&[center[0], center[1], radius[0], radius[1]], stops),
  };
  if stops.is_empty()
    || geometry.iter().any(|value| !value.is_finite())
    || stops.iter().any(|stop| {
      !stop.position.is_finite()
        || stop
          .color
          .channels()
          .into_iter()
          .any(|value| !value.is_finite())
    })
  {
    return Err("motion gradient must contain finite geometry and stops");
  }
  Ok(())
}

fn shadow_is_finite(value: MotionShadow) -> bool {
  [value.x, value.y, value.blur, value.spread]
    .into_iter()
    .chain(value.color.channels())
    .all(f32::is_finite)
}
