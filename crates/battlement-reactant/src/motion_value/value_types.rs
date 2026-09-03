use std::time::Duration;

use battlement::{Color, Gradient, GradientStop, Length, Shadow, TransformOperation};

use crate::{
  motion_filter::{MotionFilterList, PaintFilterList},
  motion_value::{MotionValueType, SpringValue, private},
};

macro_rules! scalar_value_type {
  ($type:ty, $into:expr, $pattern:pat => $from:expr, $scalar:expr) => {
    impl private::MotionValueTypeSealed for $type {}
    impl MotionValueType for $type {
      fn into_motion_value(self) -> battlement::MotionValue {
        $into(self)
      }
      fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
        match value {
          $pattern => Some($from),
          _ => None,
        }
      }
      fn mix(from: &Self, to: &Self, progress: f64) -> Self {
        let progress = progress as f32;
        *from + (*to - *from) * progress
      }
      fn range_scalar(&self) -> Option<f64> {
        Some($scalar(self))
      }
    }
  };
}

scalar_value_type!(
  f32,
  battlement::MotionValue::Scalar,
  battlement::MotionValue::Scalar(value) => *value,
  |value: &f32| f64::from(*value)
);

impl private::MotionValueTypeSealed for Duration {}
impl MotionValueType for Duration {
  fn into_motion_value(self) -> battlement::MotionValue {
    battlement::MotionValue::Scalar(self.as_secs_f32())
  }
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
    match value {
      battlement::MotionValue::Scalar(value) if *value >= 0.0 => {
        Some(Duration::from_secs_f32(*value))
      }
      _ => None,
    }
  }
  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    Duration::from_secs_f64(from.as_secs_f64() + (to.as_secs_f64() - from.as_secs_f64()) * progress)
  }
  fn range_scalar(&self) -> Option<f64> {
    Some(self.as_secs_f64())
  }
}

impl private::MotionValueTypeSealed for Length {}
impl private::SpringValueSealed for Length {}
impl MotionValueType for Length {
  fn into_motion_value(self) -> battlement::MotionValue {
    battlement::MotionValue::Length(self)
  }
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
    match value {
      battlement::MotionValue::Length(value) => Some(*value),
      _ => None,
    }
  }
  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    let progress = progress as f32;
    let [from_px, from_percent] = from.components();
    let [to_px, to_percent] = to.components();
    Length::calc(
      from_px + (to_px - from_px) * progress,
      from_percent + (to_percent - from_percent) * progress,
    )
  }
  fn range_scalar(&self) -> Option<f64> {
    let [px, percent] = self.components();
    (percent == 0.0).then_some(f64::from(px))
  }
}

impl private::MotionValueTypeSealed for Color {}
impl private::SpringValueSealed for Color {}
impl MotionValueType for Color {
  fn into_motion_value(self) -> battlement::MotionValue {
    battlement::MotionValue::Color(self)
  }
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
    match value {
      battlement::MotionValue::Color(value) => Some(*value),
      _ => None,
    }
  }
  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    mix_color(from, to, progress)
  }
  fn range_scalar(&self) -> Option<f64> {
    None
  }
}

impl private::MotionValueTypeSealed for Vec<TransformOperation> {}
impl MotionValueType for Vec<TransformOperation> {
  fn into_motion_value(self) -> battlement::MotionValue {
    battlement::MotionValue::TransformList(self)
  }
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
    match value {
      battlement::MotionValue::TransformList(value) => Some(value.clone()),
      _ => None,
    }
  }
  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    mix_transforms(from, to, progress)
  }
  fn range_scalar(&self) -> Option<f64> {
    None
  }
}

impl private::MotionValueTypeSealed for MotionFilterList {}
impl MotionValueType for MotionFilterList {
  fn into_motion_value(self) -> battlement::MotionValue {
    battlement::MotionValue::FilterList(self.into())
  }
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
    match value {
      battlement::MotionValue::FilterList(value) => Self::from_protocol(value),
      _ => None,
    }
  }
  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    Self::mix(from, to, progress)
  }
  fn range_scalar(&self) -> Option<f64> {
    None
  }
}

impl private::MotionValueTypeSealed for PaintFilterList {}
impl MotionValueType for PaintFilterList {
  fn into_motion_value(self) -> battlement::MotionValue {
    battlement::MotionValue::FilterList(self.into())
  }
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
    match value {
      battlement::MotionValue::FilterList(value) => Self::from_protocol(value),
      _ => None,
    }
  }
  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    Self::mix(from, to, progress)
  }
  fn range_scalar(&self) -> Option<f64> {
    None
  }
}

