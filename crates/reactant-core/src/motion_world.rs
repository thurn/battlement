//! World-space units and local interaction offsets for shared Motion targets.

use battlement::{MotionProperty, MotionValue};

use crate::{
  motion::{Keyframes, StyleTarget},
  motion_value::MotionValue as TypedMotionValue,
};

impl StyleTarget {
  /// Sets the `local_position_x` channel in world-units.
  #[must_use]
  pub fn local_position_x(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalPositionX,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_position_x` channel.
  #[must_use]
  pub fn local_position_x_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalPositionX,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_position_y` channel in world-units.
  #[must_use]
  pub fn local_position_y(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalPositionY,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_position_y` channel.
  #[must_use]
  pub fn local_position_y_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalPositionY,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_position_z` channel in world-units.
  #[must_use]
  pub fn local_position_z(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalPositionZ,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_position_z` channel.
  #[must_use]
  pub fn local_position_z_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalPositionZ,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_rotation_x` channel in degrees.
  #[must_use]
  pub fn local_rotation_x(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalRotationX,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_rotation_x` channel.
  #[must_use]
  pub fn local_rotation_x_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalRotationX,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_rotation_y` channel in degrees.
  #[must_use]
  pub fn local_rotation_y(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalRotationY,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_rotation_y` channel.
  #[must_use]
  pub fn local_rotation_y_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalRotationY,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_rotation_z` channel in degrees.
  #[must_use]
  pub fn local_rotation_z(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalRotationZ,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_rotation_z` channel.
  #[must_use]
  pub fn local_rotation_z_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalRotationZ,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_scale_x` channel in numbers.
  #[must_use]
  pub fn local_scale_x(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalScaleX,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_scale_x` channel.
  #[must_use]
  pub fn local_scale_x_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalScaleX,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_scale_y` channel in numbers.
  #[must_use]
  pub fn local_scale_y(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalScaleY,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_scale_y` channel.
  #[must_use]
  pub fn local_scale_y_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalScaleY,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_scale_z` channel in numbers.
  #[must_use]
  pub fn local_scale_z(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalScaleZ,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_scale_z` channel.
  #[must_use]
  pub fn local_scale_z_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalScaleZ,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_offset_x` channel in world-units.
  #[must_use]
  pub fn local_offset_x(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalOffsetX,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_offset_x` channel.
  #[must_use]
  pub fn local_offset_x_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalOffsetX,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_offset_y` channel in world-units.
  #[must_use]
  pub fn local_offset_y(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalOffsetY,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_offset_y` channel.
  #[must_use]
  pub fn local_offset_y_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalOffsetY,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_offset_z` channel in world-units.
  #[must_use]
  pub fn local_offset_z(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalOffsetZ,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_offset_z` channel.
  #[must_use]
  pub fn local_offset_z_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalOffsetZ,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_tilt_x` channel in degrees.
  #[must_use]
  pub fn local_tilt_x(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalTiltX,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_tilt_x` channel.
  #[must_use]
  pub fn local_tilt_x_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalTiltX,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_tilt_y` channel in degrees.
  #[must_use]
  pub fn local_tilt_y(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalTiltY,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_tilt_y` channel.
  #[must_use]
  pub fn local_tilt_y_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalTiltY,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_tilt_z` channel in degrees.
  #[must_use]
  pub fn local_tilt_z(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalTiltZ,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_tilt_z` channel.
  #[must_use]
  pub fn local_tilt_z_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalTiltZ,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_scale_factor_x` channel in numbers.
  #[must_use]
  pub fn local_scale_factor_x(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalScaleFactorX,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_scale_factor_x` channel.
  #[must_use]
  pub fn local_scale_factor_x_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalScaleFactorX,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_scale_factor_y` channel in numbers.
  #[must_use]
  pub fn local_scale_factor_y(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalScaleFactorY,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_scale_factor_y` channel.
  #[must_use]
  pub fn local_scale_factor_y_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalScaleFactorY,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }

  /// Sets the `local_scale_factor_z` channel in numbers.
  #[must_use]
  pub fn local_scale_factor_z(self, value: f32) -> Self {
    self.set(
      MotionProperty::LocalScaleFactorZ,
      vec![MotionValue::Scalar(value)],
      None,
    )
  }

  /// Sets keyframes for the `local_scale_factor_z` channel.
  #[must_use]
  pub fn local_scale_factor_z_keyframes(self, value: Keyframes<f32>) -> Self {
    self.set(
      MotionProperty::LocalScaleFactorZ,
      value.values.into_iter().map(MotionValue::Scalar).collect(),
      value.times,
    )
  }
  /// Binds `local_position_x` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_position_x_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalPositionX, value.erase())
  }

  /// Binds `local_position_y` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_position_y_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalPositionY, value.erase())
  }

  /// Binds `local_position_z` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_position_z_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalPositionZ, value.erase())
  }

  /// Binds `local_rotation_x` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_rotation_x_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalRotationX, value.erase())
  }

  /// Binds `local_rotation_y` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_rotation_y_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalRotationY, value.erase())
  }

  /// Binds `local_rotation_z` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_rotation_z_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalRotationZ, value.erase())
  }

  /// Binds `local_scale_x` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_scale_x_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalScaleX, value.erase())
  }

  /// Binds `local_scale_y` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_scale_y_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalScaleY, value.erase())
  }

  /// Binds `local_scale_z` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_scale_z_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalScaleZ, value.erase())
  }

  /// Binds `local_offset_x` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_offset_x_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalOffsetX, value.erase())
  }

  /// Binds `local_offset_y` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_offset_y_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalOffsetY, value.erase())
  }

  /// Binds `local_offset_z` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_offset_z_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalOffsetZ, value.erase())
  }

  /// Binds `local_tilt_x` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_tilt_x_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalTiltX, value.erase())
  }

  /// Binds `local_tilt_y` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_tilt_y_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalTiltY, value.erase())
  }

  /// Binds `local_tilt_z` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_tilt_z_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalTiltZ, value.erase())
  }

  /// Binds `local_scale_factor_x` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_scale_factor_x_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalScaleFactorX, value.erase())
  }

  /// Binds `local_scale_factor_y` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_scale_factor_y_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalScaleFactorY, value.erase())
  }

  /// Binds `local_scale_factor_z` to a native Motion value without component evaluation.
  #[must_use]
  pub fn local_scale_factor_z_value(self, value: TypedMotionValue<f32>) -> Self {
    self.bind(MotionProperty::LocalScaleFactorZ, value.erase())
  }
}
