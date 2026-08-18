//! Prepared Addressables assets.

use std::{fmt, marker::PhantomData};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An Addressables key tagged with the Unity asset type it must resolve to.
///
/// Public APIs use role-specific aliases such as [`SceneAddress`] and
/// [`TextureAddress`], so an address prepared for one Unity asset type cannot
/// be passed where another is required.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetAddress<K> {
    value: String,
    kind: PhantomData<fn() -> K>,
}

impl<K> AssetAddress<K> {
    /// Creates a typed address from its stable, namespaced Addressables key.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            kind: PhantomData,
        }
    }

    /// Returns the Addressables key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Consumes the typed address and returns its Addressables key.
    #[must_use]
    pub fn into_string(self) -> String {
        self.value
    }
}

impl<K> fmt::Display for AssetAddress<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.value)
    }
}

impl<K> From<&str> for AssetAddress<K> {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl<K> From<String> for AssetAddress<K> {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl<K> AsRef<str> for AssetAddress<K> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<K> Serialize for AssetAddress<K> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de, K> Deserialize<'de> for AssetAddress<K> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::new)
    }
}

mod kind {
    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Scene;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Prefab;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct ParticleEffect;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Material;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Texture;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct AudioClip;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Font;
}

/// An Addressable content-scene key.
pub type SceneAddress = AssetAddress<kind::Scene>;
/// An Addressable prefab key.
pub type PrefabAddress = AssetAddress<kind::Prefab>;
/// An Addressable particle-effect-prefab key.
pub type ParticleEffectAddress = AssetAddress<kind::ParticleEffect>;
/// An Addressable material key.
pub type MaterialAddress = AssetAddress<kind::Material>;
/// An Addressable texture key.
pub type TextureAddress = AssetAddress<kind::Texture>;
/// An Addressable audio-clip key.
pub type AudioClipAddress = AssetAddress<kind::AudioClip>;
/// An Addressable TextMesh Pro font key.
pub type FontAddress = AssetAddress<kind::Font>;

/// One Addressables entry loaded and type-checked before commands may use it.
///
/// The union couples every address with its expected Unity asset type, so a
/// producer cannot construct a declaration whose kind disagrees with its
/// address type.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PreparedAsset {
    /// An Addressable content scene.
    Scene(SceneAddress),
    /// A prefab instantiated as a persistent game object.
    Prefab(PrefabAddress),
    /// A prefab used for temporary particle effects.
    ParticleEffect(ParticleEffectAddress),
    /// A material assignable to a supported renderer.
    Material(MaterialAddress),
    /// A texture used by an image quad.
    Texture(TextureAddress),
    /// An audio clip played by Masonry-owned audio sources.
    AudioClip(AudioClipAddress),
    /// A TextMesh Pro font asset.
    Font(FontAddress),
}
