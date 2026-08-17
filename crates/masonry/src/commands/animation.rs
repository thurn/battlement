use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ObjectId;

/// Plays an Animator state with explicit scheduling time.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorPlayPayload {
    /// Target prefab game object with a supported Animator.
    pub object_id: ObjectId,
    /// Animator state name.
    #[schemars(length(max = 65_536))]
    pub state: String,
    /// Nonnegative Animator layer index.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub layer: u32,
    /// Normalized starting time in the inclusive range `[0, 1]`.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub normalized_start_time: f64,
    /// Explicit operation duration for group scheduling; zero does not wait.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(max = 86_400_000))]
    pub wait_ms: u64,
}

/// Cross-fades to an Animator state with explicit scheduling time.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorCrossFadePayload {
    /// Target prefab game object with a supported Animator.
    pub object_id: ObjectId,
    /// Animator state name.
    #[schemars(length(max = 65_536))]
    pub state: String,
    /// Nonnegative Animator layer index.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    pub layer: u32,
    /// Normalized starting time in the inclusive range `[0, 1]`.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(min = 0.0, max = 1.0))]
    pub normalized_start_time: f64,
    /// Explicit operation duration for group scheduling; zero does not wait.
    #[serde(default, skip_serializing_if = "crate::serialization::is_default")]
    #[schemars(range(max = 86_400_000))]
    pub wait_ms: u64,
    /// Positive cross-fade duration in milliseconds.
    #[schemars(range(min = 1, max = 86_400_000))]
    pub cross_fade_ms: u64,
}

/// Sets a persistent boolean Animator parameter.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorBoolPayload {
    /// Target prefab game object with a supported Animator.
    pub object_id: ObjectId,
    /// Parameter name.
    #[schemars(length(max = 65_536))]
    pub parameter: String,
    /// New boolean value.
    pub value: bool,
}

/// Sets a persistent signed 32-bit Animator parameter.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorIntPayload {
    /// Target prefab game object with a supported Animator.
    pub object_id: ObjectId,
    /// Parameter name.
    #[schemars(length(max = 65_536))]
    pub parameter: String,
    /// New signed 32-bit value.
    pub value: i32,
}

/// Sets a persistent finite floating-point Animator parameter.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorFloatPayload {
    /// Target prefab game object with a supported Animator.
    pub object_id: ObjectId,
    /// Parameter name.
    #[schemars(length(max = 65_536))]
    pub parameter: String,
    /// New finite floating-point value.
    pub value: f64,
}

/// Names an Animator parameter without an associated value.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorParameterPayload {
    /// Target prefab game object with a supported Animator.
    pub object_id: ObjectId,
    /// Parameter name.
    #[schemars(length(max = 65_536))]
    pub parameter: String,
}

/// Sets nonnegative Animator playback speed.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct AnimatorSpeedPayload {
    /// Target prefab game object with a supported Animator.
    pub object_id: ObjectId,
    /// Nonnegative playback speed.
    #[schemars(range(min = 0.0))]
    pub speed: f64,
}
