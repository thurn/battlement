use battlement_types::Color;
use serde::{Deserialize, Serialize};

use crate::{
  FilterFunction, FilterList, Gradient, GradientStop, Length, Shadow, TransformOperation,
};

/// Every normalized value shape accepted by [`MotionProperty`](crate::MotionProperty).
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum MotionValue {
  /// One finite scalar.
  Scalar(f32),
  /// One typed length.
  Length(Length),
  /// One RGBA color.
  Color(Color),
  /// One two-channel vector.
  Vector2([f32; 2]),
  /// One three-channel vector.
  Vector3([f32; 3]),
  /// One angle in degrees.
  Angle(f32),
  /// One ordered transform list.
  TransformList(Vec<TransformOperation>),
  /// One ordered filter list.
  FilterList(FilterList),
  /// One compatible shadow list.
  ShadowList(Vec<Shadow>),
  /// One compatible gradient.
  Gradient(Gradient),
  /// Top, right, bottom, and left clip insets.
  ClipInset([Length; 4]),
  /// Ordered polygon vertices.
  ClipPolygon(Vec<[Length; 2]>),
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
      Self::Color(value)
        if [value.r, value.g, value.b, value.a]
          .into_iter()
          .any(|value| !value.is_finite()) =>
      {
        Err("motion color must be finite")
      }
      Self::Vector2(values) if values.iter().any(|value| !value.is_finite()) => {
        Err("motion vector must be finite")
      }
      Self::Vector3(values) if values.iter().any(|value| !value.is_finite()) => {
        Err("motion vector must be finite")
      }
      Self::TransformList(values) => validate_transforms(values),
      Self::FilterList(values) => validate_filters(values.as_slice()),
      Self::ShadowList(values) if values.iter().any(|value| !shadow_is_finite(*value)) => {
        Err("motion shadow must be finite")
      }
      Self::ShadowList(values) if values.iter().any(|value| value.blur != 0.0) => {
        Err("motion box-shadow blur is unsupported by Unity; use generated paint")
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

fn validate_transforms(values: &[TransformOperation]) -> Result<(), &'static str> {
  for value in values {
    let finite = match value {
      TransformOperation::Translate(values) => values.iter().all(|value| value.is_finite()),
      TransformOperation::Rotate(values) | TransformOperation::Scale(values) => {
        values.iter().all(|value| value.is_finite())
      }
      TransformOperation::Skew(values) => values.iter().all(|value| value.is_finite()),
    };
    if !finite {
      return Err("motion transform must be finite");
    }
  }
  Ok(())
}

fn validate_filters(values: &[FilterFunction]) -> Result<(), &'static str> {
  for value in values {
    let finite = match value {
      FilterFunction::Brightness(_) => return Err("motion brightness is unsupported by Unity"),
      FilterFunction::Saturate(_) => return Err("motion saturation is unsupported by Unity"),
      FilterFunction::DropShadow(_) => {
        return Err("motion filter drop-shadow is unsupported by Unity");
      }
      FilterFunction::Blur(value)
      | FilterFunction::Contrast(value)
      | FilterFunction::HueRotate(value)
      | FilterFunction::Opacity(value)
      | FilterFunction::Invert(value)
      | FilterFunction::Grayscale(value)
      | FilterFunction::Sepia(value) => value.is_finite(),
      FilterFunction::Tint(value) => [value.r, value.g, value.b, value.a]
        .into_iter()
        .all(f64::is_finite),
    };
    if !finite {
      return Err("motion filter must be finite");
    }
  }
  Ok(())
}

fn validate_gradient(value: &Gradient) -> Result<(), &'static str> {
  let (geometry, stops): (&[f32], &[GradientStop]) = match value {
    Gradient::Linear { angle, stops } => (std::slice::from_ref(angle), stops),
    Gradient::Radial {
      center,
      radius,
      stops,
    } => (&[center[0], center[1], radius[0], radius[1]], stops),
  };
  if stops.is_empty()
    || geometry.iter().any(|value| !value.is_finite())
    || stops.iter().any(|stop| {
      !stop.position.is_finite()
        || [stop.color.r, stop.color.g, stop.color.b, stop.color.a]
          .into_iter()
          .any(|value| !value.is_finite())
    })
  {
    return Err("motion gradient must contain finite geometry and stops");
  }
  Ok(())
}

fn shadow_is_finite(value: Shadow) -> bool {
  [value.x, value.y, value.blur, value.spread]
    .into_iter()
    .all(f32::is_finite)
    && [value.color.r, value.color.g, value.color.b, value.color.a]
      .into_iter()
      .all(f64::is_finite)
}
