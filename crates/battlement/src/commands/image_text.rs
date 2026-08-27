use serde::{Deserialize, Serialize};

use crate::{
  HorizontalAlignment, ImageFit, ObjectId, RgbColor, TextMeshProFontAddress, TextureAddress, Tween,
  VerticalAlignment,
};

/// Replaces the prepared texture on an image object.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SetTexturePayload {
  /// Target image game object.
  pub object_id: ObjectId,
  /// Prepared texture address.
  pub address: TextureAddress,
}

/// Replaces the prepared TextMesh Pro font on a text object.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SetFontPayload {
  /// Target text game object.
  pub object_id: ObjectId,
  /// Prepared TextMesh Pro font address.
  pub address: TextMeshProFontAddress,
}

/// Resizes a Battlement image quad.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ImageSizePayload {
  /// Target image object.
  pub object_id: ObjectId,
  /// Positive world-space width.
  pub width: f64,
  /// Positive world-space height.
  pub height: f64,
}

/// Changes an image quad's texture fitting behavior.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ImageFitPayload {
  /// Target image object.
  pub object_id: ObjectId,
  /// Requested fitting mode.
  pub fit: ImageFit,
}

/// Sets an image quad's linear RGB tint.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TintPayload {
  /// Target image object.
  pub object_id: ObjectId,
  /// Requested linear RGB tint.
  pub tint: RgbColor,
}

/// Tweens an image quad's linear RGB tint.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenTintPayload {
  /// Target image object.
  pub object_id: ObjectId,
  /// Requested final linear RGB tint.
  pub tint: RgbColor,
  /// Tween timing and repetition.
  pub tween: Tween,
}

/// Sets an image quad's opacity.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct OpacityPayload {
  /// Target image object.
  pub object_id: ObjectId,
  /// Requested opacity in the inclusive range `[0, 1]`.
  pub opacity: f64,
}

/// Tweens an image quad's opacity.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenOpacityPayload {
  /// Target image object.
  pub object_id: ObjectId,
  /// Requested final opacity in the inclusive range `[0, 1]`.
  pub opacity: f64,
  /// Tween timing and repetition.
  pub tween: Tween,
}

/// Replaces displayed world-text content.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TextContentPayload {
  /// Target world-text object.
  pub object_id: ObjectId,
  /// New text content.
  pub text: String,
}

/// Sets a world-text object's positive size.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TextSizePayload {
  /// Target world-text object.
  pub object_id: ObjectId,
  /// Positive world-space text size.
  pub size: f64,
}

/// Tweens a world-text object's positive size.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TweenTextSizePayload {
  /// Target world-text object.
  pub object_id: ObjectId,
  /// Positive final world-space text size.
  pub size: f64,
  /// Tween timing and repetition.
  pub tween: Tween,
}

/// Sets horizontal and vertical world-text alignment.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TextAlignmentPayload {
  /// Target world-text object.
  pub object_id: ObjectId,
  /// Horizontal alignment.
  pub horizontal: HorizontalAlignment,
  /// Vertical alignment.
  pub vertical: VerticalAlignment,
}

/// Sets world-text wrapping width, or disables wrapping with [`None`].
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TextWrappingPayload {
  /// Target world-text object.
  pub object_id: ObjectId,
  /// Positive wrapping width; [`None`] disables wrapping.
  pub wrap_width: Option<f64>,
}
