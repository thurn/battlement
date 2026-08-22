//! Prepared Addressables assets.

use std::{borrow::Cow, fmt, marker::PhantomData};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An Addressables key tagged with the Unity asset type it must resolve to.
///
/// Public APIs use role-specific aliases such as [`SceneAddress`] and
/// [`TextureAddress`], so an address prepared for one Unity asset type cannot
/// be passed where another is required.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetAddress<K> {
    value: Cow<'static, str>,
    kind: PhantomData<fn() -> K>,
}

impl<K> AssetAddress<K> {
    /// Creates a typed address from its stable, namespaced Addressables key.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: Cow::Owned(value.into()),
            kind: PhantomData,
        }
    }

    /// Creates a typed address backed directly by a static string.
    #[must_use]
    pub const fn from_static(value: &'static str) -> Self {
        Self {
            value: Cow::Borrowed(value),
            kind: PhantomData,
        }
    }

    /// Returns the Addressables key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.value.as_ref()
    }

    /// Consumes the typed address and returns its Addressables key.
    #[must_use]
    pub fn into_string(self) -> String {
        self.value.into_owned()
    }
}

impl<K> fmt::Display for AssetAddress<K> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
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
    pub struct Material;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Texture;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct AudioClip;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Font;

    #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
    pub struct Untyped;
}

/// An Addressable content-scene key.
pub type SceneAddress = AssetAddress<kind::Scene>;
/// An Addressable prefab key.
pub type PrefabAddress = AssetAddress<kind::Prefab>;
/// An Addressable material key.
pub type MaterialAddress = AssetAddress<kind::Material>;
/// An Addressable texture key.
pub type TextureAddress = AssetAddress<kind::Texture>;
/// An Addressable audio-clip key.
pub type AudioClipAddress = AssetAddress<kind::AudioClip>;
/// An Addressable TextMesh Pro font key.
pub type FontAddress = AssetAddress<kind::Font>;
/// An Addressables key for a Unity asset type Masonry does not model directly.
pub type UntypedAssetAddress = AssetAddress<kind::Untyped>;

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
    ParticleEffect(PrefabAddress),
    /// A material assignable to a supported renderer.
    Material(MaterialAddress),
    /// A texture used by an image quad.
    Texture(TextureAddress),
    /// An audio clip played by Masonry-owned audio sources.
    AudioClip(AudioClipAddress),
    /// A TextMesh Pro font asset.
    Font(FontAddress),
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
}
