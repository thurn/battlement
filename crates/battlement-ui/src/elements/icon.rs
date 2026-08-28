use battlement_types::{RenderTextureAddress, SpriteAddress, TextureAddress, VectorImageAddress};
use serde::{Deserialize, Serialize};

use crate::Prop;

/// One prepared graphical asset displayed by a control's native icon slot.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum IconSource {
  /// A raster `Texture2D` icon.
  Texture(TextureAddress),
  /// A sprite icon retaining imported geometry.
  Sprite(SpriteAddress),
  /// A resolution-independent UI Toolkit vector icon.
  VectorImage(VectorImageAddress),
  /// A live render target used as an icon.
  RenderTexture(RenderTextureAddress),
}

impl IconSource {
  /// Returns the Addressables key held by this source.
  #[must_use]
  pub fn address(&self) -> &str {
    match self {
      Self::Texture(value) => value.as_str(),
      Self::Sprite(value) => value.as_str(),
      Self::VectorImage(value) => value.as_str(),
      Self::RenderTexture(value) => value.as_str(),
    }
  }
}

impl From<TextureAddress> for IconSource {
  fn from(value: TextureAddress) -> Self {
    Self::Texture(value)
  }
}

impl From<SpriteAddress> for IconSource {
  fn from(value: SpriteAddress) -> Self {
    Self::Sprite(value)
  }
}

impl From<VectorImageAddress> for IconSource {
  fn from(value: VectorImageAddress) -> Self {
    Self::VectorImage(value)
  }
}

impl From<RenderTextureAddress> for IconSource {
  fn from(value: RenderTextureAddress) -> Self {
    Self::RenderTexture(value)
  }
}

impl From<TextureAddress> for Prop<IconSource> {
  fn from(value: TextureAddress) -> Self {
    Self::Set(value.into())
  }
}

impl From<SpriteAddress> for Prop<IconSource> {
  fn from(value: SpriteAddress) -> Self {
    Self::Set(value.into())
  }
}

impl From<VectorImageAddress> for Prop<IconSource> {
  fn from(value: VectorImageAddress) -> Self {
    Self::Set(value.into())
  }
}

impl From<RenderTextureAddress> for Prop<IconSource> {
  fn from(value: RenderTextureAddress) -> Self {
    Self::Set(value.into())
  }
}
