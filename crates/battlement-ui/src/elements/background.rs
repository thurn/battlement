use battlement_types::{RenderTextureAddress, SpriteAddress, TextureAddress, VectorImageAddress};
use serde::{Deserialize, Serialize};

/// One prepared graphical asset painted behind an element's content.
///
/// Unlike [`ImageSource`](crate::ImageSource), this source participates in
/// background styling, including tinting and nine-slice rendering. The asset
/// must be present in the snapshot's prepared set with the matching type.
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum BackgroundSource {
    /// A prepared raster texture.
    Texture(TextureAddress),
    /// A prepared sprite retaining its imported geometry and border metadata.
    Sprite(SpriteAddress),
    /// A prepared resolution-independent UI Toolkit vector image.
    VectorImage(VectorImageAddress),
    /// A prepared live render target.
    RenderTexture(RenderTextureAddress),
}

impl From<TextureAddress> for BackgroundSource {
    fn from(value: TextureAddress) -> Self {
        Self::Texture(value)
    }
}

impl From<SpriteAddress> for BackgroundSource {
    fn from(value: SpriteAddress) -> Self {
        Self::Sprite(value)
    }
}

impl From<VectorImageAddress> for BackgroundSource {
    fn from(value: VectorImageAddress) -> Self {
        Self::VectorImage(value)
    }
}

impl From<RenderTextureAddress> for BackgroundSource {
    fn from(value: RenderTextureAddress) -> Self {
        Self::RenderTexture(value)
    }
}
