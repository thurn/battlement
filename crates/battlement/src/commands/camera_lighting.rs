use serde::{Deserialize, Serialize};

use crate::{CameraClearMode, Color, LightType, ObjectId, ShadowMode, Tween};

/// Switches a camera to perspective projection.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct PerspectivePayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Vertical field of view in degrees, strictly between 1 and 179.
    pub field_of_view: f64,
}

/// Tweens a perspective camera's vertical field of view.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenFieldOfViewPayload {
    /// Target perspective camera object.
    pub object_id: ObjectId,
    /// Final vertical field of view in degrees, strictly between 1 and 179.
    pub field_of_view: f64,
    /// Tween timing and repetition.
    pub tween: Tween,
}

/// Switches a camera to orthographic projection.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct OrthographicPayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Positive orthographic half-height.
    pub size: f64,
}

/// Tweens an orthographic camera's size.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenOrthographicSizePayload {
    /// Target orthographic camera object.
    pub object_id: ObjectId,
    /// Positive final orthographic half-height.
    pub size: f64,
    /// Tween timing and repetition.
    pub tween: Tween,
}

/// Sets a camera's clipping distances.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CameraClippingPayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Positive near clipping distance.
    pub near: f64,
    /// Far clipping distance, which must be greater than `near`.
    pub far: f64,
}

/// Sets a camera's clear behavior.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct CameraClearPayload {
    /// Target camera object.
    pub object_id: ObjectId,
    /// Requested clear mode.
    pub clear_mode: CameraClearMode,
    /// Present for [`CameraClearMode::SolidColor`] and absent otherwise.
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub clear_color: Option<Color>,
}

/// Changes a standard light's type.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LightTypePayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested standard light type.
    pub light_type: LightType,
}

/// Sets a light or text object's linear RGBA color.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ColorPayload {
    /// Target light or world-text object.
    pub object_id: ObjectId,
    /// Requested linear color.
    pub color: Color,
}

/// Tweens a light or text object's linear RGBA color.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenColorPayload {
    /// Target light or world-text object.
    pub object_id: ObjectId,
    /// Requested final linear color.
    pub color: Color,
    /// Tween timing and repetition.
    pub tween: Tween,
}

/// Sets a light's nonnegative intensity.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct IntensityPayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested nonnegative intensity.
    pub intensity: f64,
}

/// Tweens a light's nonnegative intensity.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenIntensityPayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested final nonnegative intensity.
    pub intensity: f64,
    /// Tween timing and repetition.
    pub tween: Tween,
}

/// Sets the positive range of a point or spot light.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LightRangePayload {
    /// Target point or spot light object.
    pub object_id: ObjectId,
    /// Positive range in world units.
    pub range: f64,
}

/// Sets a spot light's cone angles.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct SpotAnglePayload {
    /// Target spot light object.
    pub object_id: ObjectId,
    /// Outer angle in degrees, strictly between zero and 179.
    pub outer_spot_angle: f64,
    /// Inner angle in `[0, outer_spot_angle]`.
    pub inner_spot_angle: f64,
}

/// Sets a standard light's shadow mode.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LightShadowsPayload {
    /// Target light object.
    pub object_id: ObjectId,
    /// Requested shadow mode.
    pub shadows: ShadowMode,
}
