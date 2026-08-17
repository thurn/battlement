use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{CameraClearMode, Color, LightType, ObjectId, ShadowMode, Tween};

/// Switches a camera to perspective projection.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct PerspectivePayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Vertical field of view in degrees, strictly between 1 and 179.
    #[schemars(range(min = 1.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 1.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub field_of_view: f64,
}

/// Tweens a perspective camera's vertical field of view.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenFieldOfViewPayload {
    /// Target perspective camera object.
    pub object_id: ObjectId,
    /// Final vertical field of view in degrees, strictly between 1 and 179.
    #[schemars(range(min = 1.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 1.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub field_of_view: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Switches a camera to orthographic projection.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OrthographicPayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Positive orthographic half-height.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
}

/// Tweens an orthographic camera's size.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenOrthographicSizePayload {
    /// Target orthographic camera object.
    pub object_id: ObjectId,
    /// Positive final orthographic half-height.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets a camera's clipping distances.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CameraClippingPayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Positive near clipping distance.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub near: f64,
    /// Far clipping distance, which must be greater than `near`.
    #[schemars(range(min = 0.0))]
    pub far: f64,
}

/// Sets a camera's clear behavior.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct CameraClearPayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Requested clear mode.
    pub clear_mode: CameraClearMode,
    /// Required for `solidColor`; otherwise omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_color: Option<Color>,
}

/// Changes a standard light's type.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LightTypePayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested standard light type.
    pub light_type: LightType,
}

/// Sets a light or text object's linear RGBA color.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ColorPayload {
    /// Target light or world-text object.
    pub object_id: ObjectId,
    /// Requested linear color.
    pub color: Color,
}

/// Tweens a light or text object's linear RGBA color.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenColorPayload {
    /// Target light or world-text object.
    pub object_id: ObjectId,
    /// Requested final linear color.
    pub color: Color,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets a light's nonnegative intensity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct IntensityPayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested nonnegative intensity.
    #[schemars(range(min = 0.0))]
    pub intensity: f64,
}

/// Tweens a light's nonnegative intensity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenIntensityPayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested final nonnegative intensity.
    #[schemars(range(min = 0.0))]
    pub intensity: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets the positive range of a point or spot light.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LightRangePayload {
    /// Target point or spot light object.
    pub object_id: ObjectId,
    /// Positive range in world units.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub range: f64,
}

/// Sets a spot light's cone angles.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SpotAnglePayload {
    /// Target spot light object.
    pub object_id: ObjectId,
    /// Outer angle in degrees, strictly between zero and 179.
    #[schemars(range(min = 0.0, max = 179.0))]
    #[schemars(
        extend("exclusiveMinimum" = 0.0),
        extend("exclusiveMaximum" = 179.0)
    )]
    pub outer_spot_angle: f64,
    /// Inner angle in `[0, outer_spot_angle]`.
    #[schemars(range(min = 0.0, max = 179.0))]
    pub inner_spot_angle: f64,
}

/// Sets a standard light's shadow mode.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LightShadowsPayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested shadow mode.
    pub shadows: ShadowMode,
}
