#nullable enable

using System;

namespace Masonry
{
    /// <summary>An Addressable content-scene key.</summary>
    public readonly struct SceneAddress : IEquatable<SceneAddress>
    {
        /// <summary>Creates a typed address from its stable, namespaced key.</summary>
        public SceneAddress(string value) => Value = value;

        /// <summary>Gets the Addressables key.</summary>
        public string Value { get; }

        public bool Equals(SceneAddress other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is SceneAddress other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value;

        public static bool operator ==(SceneAddress left, SceneAddress right) => left.Equals(right);

        public static bool operator !=(SceneAddress left, SceneAddress right) =>
            !left.Equals(right);
    }

    /// <summary>An Addressable persistent-prefab key.</summary>
    public readonly struct PrefabAddress : IEquatable<PrefabAddress>
    {
        /// <summary>Creates a typed address from its stable, namespaced key.</summary>
        public PrefabAddress(string value) => Value = value;

        /// <summary>Gets the Addressables key.</summary>
        public string Value { get; }

        public bool Equals(PrefabAddress other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is PrefabAddress other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value;

        public static bool operator ==(PrefabAddress left, PrefabAddress right) =>
            left.Equals(right);

        public static bool operator !=(PrefabAddress left, PrefabAddress right) =>
            !left.Equals(right);
    }

    /// <summary>An Addressable particle-effect-prefab key.</summary>
    public readonly struct ParticleEffectAddress : IEquatable<ParticleEffectAddress>
    {
        /// <summary>Creates a typed address from its stable, namespaced key.</summary>
        public ParticleEffectAddress(string value) => Value = value;

        /// <summary>Gets the Addressables key.</summary>
        public string Value { get; }

        public bool Equals(ParticleEffectAddress other) => Value == other.Value;

        public override bool Equals(object? obj) =>
            obj is ParticleEffectAddress other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value;

        public static bool operator ==(ParticleEffectAddress left, ParticleEffectAddress right) =>
            left.Equals(right);

        public static bool operator !=(ParticleEffectAddress left, ParticleEffectAddress right) =>
            !left.Equals(right);
    }

    /// <summary>An Addressable material key.</summary>
    public readonly struct MaterialAddress : IEquatable<MaterialAddress>
    {
        /// <summary>Creates a typed address from its stable, namespaced key.</summary>
        public MaterialAddress(string value) => Value = value;

        /// <summary>Gets the Addressables key.</summary>
        public string Value { get; }

        public bool Equals(MaterialAddress other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is MaterialAddress other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value;

        public static bool operator ==(MaterialAddress left, MaterialAddress right) =>
            left.Equals(right);

        public static bool operator !=(MaterialAddress left, MaterialAddress right) =>
            !left.Equals(right);
    }

    /// <summary>An Addressable image-texture key.</summary>
    public readonly struct TextureAddress : IEquatable<TextureAddress>
    {
        /// <summary>Creates a typed address from its stable, namespaced key.</summary>
        public TextureAddress(string value) => Value = value;

        /// <summary>Gets the Addressables key.</summary>
        public string Value { get; }

        public bool Equals(TextureAddress other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is TextureAddress other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value;

        public static bool operator ==(TextureAddress left, TextureAddress right) =>
            left.Equals(right);

        public static bool operator !=(TextureAddress left, TextureAddress right) =>
            !left.Equals(right);
    }

    /// <summary>An Addressable audio-clip key.</summary>
    public readonly struct AudioClipAddress : IEquatable<AudioClipAddress>
    {
        /// <summary>Creates a typed address from its stable, namespaced key.</summary>
        public AudioClipAddress(string value) => Value = value;

        /// <summary>Gets the Addressables key.</summary>
        public string Value { get; }

        public bool Equals(AudioClipAddress other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is AudioClipAddress other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value;

        public static bool operator ==(AudioClipAddress left, AudioClipAddress right) =>
            left.Equals(right);

        public static bool operator !=(AudioClipAddress left, AudioClipAddress right) =>
            !left.Equals(right);
    }

    /// <summary>An Addressable TextMesh Pro font key.</summary>
    public readonly struct FontAddress : IEquatable<FontAddress>
    {
        /// <summary>Creates a typed address from its stable, namespaced key.</summary>
        public FontAddress(string value) => Value = value;

        /// <summary>Gets the Addressables key.</summary>
        public string Value { get; }

        public bool Equals(FontAddress other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is FontAddress other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();

        public override string ToString() => Value;

        public static bool operator ==(FontAddress left, FontAddress right) => left.Equals(right);

        public static bool operator !=(FontAddress left, FontAddress right) => !left.Equals(right);
    }
}