impl private::MotionValueTypeSealed for Vec<Shadow> {}
impl MotionValueType for Vec<Shadow> {
  fn into_motion_value(self) -> battlement::MotionValue {
    battlement::MotionValue::ShadowList(self)
  }
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
    match value {
      battlement::MotionValue::ShadowList(value) => Some(value.clone()),
      _ => None,
    }
  }
  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    mix_shadows(from, to, progress)
  }
  fn range_scalar(&self) -> Option<f64> {
    None
  }
}

impl private::MotionValueTypeSealed for Gradient {}
impl MotionValueType for Gradient {
  fn into_motion_value(self) -> battlement::MotionValue {
    battlement::MotionValue::Gradient(self)
  }
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
    match value {
      battlement::MotionValue::Gradient(value) => Some(value.clone()),
      _ => None,
    }
  }
  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    mix_gradients(from, to, progress)
  }
  fn range_scalar(&self) -> Option<f64> {
    None
  }
}

impl private::SpringValueSealed for f32 {}
impl SpringValue for f32 {}
impl SpringValue for Length {}
impl SpringValue for Color {}

fn mix_transforms(
  from: &[TransformOperation],
  to: &[TransformOperation],
  progress: f64,
) -> Vec<TransformOperation> {
  if from.len() != to.len() {
    return if progress < 0.5 { from } else { to }.to_vec();
  }
  from
    .iter()
    .zip(to)
    .map(|(from, to)| match (from, to) {
      (TransformOperation::Translate(from), TransformOperation::Translate(to)) => {
        TransformOperation::Translate(std::array::from_fn(|index| {
          Length::mix(&from[index], &to[index], progress)
        }))
      }
      (TransformOperation::Rotate(from), TransformOperation::Rotate(to)) => {
        TransformOperation::Rotate(std::array::from_fn(|index| {
          f32::mix(&from[index], &to[index], progress)
        }))
      }
      (TransformOperation::Skew(from), TransformOperation::Skew(to)) => {
        TransformOperation::Skew(std::array::from_fn(|index| {
          f32::mix(&from[index], &to[index], progress)
        }))
      }
      (TransformOperation::Scale(from), TransformOperation::Scale(to)) => {
        TransformOperation::Scale(std::array::from_fn(|index| {
          f32::mix(&from[index], &to[index], progress)
        }))
      }
      _ => if progress < 0.5 { from } else { to }.clone(),
    })
    .collect()
}

fn mix_color(from: &Color, to: &Color, progress: f64) -> Color {
  Color::rgba(
    from.r + (to.r - from.r) * progress,
    from.g + (to.g - from.g) * progress,
    from.b + (to.b - from.b) * progress,
    from.a + (to.a - from.a) * progress,
  )
}

fn mix_shadows(from: &[Shadow], to: &[Shadow], progress: f64) -> Vec<Shadow> {
  if from.len() != to.len() {
    return if progress < 0.5 { from } else { to }.to_vec();
  }
  from
    .iter()
    .zip(to)
    .map(|(from, to)| {
      if from.inset != to.inset {
        return *if progress < 0.5 { from } else { to };
      }
      Shadow {
        x: f32::mix(&from.x, &to.x, progress),
        y: f32::mix(&from.y, &to.y, progress),
        blur: f32::mix(&from.blur, &to.blur, progress),
        spread: f32::mix(&from.spread, &to.spread, progress),
        color: mix_color(&from.color, &to.color, progress),
        inset: to.inset,
      }
    })
    .collect()
}

fn mix_gradients(from: &Gradient, to: &Gradient, progress: f64) -> Gradient {
  match (from, to) {
    (
      Gradient::Linear {
        angle: from_angle,
        stops: from_stops,
      },
      Gradient::Linear {
        angle: to_angle,
        stops: to_stops,
      },
    ) if from_stops.len() == to_stops.len() => Gradient::Linear {
      angle: f32::mix(from_angle, to_angle, progress),
      stops: mix_gradient_stops(from_stops, to_stops, progress),
    },
    (
      Gradient::Radial {
        center: from_center,
        radius: from_radius,
        stops: from_stops,
      },
      Gradient::Radial {
        center: to_center,
        radius: to_radius,
        stops: to_stops,
      },
    ) if from_stops.len() == to_stops.len() => Gradient::Radial {
      center: std::array::from_fn(|index| {
        f32::mix(&from_center[index], &to_center[index], progress)
      }),
      radius: std::array::from_fn(|index| {
        f32::mix(&from_radius[index], &to_radius[index], progress)
      }),
      stops: mix_gradient_stops(from_stops, to_stops, progress),
    },
    _ => if progress < 0.5 { from } else { to }.clone(),
  }
}

fn mix_gradient_stops(
  from: &[GradientStop],
  to: &[GradientStop],
  progress: f64,
) -> Vec<GradientStop> {
  from
    .iter()
    .zip(to)
    .map(|(from, to)| GradientStop {
      color: mix_color(&from.color, &to.color, progress),
      position: f32::mix(&from.position, &to.position, progress),
    })
    .collect()
}
