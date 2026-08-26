#nullable enable

using System;
using System.Collections.Generic;
using System.Text;

namespace Battlement
{
    internal static class BattlementSnapshotCatalogValidator
    {
        private const int MaximumAssets = 16_384;
        private const int MaximumScenes = 32;
        private const int MaximumStringBytes = 65_536;

        public static Dictionary<string, PreparedAsset> ValidatePrepared(
            IReadOnlyList<PreparedAsset> assets
        )
        {
            Preconditions.CheckNotNull(assets, nameof(assets));
            if (assets.Count > MaximumAssets)
            {
                throw Invalid(
                    CoreErrorCode.LimitExceeded,
                    $"A snapshot cannot prepare more than {MaximumAssets} assets."
                );
            }

            var result = new Dictionary<string, PreparedAsset>(
                assets.Count,
                StringComparer.Ordinal
            );
            foreach (PreparedAsset asset in assets)
            {
                string address = AssetAddress(asset);
                RequireString(address, "Prepared asset address");
                if (!result.TryAdd(address, asset))
                {
                    throw Invalid(
                        CoreErrorCode.DuplicateId,
                        $"Prepared asset address '{address}' appeared more than once."
                    );
                }
            }

            return result;
        }

        public static (Guid Primary, HashSet<Guid> Ids) ValidateScenes(
            IReadOnlyList<BattlementScene> scenes,
            SceneId? primarySceneId,
            IReadOnlyDictionary<string, PreparedAsset> prepared
        )
        {
            Preconditions.CheckNotNull(scenes, nameof(scenes));
            if (scenes.Count is 0 or > MaximumScenes)
            {
                throw Invalid(
                    scenes.Count == 0 ? CoreErrorCode.UnknownScene : CoreErrorCode.LimitExceeded,
                    $"A snapshot must contain between 1 and {MaximumScenes} content scenes."
                );
            }

            var ids = new HashSet<Guid>();
            var addresses = new HashSet<string>(StringComparer.Ordinal);
            foreach (BattlementScene scene in scenes)
            {
                Guid id = RequireId(scene.Id.Value, "scene");
                string address = scene.Address.Value;
                RequireString(address, "Scene address");
                if (!ids.Add(id) || !addresses.Add(address))
                {
                    throw Invalid(
                        CoreErrorCode.DuplicateId,
                        "Scene UUIDs and addresses must be unique within a snapshot."
                    );
                }
                if (
                    !prepared.TryGetValue(address, out PreparedAsset asset)
                    || asset is not PreparedAsset.Scene
                )
                {
                    throw Invalid(
                        CoreErrorCode.AssetNotPrepared,
                        $"The scene address '{address}' was not in the prepared set "
                            + "with the required type."
                    );
                }
            }

            Guid primary =
                primarySceneId?.Value ?? (scenes.Count == 1 ? scenes[0].Id.Value : default);
            if (primary == Guid.Empty || !ids.Contains(primary))
            {
                throw Invalid(
                    CoreErrorCode.UnknownScene,
                    "The primary scene must name a scene in the snapshot."
                );
            }

            return (primary, ids);
        }

        private static string AssetAddress(PreparedAsset asset) =>
            Preconditions.CheckNotNull(asset, nameof(asset)) switch
            {
                PreparedAsset.Scene value => value.Address.Value,
                PreparedAsset.Prefab value => value.Address.Value,
                PreparedAsset.ParticleEffect value => value.Address.Value,
                PreparedAsset.Material value => value.Address.Value,
                PreparedAsset.Texture value => value.Address.Value,
                PreparedAsset.Sprite value => value.Address.Value,
                PreparedAsset.VectorImage value => value.Address.Value,
                PreparedAsset.RenderTexture value => value.Address.Value,
                PreparedAsset.AudioClip value => value.Address.Value,
                PreparedAsset.TextMeshProFont value => value.Address.Value,
                PreparedAsset.UiFont value => value.Address.Value,
                _ => throw Invalid(CoreErrorCode.UnknownAsset, "Unknown prepared asset kind."),
            };

        private static Guid RequireId(Guid value, string name) =>
            value != Guid.Empty
                ? value
                : throw Invalid(CoreErrorCode.InvalidProperty, $"The {name} UUID must be nonzero.");

        private static void RequireString(string? value, string name)
        {
            if (string.IsNullOrEmpty(value))
                throw Invalid(CoreErrorCode.InvalidProperty, $"{name} cannot be empty.");
            if (Encoding.UTF8.GetByteCount(value) > MaximumStringBytes)
            {
                throw Invalid(
                    CoreErrorCode.LimitExceeded,
                    $"{name} exceeds {MaximumStringBytes} UTF-8 bytes."
                );
            }
        }

        private static BattlementWorldException Invalid(CoreErrorCode code, string message) =>
            new(code, message);
    }
}
