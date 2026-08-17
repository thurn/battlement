use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    FontAddress, HorizontalAlignment, ImageFit, ObjectId, RgbColor, TextureAddress, Tween,
    VerticalAlignment,
};

/// Replaces the prepared texture on an image object.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SetTexturePayload {
    /// Target image game object.
    pub object_id: ObjectId,
    /// Prepared texture address.
    pub address: TextureAddress,
}

/// Replaces the prepared TextMesh Pro font on a text object.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct SetFontPayload {
    /// Target text game object.
    pub object_id: ObjectId,
    /// Prepared TextMesh Pro font address.
    pub address: FontAddress,
}

/// Resizes a Masonry image quad.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ImageSizePayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Positive world-space width.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub width: f64,
    /// Positive world-space height.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub height: f64,
}

/// Changes an image quad's texture fitting behavior.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ImageFitPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested fitting mode.
    pub fit: ImageFit,
}

/// Sets an image quad's linear RGB tint.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TintPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested linear RGB tint.
    pub tint: RgbColor,
}

/// Tweens an image quad's linear RGB tint.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenTintPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested final linear RGB tint.
    pub tint: RgbColor,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets an image quad's opacity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct OpacityPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested opacity in the inclusive range `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub opacity: f64,
}

/// Tweens an image quad's opacity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenOpacityPayload {
    /// Target image object.
    pub object_id: ObjectId,
    /// Requested final opacity in the inclusive range `[0, 1]`.
    #[schemars(range(min = 0.0, max = 1.0))]
    pub opacity: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Replaces displayed world-text content.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TextContentPayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// New text content.
    #[schemars(length(max = 65_536))]
    pub text: String,
}

/// Sets a world-text object's positive size.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TextSizePayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// Positive world-space text size.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
}

/// Tweens a world-text object's positive size.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TweenTextSizePayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// Positive final world-space text size.
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub size: f64,
    /// Tween timing and repetition.
    #[serde(flatten)]
    pub tween: Tween,
}

/// Sets horizontal and vertical world-text alignment.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TextAlignmentPayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// Horizontal alignment.
    pub horizontal: HorizontalAlignment,
    /// Vertical alignment.
    pub vertical: VerticalAlignment,
}

/// Enables or disables world-text wrapping.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct TextWrappingPayload {
    /// Target world-text object.
    pub object_id: ObjectId,
    /// Whether wrapping is enabled.
    pub enabled: bool,
    /// Positive width required when wrapping is enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 0.0))]
    #[schemars(extend("exclusiveMinimum" = 0.0))]
    pub wrap_width: Option<f64>,
}
