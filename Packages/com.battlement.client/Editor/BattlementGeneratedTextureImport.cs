#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.AddressableAssets.Settings;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>One generated texture with an exact Unity import contract.</summary>
    public sealed class BattlementGeneratedTexture
    {
        public BattlementGeneratedTexture(
            string address,
            string guid,
            string path,
            string filter,
            bool mipmaps,
            string wrap,
            string compression
        )
        {
            Address = address;
            Guid = guid;
            Path = path;
            Filter = filter;
            Mipmaps = mipmaps;
            Wrap = wrap;
            Compression = compression;
        }

        public string Address { get; }
        public string Guid { get; }
        public string Path { get; }
        public string Filter { get; }
        public bool Mipmaps { get; }
        public string Wrap { get; }
        public string Compression { get; }
    }

    /// <summary>Temporarily registers validated generated textures with Addressables.</summary>
    public static class BattlementGeneratedTextureImport
    {
        private static readonly Dictionary<string, Dictionary<string, Hash128>> Validations = new(
            StringComparer.Ordinal
        );

        public static IDisposable Prepare(
            AddressableAssetSettings settings,
            string addressPrefix,
            string catalogIdentity,
            IReadOnlyCollection<BattlementGeneratedTexture> assets
        )
        {
            ValidateImportedAssets(catalogIdentity, assets);
            RejectConflicts(settings, addressPrefix, assets);
            AddressableAssetGroup? group = settings.DefaultGroup;
            if (group == null)
            {
                group = settings.groups.FirstOrDefault(value => value != null);
            }
            if (assets.Count > 0 && group == null)
            {
                throw new InvalidOperationException(
                    "Addressables has no group for generated textures."
                );
            }
            bool settingsWasDirty = EditorUtility.IsDirty(settings);
            bool groupWasDirty = group != null && EditorUtility.IsDirty(group);
            var registered = new List<string>();
            try
            {
                foreach (BattlementGeneratedTexture asset in assets)
                {
                    AddressableAssetEntry entry = settings.CreateOrMoveEntry(
                        asset.Guid,
                        group,
                        false,
                        false
                    );
                    entry.address = asset.Address;
                    registered.Add(asset.Guid);
                }
                return new Registration(
                    settings,
                    group,
                    registered.ToArray(),
                    settingsWasDirty,
                    groupWasDirty
                );
            }
            catch
            {
                RemoveTemporaryEntries(settings, registered);
                RestoreDirtyState(settings, group, settingsWasDirty, groupWasDirty);
                throw;
            }
        }

        private static void ValidateImportedAssets(
            string catalogIdentity,
            IReadOnlyCollection<BattlementGeneratedTexture> assets
        )
        {
            if (Validations.TryGetValue(catalogIdentity, out Dictionary<string, Hash128> cached))
            {
                bool current = assets.All(asset =>
                    cached.TryGetValue(asset.Guid, out Hash128 hash)
                    && hash == AssetDatabase.GetAssetDependencyHash(asset.Path)
                );
                if (current)
                {
                    return;
                }
            }

            var dependencies = new Dictionary<string, Hash128>(StringComparer.Ordinal);
            foreach (BattlementGeneratedTexture asset in assets)
            {
                if (AssetImporter.GetAtPath(asset.Path) == null && File.Exists(asset.Path))
                {
                    AssetDatabase.ImportAsset(
                        asset.Path,
                        ImportAssetOptions.ForceSynchronousImport
                    );
                }
                if (AssetDatabase.AssetPathToGUID(asset.Path) != asset.Guid)
                {
                    throw new InvalidOperationException(
                        $"Generated texture '{asset.Path}' has the wrong Unity GUID."
                    );
                }
                if (AssetDatabase.LoadAssetAtPath(asset.Path, typeof(Texture2D)) is not Texture2D)
                {
                    throw new InvalidOperationException(
                        $"Generated address '{asset.Address}' did not import as Texture2D."
                    );
                }
                ValidateImporter(asset);
                dependencies.Add(asset.Guid, AssetDatabase.GetAssetDependencyHash(asset.Path));
            }
            Validations[catalogIdentity] = dependencies;
        }

        private static void ValidateImporter(BattlementGeneratedTexture asset)
        {
            TextureImporter? importer = AssetImporter.GetAtPath(asset.Path) as TextureImporter;
            if (importer == null)
            {
                throw new InvalidOperationException(
                    $"Generated address '{asset.Address}' has no TextureImporter."
                );
            }
            FilterMode filter =
                asset.Filter == "bilinear" ? FilterMode.Bilinear
                : asset.Filter == "nearest" ? FilterMode.Point
                : asset.Filter == "trilinear" ? FilterMode.Trilinear
                : throw new InvalidOperationException(
                    $"Unknown generated filter '{asset.Filter}'."
                );
            TextureWrapMode wrap =
                asset.Wrap == "clamp" ? TextureWrapMode.Clamp
                : asset.Wrap == "repeat" ? TextureWrapMode.Repeat
                : throw new InvalidOperationException($"Unknown generated wrap '{asset.Wrap}'.");
            TextureImporterCompression compression = asset.Compression switch
            {
                "lossless" => TextureImporterCompression.Uncompressed,
                "lossyLow" => TextureImporterCompression.CompressedLQ,
                "lossyNormal" => TextureImporterCompression.Compressed,
                "lossyHigh" => TextureImporterCompression.CompressedHQ,
                _ => throw new InvalidOperationException(
                    $"Unknown generated compression '{asset.Compression}'."
                ),
            };
            if (
                importer.textureType != TextureImporterType.Default
                || !importer.sRGBTexture
                || !importer.alphaIsTransparency
                || importer.mipmapEnabled != asset.Mipmaps
                || importer.filterMode != filter
                || importer.wrapModeU != wrap
                || importer.wrapModeV != wrap
                || importer.wrapModeW != wrap
                || importer.textureCompression != compression
            )
            {
                throw new InvalidOperationException(
                    $"Generated address '{asset.Address}' has stale texture import settings."
                );
            }
        }

        private static void RejectConflicts(
            AddressableAssetSettings settings,
            string addressPrefix,
            IReadOnlyCollection<BattlementGeneratedTexture> assets
        )
        {
            var guids = assets.Select(asset => asset.Guid).ToHashSet(StringComparer.Ordinal);
            foreach (
                AddressableAssetEntry entry in settings
                    .groups.Where(group => group != null)
                    .SelectMany(group => group.entries)
            )
            {
                if (
                    entry.address.StartsWith(addressPrefix, StringComparison.Ordinal)
                    || guids.Contains(entry.guid)
                )
                {
                    throw new InvalidOperationException(
                        $"User-owned Addressables entry '{entry.address}' conflicts with generated "
                            + $"asset GUID '{entry.guid}'."
                    );
                }
            }
        }

        private static void RemoveTemporaryEntries(
            AddressableAssetSettings settings,
            IEnumerable<string> guids
        )
        {
            foreach (string guid in guids)
            {
                settings.RemoveAssetEntry(guid, false);
            }
        }

        private static void RestoreDirtyState(
            AddressableAssetSettings settings,
            AddressableAssetGroup? group,
            bool settingsWasDirty,
            bool groupWasDirty
        )
        {
            if (!settingsWasDirty)
            {
                EditorUtility.ClearDirty(settings);
            }
            if (group != null && !groupWasDirty)
            {
                EditorUtility.ClearDirty(group);
            }
        }

        private sealed class Registration : IDisposable
        {
            private readonly AddressableAssetSettings settings;
            private readonly AddressableAssetGroup? group;
            private readonly string[] guids;
            private readonly bool settingsWasDirty;
            private readonly bool groupWasDirty;
            private bool isDisposed;

            public Registration(
                AddressableAssetSettings settings,
                AddressableAssetGroup? group,
                string[] guids,
                bool settingsWasDirty,
                bool groupWasDirty
            )
            {
                this.settings = settings;
                this.group = group;
                this.guids = guids;
                this.settingsWasDirty = settingsWasDirty;
                this.groupWasDirty = groupWasDirty;
            }

            public void Dispose()
            {
                if (isDisposed)
                {
                    return;
                }
                RemoveTemporaryEntries(settings, guids);
                RestoreDirtyState(settings, group, settingsWasDirty, groupWasDirty);
                isDisposed = true;
            }
        }
    }
}
