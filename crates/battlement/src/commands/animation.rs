use serde::{Deserialize, Serialize};

use crate::ObjectId;

/// Plays an Animator state with explicit scheduling time.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnimatorPlayPayload {
  /// Target prefab game object with a supported Animator.
  pub object_id: ObjectId,
  /// Animator state name.
  pub state: String,
  /// Nonnegative Animator layer index.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub layer: u32,
  /// Normalized starting time in the inclusive range `[0, 1]`.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub normalized_start_time: f64,
  /// Explicit operation duration for group scheduling; zero does not wait.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub wait_ms: u64,
}

/// Cross-fades to an Animator state with explicit scheduling time.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnimatorCrossFadePayload {
  /// Target prefab game object with a supported Animator.
  pub object_id: ObjectId,
  /// Animator state name.
  pub state: String,
  /// Nonnegative Animator layer index.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub layer: u32,
  /// Normalized starting time in the inclusive range `[0, 1]`.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub normalized_start_time: f64,
  /// Explicit operation duration for group scheduling; zero does not wait.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub wait_ms: u64,
  /// Positive cross-fade duration in milliseconds.
  pub cross_fade_ms: u64,
}

/// Sets a persistent boolean Animator parameter.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnimatorBoolPayload {
  /// Target prefab game object with a supported Animator.
  pub object_id: ObjectId,
  /// Parameter name.
  pub parameter: String,
  /// New boolean value.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub value: bool,
}

/// Sets a persistent signed 32-bit Animator parameter.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnimatorIntPayload {
  /// Target prefab game object with a supported Animator.
  pub object_id: ObjectId,
  /// Parameter name.
  pub parameter: String,
  /// New signed 32-bit value.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub value: i32,
}

/// Sets a persistent finite floating-point Animator parameter.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnimatorFloatPayload {
  /// Target prefab game object with a supported Animator.
  pub object_id: ObjectId,
  /// Parameter name.
  pub parameter: String,
  /// New finite floating-point value.
  #[serde(default, skip_serializing_if = "crate::is_default")]
  pub value: f64,
}

/// Names an Animator parameter without an associated value.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnimatorParameterPayload {
  /// Target prefab game object with a supported Animator.
  pub object_id: ObjectId,
  /// Parameter name.
  pub parameter: String,
}

/// Sets nonnegative Animator playback speed.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct AnimatorSpeedPayload {
  /// Target prefab game object with a supported Animator.
  pub object_id: ObjectId,
  /// Nonnegative playback speed.
  pub speed: f64,
}
