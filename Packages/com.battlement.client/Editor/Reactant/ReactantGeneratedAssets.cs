#nullable enable

using System;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using Battlement.Editor;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using UnityEditor;
using UnityEditor.AddressableAssets.Settings;

namespace Reactant.Editor
{
    [InitializeOnLoad]
    internal static class ReactantEditorPreparation
    {
        static ReactantEditorPreparation()
        {
            BattlementEditorPreparation.Register(ReactantGeneratedAssets.Prepare);
        }
    }

    /// <summary>Temporarily registers a validated generated texture catalog.</summary>
    internal sealed class ReactantGeneratedAssets : IDisposable
    {
        private const string AddressPrefix = "battlement-reactant/generated/";
        private const string GeneratedRoot = "Assets/Generated/BattlementReactant";
        private const string ManifestPath = GeneratedRoot + "/manifest.json";
        private const string SidecarPath =
            GeneratedRoot + "/Resources/BattlementReactantAssetCatalog.json";

        private readonly IDisposable registration;
        private bool isDisposed;

        private ReactantGeneratedAssets(IDisposable registration, bool hasEntries)
        {
            this.registration = registration;
            HasEntries = hasEntries;
        }

        internal bool HasEntries { get; }

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

                return new ReactantGeneratedAssets(EmptyDisposable.Instance, false);
            }
            if (!File.Exists(SidecarPath))
            {
                throw new InvalidOperationException(
                    "The generated Reactant manifest has no runtime sidecar."
                );
            }

            Catalog catalog = ReadCatalog();
            return new ReactantGeneratedAssets(
                BattlementGeneratedTextureImport.Prepare(
                    settings,
                    AddressPrefix,
                    catalog.ManifestHash,
                    catalog.Assets
                ),
                catalog.Assets.Length > 0
            );
        }

        public void Dispose()
        {
            if (isDisposed)
            {
                return;
            }

            registration.Dispose();
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
            BattlementGeneratedTexture[] records = assets.Select(ReadAsset).ToArray();
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

        private static BattlementGeneratedTexture ReadAsset(JToken token)
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
            return new BattlementGeneratedTexture(
                address,
                guid,
                $"{GeneratedRoot}/{png}",
                filter,
                mipmaps,
                RequiredString(import, "wrapMode"),
                RequiredString(import, "compression")
            );
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
            public Catalog(string manifestHash, BattlementGeneratedTexture[] assets)
            {
                ManifestHash = manifestHash;
                Assets = assets;
            }

            public string ManifestHash { get; }

            public BattlementGeneratedTexture[] Assets { get; }
        }

        private sealed class EmptyDisposable : IDisposable
        {
            public static readonly EmptyDisposable Instance = new();

            public void Dispose() { }
        }
    }
}
