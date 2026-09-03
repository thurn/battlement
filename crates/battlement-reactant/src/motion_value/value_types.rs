use std::time::Duration;

use battlement::{Color, FilterFunction, FilterList, Length, TransformOperation};

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

impl private::MotionValueTypeSealed for FilterList {}
impl MotionValueType for FilterList {
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
    FilterList::new(mix_filters(from.as_slice(), to.as_slice(), progress))
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

fn mix_filters(
  from: &[FilterFunction],
  to: &[FilterFunction],
  progress: f64,
) -> Vec<FilterFunction> {
  if from.len() != to.len() {
    return if progress < 0.5 { from } else { to }.to_vec();
  }
  from
    .iter()
    .zip(to)
    .map(|(from, to)| match (from, to) {
      (FilterFunction::Blur(from), FilterFunction::Blur(to)) => {
        FilterFunction::Blur(f32::mix(from, to, progress))
      }
      (FilterFunction::Brightness(from), FilterFunction::Brightness(to)) => {
        FilterFunction::Brightness(f32::mix(from, to, progress))
      }
      (FilterFunction::Saturate(from), FilterFunction::Saturate(to)) => {
        FilterFunction::Saturate(f32::mix(from, to, progress))
      }
      (FilterFunction::Contrast(from), FilterFunction::Contrast(to)) => {
        FilterFunction::Contrast(f32::mix(from, to, progress))
      }
      (FilterFunction::HueRotate(from), FilterFunction::HueRotate(to)) => {
        FilterFunction::HueRotate(f32::mix(from, to, progress))
      }
      (FilterFunction::Opacity(from), FilterFunction::Opacity(to)) => {
        FilterFunction::Opacity(f32::mix(from, to, progress))
      }
      (FilterFunction::Invert(from), FilterFunction::Invert(to)) => {
        FilterFunction::Invert(f32::mix(from, to, progress))
      }
      (FilterFunction::Grayscale(from), FilterFunction::Grayscale(to)) => {
        FilterFunction::Grayscale(f32::mix(from, to, progress))
      }
      (FilterFunction::Sepia(from), FilterFunction::Sepia(to)) => {
        FilterFunction::Sepia(f32::mix(from, to, progress))
      }
      (FilterFunction::Tint(from), FilterFunction::Tint(to)) => {
        FilterFunction::Tint(mix_color(from, to, progress))
      }
      _ => *if progress < 0.5 { from } else { to },
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
