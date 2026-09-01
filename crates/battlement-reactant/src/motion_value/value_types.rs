use std::time::Duration;

use battlement::{MotionColor, MotionFilter, MotionLength, MotionTransform};

use crate::motion_value::{MotionValueType, SpringValue, private};

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

impl private::MotionValueTypeSealed for MotionLength {}
impl private::SpringValueSealed for MotionLength {}
impl MotionValueType for MotionLength {
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
    MotionLength::calc(
      from.px + (to.px - from.px) * progress,
      from.percent + (to.percent - from.percent) * progress,
    )
  }
  fn range_scalar(&self) -> Option<f64> {
    (self.percent == 0.0).then_some(f64::from(self.px))
  }
}

impl private::MotionValueTypeSealed for MotionColor {}
impl private::SpringValueSealed for MotionColor {}
impl MotionValueType for MotionColor {
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
    let progress = progress as f32;
    MotionColor::new(
      from.red + (to.red - from.red) * progress,
      from.green + (to.green - from.green) * progress,
      from.blue + (to.blue - from.blue) * progress,
      from.alpha + (to.alpha - from.alpha) * progress,
    )
  }
  fn range_scalar(&self) -> Option<f64> {
    None
  }
}

impl private::MotionValueTypeSealed for Vec<MotionTransform> {}
impl MotionValueType for Vec<MotionTransform> {
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

impl private::MotionValueTypeSealed for Vec<MotionFilter> {}
impl MotionValueType for Vec<MotionFilter> {
  fn into_motion_value(self) -> battlement::MotionValue {
    battlement::MotionValue::FilterList(self)
  }
  fn from_motion_value(value: &battlement::MotionValue) -> Option<Self> {
    match value {
      battlement::MotionValue::FilterList(value) => Some(value.clone()),
      _ => None,
    }
  }
  fn mix(from: &Self, to: &Self, progress: f64) -> Self {
    mix_filters(from, to, progress)
  }
  fn range_scalar(&self) -> Option<f64> {
    None
  }
}

impl private::SpringValueSealed for f32 {}
impl SpringValue for f32 {}
impl SpringValue for MotionLength {}
impl SpringValue for MotionColor {}

fn mix_transforms(
  from: &[MotionTransform],
  to: &[MotionTransform],
  progress: f64,
) -> Vec<MotionTransform> {
  if from.len() != to.len() {
    return if progress < 0.5 { from } else { to }.to_vec();
  }
  from
    .iter()
    .zip(to)
    .map(|(from, to)| match (from, to) {
      (MotionTransform::Translate(from), MotionTransform::Translate(to)) => {
        MotionTransform::Translate(std::array::from_fn(|index| {
          MotionLength::mix(&from[index], &to[index], progress)
        }))
      }
      (MotionTransform::Rotate(from), MotionTransform::Rotate(to)) => {
        MotionTransform::Rotate(std::array::from_fn(|index| {
          f32::mix(&from[index], &to[index], progress)
        }))
      }
      (MotionTransform::Skew(from), MotionTransform::Skew(to)) => {
        MotionTransform::Skew(std::array::from_fn(|index| {
          f32::mix(&from[index], &to[index], progress)
        }))
      }
      (MotionTransform::Scale(from), MotionTransform::Scale(to)) => {
        MotionTransform::Scale(std::array::from_fn(|index| {
          f32::mix(&from[index], &to[index], progress)
        }))
      }
      _ => if progress < 0.5 { from } else { to }.clone(),
    })
    .collect()
}

fn mix_filters(from: &[MotionFilter], to: &[MotionFilter], progress: f64) -> Vec<MotionFilter> {
  if from.len() != to.len() {
    return if progress < 0.5 { from } else { to }.to_vec();
  }
  from
    .iter()
    .zip(to)
    .map(|(from, to)| match (from, to) {
      (MotionFilter::Blur(from), MotionFilter::Blur(to)) => {
        MotionFilter::Blur(f32::mix(from, to, progress))
      }
      (MotionFilter::Brightness(from), MotionFilter::Brightness(to)) => {
        MotionFilter::Brightness(f32::mix(from, to, progress))
      }
      (MotionFilter::Saturate(from), MotionFilter::Saturate(to)) => {
        MotionFilter::Saturate(f32::mix(from, to, progress))
      }
      (MotionFilter::Contrast(from), MotionFilter::Contrast(to)) => {
        MotionFilter::Contrast(f32::mix(from, to, progress))
      }
      (MotionFilter::HueRotate(from), MotionFilter::HueRotate(to)) => {
        MotionFilter::HueRotate(f32::mix(from, to, progress))
      }
      (MotionFilter::Opacity(from), MotionFilter::Opacity(to)) => {
        MotionFilter::Opacity(f32::mix(from, to, progress))
      }
      _ => if progress < 0.5 { from } else { to }.clone(),
    })
    .collect()
}
