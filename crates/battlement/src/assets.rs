//! Prepared Addressables declarations owned by the core protocol.

use serde::{Deserialize, Serialize};

pub use battlement_types::{
    AssetAddress, AudioClipAddress, FontAddress, MaterialAddress, PrefabAddress,
    RenderTextureAddress, SceneAddress, SpriteAddress, TextureAddress, UiFontAddress,
    UnityFontAddress, UntypedAssetAddress, VectorImageAddress,
};

/// One Addressables entry loaded and type-checked before commands may use it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PreparedAsset {
    /// An Addressable content scene.
    Scene(SceneAddress),
    /// A prefab instantiated as a persistent game object.
    Prefab(PrefabAddress),
    /// A prefab used for temporary particle effects.
    ParticleEffect(PrefabAddress),
    /// A material assignable to a supported renderer.
    Material(MaterialAddress),
    /// A texture used by an image quad.
    Texture(TextureAddress),
    /// A sprite used by UI Toolkit images and backgrounds.
    Sprite(SpriteAddress),
    /// A vector image used by UI Toolkit images and backgrounds.
    VectorImage(VectorImageAddress),
    /// A render texture used by UI Toolkit images and panel targets.
    RenderTexture(RenderTextureAddress),
    /// An audio clip played by Battlement-owned audio sources.
    AudioClip(AudioClipAddress),
    /// A TextMesh Pro font asset.
    Font(FontAddress),
    /// A UI Toolkit-compatible TextCore font asset.
    UiFont(UiFontAddress),
    /// A legacy Unity font used by UI Toolkit's `unity-font` style.
    UnityFont(UnityFontAddress),
}

impl PreparedAsset {
    /// Creates a prepared content-scene declaration.
    #[must_use]
    pub fn scene(address: impl Into<SceneAddress>) -> Self {
        Self::Scene(address.into())
    }

    /// Creates a prepared prefab declaration.
    #[must_use]
    pub fn prefab(address: impl Into<PrefabAddress>) -> Self {
        Self::Prefab(address.into())
    }

    /// Creates a prepared particle-effect declaration.
    #[must_use]
    pub fn particle_effect(address: impl Into<PrefabAddress>) -> Self {
        Self::ParticleEffect(address.into())
    }

    /// Creates a prepared material declaration.
    #[must_use]
    pub fn material(address: impl Into<MaterialAddress>) -> Self {
        Self::Material(address.into())
    }

    /// Creates a prepared texture declaration.
    #[must_use]
    pub fn texture(address: impl Into<TextureAddress>) -> Self {
        Self::Texture(address.into())
    }

    /// Creates a prepared sprite declaration.
    #[must_use]
    pub fn sprite(address: impl Into<SpriteAddress>) -> Self {
        Self::Sprite(address.into())
    }

    /// Creates a prepared vector-image declaration.
    #[must_use]
    pub fn vector_image(address: impl Into<VectorImageAddress>) -> Self {
        Self::VectorImage(address.into())
    }

    /// Creates a prepared render-texture declaration.
    #[must_use]
    pub fn render_texture(address: impl Into<RenderTextureAddress>) -> Self {
        Self::RenderTexture(address.into())
    }

    /// Creates a prepared audio-clip declaration.
    #[must_use]
    pub fn audio_clip(address: impl Into<AudioClipAddress>) -> Self {
        Self::AudioClip(address.into())
    }

    /// Creates a prepared font declaration.
    #[must_use]
    pub fn font(address: impl Into<FontAddress>) -> Self {
        Self::Font(address.into())
    }

    /// Creates a prepared UI Toolkit font declaration.
    #[must_use]
    pub fn ui_font(address: impl Into<UiFontAddress>) -> Self {
        Self::UiFont(address.into())
    }

    /// Creates a prepared legacy Unity font declaration.
    #[must_use]
    pub fn unity_font(address: impl Into<UnityFontAddress>) -> Self {
        Self::UnityFont(address.into())
    }
}
