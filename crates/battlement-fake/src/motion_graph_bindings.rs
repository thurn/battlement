//! Adapts sampled graph values to host property shapes and reduced-motion policy.

use battlement::{Length, MotionProperty, MotionValue};

pub(crate) fn adapt(property: MotionProperty, value: &MotionValue) -> MotionValue {
  let MotionValue::Scalar(scalar) = value else {
    return value.clone();
  };
  match property {
    MotionProperty::X
    | MotionProperty::Y
    | MotionProperty::Z
    | MotionProperty::Width
    | MotionProperty::Height
    | MotionProperty::MinWidth
    | MotionProperty::MinHeight
    | MotionProperty::MaxWidth
    | MotionProperty::MaxHeight => MotionValue::Length(Length::Px(*scalar)),
    MotionProperty::Scale => MotionValue::Vector2([*scalar; 2]),
    _ => value.clone(),
  }
}

pub(crate) fn reduced(property: MotionProperty, destination: &MotionValue) -> MotionValue {
  if property >= MotionProperty::LocalPositionX && property <= MotionProperty::LocalScaleZ {
    return destination.clone();
  }
  match property {
    MotionProperty::X | MotionProperty::Y | MotionProperty::Z => {
      MotionValue::Length(Length::Px(0.0))
    }
    MotionProperty::Translate => MotionValue::Vector2([0.0; 2]),
    MotionProperty::Scale => MotionValue::Vector2([1.0; 2]),
    MotionProperty::ScaleX
    | MotionProperty::ScaleY
    | MotionProperty::LocalScaleFactorX
    | MotionProperty::LocalScaleFactorY
    | MotionProperty::LocalScaleFactorZ => MotionValue::Scalar(1.0),
    MotionProperty::LocalOffsetX
    | MotionProperty::LocalOffsetY
    | MotionProperty::LocalOffsetZ
    | MotionProperty::LocalTiltX
    | MotionProperty::LocalTiltY
    | MotionProperty::LocalTiltZ => MotionValue::Scalar(0.0),
    MotionProperty::Rotate
    | MotionProperty::RotateX
    | MotionProperty::RotateY
    | MotionProperty::SkewX
    | MotionProperty::SkewY => MotionValue::Angle(0.0),
    MotionProperty::TransformList => MotionValue::TransformList(Vec::new()),
    _ => panic!("reduced Motion requires a spatial property"),
  }
}
