//! Prepared Addressables assets.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The Unity asset type expected at a prepared Addressables address.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    /// An Addressable content scene.
    Scene,
    /// A prefab instantiated as a persistent runtime object.
    Prefab,
    /// A prefab used for temporary particle effects.
    ParticleEffect,
    /// A material assignable to a supported root renderer.
    Material,
    /// A texture used by an image quad.
    Texture,
    /// An audio clip played by Masonry-owned audio sources.
    AudioClip,
    /// A TextMesh Pro font asset.
    Font,
}

/// One Addressables entry loaded and type-checked before commands may use it.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(deny_unknown_fields)]
pub struct PreparedAsset {
    /// The stable, namespaced Addressables key.
    #[schemars(length(max = 65_536))]
    pub address: String,
    /// The Unity asset type expected at the address.
    pub kind: AssetKind,
}

impl PreparedAsset {
    /// Creates a prepared-asset declaration.
    #[must_use]
    pub fn new(address: impl Into<String>, kind: AssetKind) -> Self {
        Self {
            address: address.into(),
            kind,
        }
    }
}
