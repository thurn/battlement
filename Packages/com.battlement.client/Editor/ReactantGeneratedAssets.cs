#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using UnityEditor;
using UnityEditor.AddressableAssets.Settings;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>Temporarily registers a validated generated texture catalog.</summary>
    internal sealed class ReactantGeneratedAssets : IDisposable
    {
        private const string AddressPrefix = "battlement-reactant/generated/";
        private const string GeneratedRoot = "Assets/Generated/BattlementReactant";
        private const string ManifestPath = GeneratedRoot + "/manifest.json";
        private const string SidecarPath =
            GeneratedRoot + "/Resources/BattlementReactantAssetCatalog.json";

        private static readonly Dictionary<string, Dictionary<string, Hash128>> Validations = new(
            StringComparer.Ordinal
        );

        private readonly AddressableAssetSettings? settings;
        private readonly AddressableAssetGroup? group;
        private readonly string[] guids;
        private readonly bool settingsWasDirty;
        private readonly bool groupWasDirty;
        private bool isDisposed;

        private ReactantGeneratedAssets(
            AddressableAssetSettings? settings,
            AddressableAssetGroup? group,
            string[] guids,
            bool settingsWasDirty = false,
            bool groupWasDirty = false
        )
        {
            this.settings = settings;
            this.group = group;
            this.guids = guids;
            this.settingsWasDirty = settingsWasDirty;
            this.groupWasDirty = groupWasDirty;
        }

        internal bool HasEntries => guids.Length > 0;

        internal static ReactantGeneratedAssets Prepare(AddressableAssetSettings settings)
        {
            if (!File.Exists(ManifestPath))
            {
                if (File.Exists(SidecarPath))
                {
                    throw new InvalidOperationException(
                        "The Reactant runtime sidecar exists without its generated manifest."
                    );
                }

                return new ReactantGeneratedAssets(null, null, Array.Empty<string>());
            }
            if (!File.Exists(SidecarPath))
            {
                throw new InvalidOperationException(
                    "The generated Reactant manifest has no runtime sidecar."
                );
            }

            Catalog catalog = ReadCatalog();
            ValidateImportedAssets(catalog);
            RejectConflicts(settings, catalog.Assets);
            AddressableAssetGroup? group = settings.DefaultGroup;
            if (group == null)
            {
                group = settings.groups.FirstOrDefault(value => value != null);
            }
            if (catalog.Assets.Length > 0 && group == null)
            {
                throw new InvalidOperationException(
                    "Addressables has no group for generated Reactant textures."
                );
            }
            bool settingsWasDirty = EditorUtility.IsDirty(settings);
            bool groupWasDirty = group != null && EditorUtility.IsDirty(group);
            var registered = new List<string>();
            try
            {
                foreach (AssetRecord asset in catalog.Assets)
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

                return new ReactantGeneratedAssets(
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

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            if (settings != null)
            {
                RemoveTemporaryEntries(settings, guids);
                RestoreDirtyState(settings, group, settingsWasDirty, groupWasDirty);
            }
            isDisposed = true;
        }

        private static Catalog ReadCatalog()
        {
            byte[] manifestBytes = File.ReadAllBytes(ManifestPath);
            JObject manifest = Parse(manifestBytes, "generated manifest");
            RequireFields(manifest, "assets", "browser", "rendererIdentity");
            RequireFields(
                RequireObject(manifest, "browser"),
                "executableFileIdentity",
                "executablePath",
                "executableSha256",
                "product",
                "version"
            );
            RequireFields(
                RequireObject(RequireObject(manifest, "browser"), "executableFileIdentity"),
                "byteLength",
                "fileId",
                "modifiedNanoseconds"
            );
            JArray assets =
                manifest["assets"] as JArray
                ?? throw new InvalidOperationException(
                    "Generated manifest assets must be an array."
                );
            AssetRecord[] records = assets.Select(ReadAsset).ToArray();
            string[] addresses = records.Select(asset => asset.Address).ToArray();
            RequireSortedUnique(addresses, "Generated manifest assets");

            JObject sidecar = Parse(File.ReadAllBytes(SidecarPath), "runtime sidecar");
            RequireFields(sidecar, "addresses", "manifestSha256");
            string[] sidecarAddresses =
                sidecar["addresses"]?.ToObject<string[]>()
                ?? throw new InvalidOperationException(
                    "Runtime sidecar addresses must be an array."
                );
            RequireSortedUnique(sidecarAddresses, "Runtime sidecar addresses");
            string manifestHash = Hex(SHA256.Create().ComputeHash(manifestBytes));
            if (!addresses.SequenceEqual(sidecarAddresses, StringComparer.Ordinal))
            {
                throw new InvalidOperationException(
                    "The runtime sidecar address set does not match the generated manifest."
                );
            }
            if (sidecar["manifestSha256"]?.Value<string>() != manifestHash)
            {
                throw new InvalidOperationException(
                    "The runtime sidecar hash does not match the generated manifest."
                );
            }

            return new Catalog(manifestHash, records);
        }

        private static AssetRecord ReadAsset(JToken token)
        {
            if (token is not JObject asset)
            {
                throw new InvalidOperationException("Generated manifest assets must be objects.");
            }
            RequireFields(
                asset,
                "address",
                "cacheKey",
                "canonicalRequestSha256",
                "dependencies",
                "import",
                "kind",
                "logicalCanvas",
                "png",
                "pngSha256",
                "rasterScale",
                "rasterSize",
                "sliceInsets",
                "subjectBounds",
                "unityGuid",
                "unityGuidDerivationSha256"
            );
            RequireFields(
                RequireObject(asset, "import"),
                "alphaIsTransparency",
                "compression",
                "filterMode",
                "mipmaps",
                "sRgb",
                "textureType",
                "wrapMode"
            );
            RequireFields(RequireObject(asset, "logicalCanvas"), "height", "width");
            RequireFields(RequireObject(asset, "rasterSize"), "height", "width");
            RequireFields(RequireObject(asset, "subjectBounds"), "height", "width", "x", "y");
            if (asset["sliceInsets"]?.Type != JTokenType.Null)
            {
                RequireFields(
                    RequireObject(asset, "sliceInsets"),
                    "bottom",
                    "left",
                    "right",
                    "top"
                );
            }
            if (asset["dependencies"] is not JArray dependencies)
            {
                throw new InvalidOperationException("Asset dependencies must be an array.");
            }
            foreach (JToken dependency in dependencies)
            {
                RequireFields((JObject)dependency, "contentSha256", "kind", "path");
            }

            string address = RequiredString(asset, "address");
            string requestHash = RequiredHash(asset, "canonicalRequestSha256", 64);
            string png = RequiredString(asset, "png");
            string guid = RequiredHash(asset, "unityGuid", 32);
            RequiredHash(asset, "cacheKey", 64);
            RequiredHash(asset, "pngSha256", 64);
            RequiredHash(asset, "unityGuidDerivationSha256", 64);
            if (
                address != $"{AddressPrefix}{requestHash}.png"
                || png != $"textures/{requestHash}.png"
            )
            {
                throw new InvalidOperationException(
                    $"Generated address and PNG path disagree for '{address}'."
                );
            }

            JObject import = RequireObject(asset, "import");
            string filter = RequiredString(import, "filterMode");
            if (import["mipmaps"]?.Type != JTokenType.Boolean)
            {
                throw new InvalidOperationException(
                    $"Generated import contract is invalid for '{address}'."
                );
            }
            bool mipmaps = import["mipmaps"]!.Value<bool>();
            if (
                import["alphaIsTransparency"]?.Value<bool>() != true
                || import["sRgb"]?.Value<bool>() != true
                || RequiredString(import, "textureType") != "default"
            )
            {
                throw new InvalidOperationException(
                    $"Generated import contract is invalid for '{address}'."
                );
            }
            if (mipmaps != (filter == "trilinear"))
            {
                throw new InvalidOperationException(
                    $"Generated import contract is invalid for '{address}'."
                );
            }
            return new AssetRecord(
                address,
                guid,
                $"{GeneratedRoot}/{png}",
                filter,
                mipmaps,
                RequiredString(import, "wrapMode"),
                RequiredString(import, "compression")
            );
        }

        private static void ValidateImportedAssets(Catalog catalog)
        {
            if (
                Validations.TryGetValue(
                    catalog.ManifestHash,
                    out Dictionary<string, Hash128> cached
                )
            )
            {
                bool current = catalog.Assets.All(asset =>
                    cached.TryGetValue(asset.Guid, out Hash128 hash)
                    && hash == AssetDatabase.GetAssetDependencyHash(asset.Path)
                );
                if (current)
                {
                    return;
                }
            }

            var dependencies = new Dictionary<string, Hash128>(StringComparer.Ordinal);
            foreach (AssetRecord asset in catalog.Assets)
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
            Validations[catalog.ManifestHash] = dependencies;
        }

        private static void ValidateImporter(AssetRecord asset)
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
            IReadOnlyCollection<AssetRecord> assets
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
                    entry.address.StartsWith(AddressPrefix, StringComparison.Ordinal)
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

        private static JObject Parse(byte[] bytes, string name)
        {
            using var reader = new JsonTextReader(new StreamReader(new MemoryStream(bytes)))
            {
                DateParseHandling = DateParseHandling.None,
            };
            try
            {
                return JObject.Load(
                    reader,
                    new JsonLoadSettings
                    {
                        DuplicatePropertyNameHandling = DuplicatePropertyNameHandling.Error,
                    }
                );
            }
            catch (Exception exception)
            {
                throw new InvalidOperationException($"The {name} is invalid: {exception.Message}");
            }
        }

        private static JObject RequireObject(JObject owner, string name) =>
            owner[name] as JObject
            ?? throw new InvalidOperationException($"Generated field '{name}' must be an object.");

        private static void RequireFields(JObject value, params string[] expected)
        {
            string[] actual = value.Properties().Select(property => property.Name).ToArray();
            if (!actual.OrderBy(name => name).SequenceEqual(expected.OrderBy(name => name)))
            {
                throw new InvalidOperationException(
                    "Generated JSON has missing or unknown fields."
                );
            }
        }

        private static string RequiredString(JObject value, string name) =>
            value[name]?.Value<string>() is string text && !string.IsNullOrEmpty(text)
                ? text
                : throw new InvalidOperationException(
                    $"Generated field '{name}' must be a string."
                );

        private static string RequiredHash(JObject value, string name, int length)
        {
            string hash = RequiredString(value, name);
            if (hash.Length != length || hash.Any(character => !IsLowerHex(character)))
            {
                throw new InvalidOperationException(
                    $"Generated field '{name}' must be {length} lowercase hexadecimal characters."
                );
            }
            return hash;
        }

        private static void RequireSortedUnique(string[] addresses, string name)
        {
            if (
                addresses.Any(address =>
                    !address.StartsWith(AddressPrefix, StringComparison.Ordinal)
                )
                || !addresses.SequenceEqual(
                    addresses.Distinct(StringComparer.Ordinal).OrderBy(address => address),
                    StringComparer.Ordinal
                )
            )
            {
                throw new InvalidOperationException(
                    $"{name} must be sorted, unique generated addresses."
                );
            }
        }

        private static bool IsLowerHex(char value) =>
            value is >= '0' and <= '9' or >= 'a' and <= 'f';

        private static string Hex(byte[] bytes) =>
            string.Concat(bytes.Select(value => value.ToString("x2")));

        private sealed class Catalog
        {
            public Catalog(string manifestHash, AssetRecord[] assets)
            {
                ManifestHash = manifestHash;
                Assets = assets;
            }

            public string ManifestHash { get; }

            public AssetRecord[] Assets { get; }
        }

        private sealed class AssetRecord
        {
            public AssetRecord(
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
    }
}
