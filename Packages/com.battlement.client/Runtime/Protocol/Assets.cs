#nullable enable

namespace Battlement
{
    /// <summary>
    /// One Addressables entry loaded and type-checked before commands may use it.
    /// </summary>
    /// <remarks>
    /// The union couples every address with its expected asset type, preventing
    /// declarations whose kind disagrees with their address type.
    /// </remarks>
    public abstract record PreparedAsset
    {
        private PreparedAsset() { }

        /// <summary>An Addressable content scene.</summary>
        public sealed record Scene(SceneAddress Address) : PreparedAsset;

        /// <summary>A prefab instantiated as a persistent game object.</summary>
        public sealed record Prefab(PrefabAddress Address) : PreparedAsset;

        /// <summary>A prefab used for temporary particle effects.</summary>
        public sealed record ParticleEffect(ParticleEffectAddress Address) : PreparedAsset;

        /// <summary>A material assignable to a supported renderer.</summary>
        public sealed record Material(MaterialAddress Address) : PreparedAsset;

        /// <summary>A texture used by an image quad.</summary>
        public sealed record Texture(TextureAddress Address) : PreparedAsset;

        /// <summary>A sprite used by UI Toolkit images and backgrounds.</summary>
        public sealed record Sprite(SpriteAddress Address) : PreparedAsset;

        /// <summary>A vector graphic used by UI Toolkit images and backgrounds.</summary>
        public sealed record VectorImage(VectorImageAddress Address) : PreparedAsset;

        /// <summary>A render texture used by UI Toolkit images and panel targets.</summary>
        public sealed record RenderTexture(RenderTextureAddress Address) : PreparedAsset;

        /// <summary>An audio clip played by Battlement-owned audio sources.</summary>
        public sealed record AudioClip(AudioClipAddress Address) : PreparedAsset;

        /// <summary>A TextMesh Pro font asset.</summary>
        public sealed record TextMeshProFont(TextMeshProFontAddress Address) : PreparedAsset;

        /// <summary>A UI Toolkit-compatible TextCore font asset.</summary>
        public sealed record UiFont(UiFontAddress Address) : PreparedAsset;
    }
}
