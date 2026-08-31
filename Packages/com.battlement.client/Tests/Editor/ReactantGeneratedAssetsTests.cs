#nullable enable

using System;
using System.IO;
using System.Linq;
using System.Security.Cryptography;
using Battlement.Editor;
using Newtonsoft.Json;
using Newtonsoft.Json.Linq;
using NUnit.Framework;
using UnityEditor;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.AddressableAssets.Settings.GroupSchemas;
using UnityEngine;

namespace Battlement.Tests
{
    [Parallelizable(ParallelScope.None)]
    public sealed class ReactantGeneratedAssetsTests
    {
        private const string AddressPrefix = "battlement-reactant/generated/";
        private const string GeneratedParent = "Assets/Generated";
        private const string GeneratedRoot = "Assets/Generated/BattlementReactant";
        private const string ManifestPath = GeneratedRoot + "/manifest.json";
        private const string RequestHash =
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        private const string SettingsRoot = "Assets/ReactantGeneratedAssetsTests";
        private const string SidecarPath =
            GeneratedRoot + "/Resources/BattlementReactantAssetCatalog.json";
        private const string TexturePath = GeneratedRoot + "/textures/" + RequestHash + ".png";

        [SetUp]
        public void SetUp()
        {
            CleanGeneratedAssets();
            AssetDatabase.DeleteAsset(SettingsRoot);
        }

        [TearDown]
        public void TearDown()
        {
            CleanGeneratedAssets();
            AssetDatabase.DeleteAsset(SettingsRoot);
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
        }

        [Test]
        public void RegistersExactTextureTemporarilyAndRestoresUserSettings()
        {
            GeneratedAsset generated = CreateGeneratedAsset();
            AddressableAssetSettings settings = CreateSettings();
            AddressableAssetGroup group = CreateGroup(settings);
            EditorUtility.ClearDirty(settings);
            EditorUtility.ClearDirty(group);
            Hash128 originalHash = settings.currentHash;

            using (ReactantGeneratedAssets owner = ReactantGeneratedAssets.Prepare(settings))
            {
                Assert.That(owner.HasEntries, Is.True);
                AddressableAssetEntry entry = settings.FindAssetEntry(generated.Guid);
                Assert.That(entry, Is.Not.Null);
                Assert.That(entry.address, Is.EqualTo(generated.Address));
                Assert.That(
                    AssetDatabase.LoadAssetAtPath(TexturePath, typeof(Texture2D)),
                    Is.TypeOf<Texture2D>()
                );
                AssertImporter();
            }

            Assert.That(settings.FindAssetEntry(generated.Guid), Is.Null);
            Assert.That(settings.currentHash, Is.EqualTo(originalHash));
            Assert.That(EditorUtility.IsDirty(settings), Is.False);
            Assert.That(EditorUtility.IsDirty(group), Is.False);
        }

        [Test]
        public void AbsentCatalogIsAnEmptyNoOp()
        {
            AddressableAssetSettings settings = CreateSettings();
            AddressableAssetGroup group = CreateGroup(settings);
            int groups = settings.groups.Count;

            using ReactantGeneratedAssets owner = ReactantGeneratedAssets.Prepare(settings);

            Assert.That(owner.HasEntries, Is.False);
            Assert.That(settings.groups, Has.Count.EqualTo(groups));
            Assert.That(group.entries, Is.Empty);
        }

        [Test]
        [Category("ReactantGeneratedAssetsExhaustive")]
        public void RejectsUserOwnedAddressAndGuidConflictsWithoutMutation()
        {
            GeneratedAsset generated = CreateGeneratedAsset();
            AddressableAssetSettings addressSettings = CreateSettings();
            AddressableAssetGroup addressGroup = CreateGroup(addressSettings);
            AddressableAssetEntry addressConflict = addressSettings.CreateOrMoveEntry(
                CreateTextAsset("address.txt"),
                addressGroup,
                false,
                false
            );
            addressConflict.address = generated.Address;

            Assert.That(
                () => ReactantGeneratedAssets.Prepare(addressSettings),
                Throws.InvalidOperationException.With.Message.Contains("User-owned")
            );
            Assert.That(addressGroup.entries, Has.Count.EqualTo(1));
            Assert.That(addressConflict.address, Is.EqualTo(generated.Address));

            AssetDatabase.DeleteAsset(SettingsRoot);
            AddressableAssetSettings guidSettings = CreateSettings();
            AddressableAssetGroup guidGroup = CreateGroup(guidSettings);
            AddressableAssetEntry guidConflict = guidSettings.CreateOrMoveEntry(
                generated.Guid,
                guidGroup,
                false,
                false
            );
            guidConflict.address = "user/texture";

            Assert.That(
                () => ReactantGeneratedAssets.Prepare(guidSettings),
                Throws.InvalidOperationException.With.Message.Contains("User-owned")
            );
            Assert.That(guidConflict.address, Is.EqualTo("user/texture"));
        }

        [Test]
        [Category("ReactantGeneratedAssetsExhaustive")]
        public void UnchangedManifestDoesNotDirtyOrRewriteImportedState()
        {
            CreateGeneratedAsset();
            AddressableAssetSettings settings = CreateSettings();
            AddressableAssetGroup group = CreateGroup(settings);
            EditorUtility.ClearDirty(settings);
            EditorUtility.ClearDirty(group);
            string settingsJson = EditorJsonUtility.ToJson(settings);
            Hash128 settingsHash = settings.currentHash;
            DateTime metadataWrite = File.GetLastWriteTimeUtc(TexturePath + ".meta");
            Hash128 dependency = AssetDatabase.GetAssetDependencyHash(TexturePath);

            ReactantGeneratedAssets.Prepare(settings).Dispose();
            ReactantGeneratedAssets.Prepare(settings).Dispose();

            Assert.That(EditorJsonUtility.ToJson(settings), Is.EqualTo(settingsJson));
            Assert.That(settings.currentHash, Is.EqualTo(settingsHash));
            Assert.That(File.GetLastWriteTimeUtc(TexturePath + ".meta"), Is.EqualTo(metadataWrite));
            Assert.That(AssetDatabase.GetAssetDependencyHash(TexturePath), Is.EqualTo(dependency));
            Assert.That(EditorUtility.IsDirty(settings), Is.False);
            Assert.That(EditorUtility.IsDirty(group), Is.False);
        }

        [Test]
        [Category("ReactantGeneratedAssetsExhaustive")]
        public void RejectsSidecarDriftAndStaleImportSettings()
        {
            CreateGeneratedAsset();
            File.WriteAllText(
                SidecarPath,
                $"{{\"addresses\":[],\"manifestSha256\":\"{new string('0', 64)}\"}}"
            );
            Assert.That(
                () => ReactantGeneratedAssets.Prepare(SettingsWithGroup()),
                Throws.InvalidOperationException.With.Message.Contains("address set")
            );

            CreateGeneratedAsset();
            var importer = (TextureImporter)AssetImporter.GetAtPath(TexturePath);
            importer.mipmapEnabled = true;
            importer.SaveAndReimport();
            Assert.That(
                () => ReactantGeneratedAssets.Prepare(SettingsWithGroup()),
                Throws.InvalidOperationException.With.Message.Contains(
                    "stale texture import settings"
                )
            );
        }

        private static GeneratedAsset CreateGeneratedAsset()
        {
            Directory.CreateDirectory(Path.GetDirectoryName(TexturePath)!);
            Directory.CreateDirectory(Path.GetDirectoryName(SidecarPath)!);
            var texture = new Texture2D(2, 2, TextureFormat.RGBA32, false);
            texture.SetPixels32(
                new[]
                {
                    new UnityEngine.Color32(0, 0, 0, 0),
                    new UnityEngine.Color32(255, 0, 0, 255),
                    new UnityEngine.Color32(0, 255, 0, 255),
                    new UnityEngine.Color32(0, 0, 255, 255),
                }
            );
            texture.Apply();
            File.WriteAllBytes(TexturePath, texture.EncodeToPNG());
            UnityEngine.Object.DestroyImmediate(texture);
            AssetDatabase.ImportAsset(TexturePath, ImportAssetOptions.ForceSynchronousImport);
            var importer = (TextureImporter)AssetImporter.GetAtPath(TexturePath);
            importer.textureType = TextureImporterType.Default;
            importer.sRGBTexture = true;
            importer.alphaIsTransparency = true;
            importer.mipmapEnabled = false;
            importer.filterMode = FilterMode.Bilinear;
            importer.wrapMode = TextureWrapMode.Clamp;
            importer.textureCompression = TextureImporterCompression.Uncompressed;
            importer.SaveAndReimport();
            string guid = AssetDatabase.AssetPathToGUID(TexturePath);
            string address = AddressPrefix + RequestHash + ".png";
            JObject manifest = Manifest(address, guid);
            byte[] manifestBytes = System.Text.Encoding.UTF8.GetBytes(
                manifest.ToString(Formatting.Indented) + "\n"
            );
            File.WriteAllBytes(ManifestPath, manifestBytes);
            string manifestHash = Hex(SHA256.Create().ComputeHash(manifestBytes));
            File.WriteAllText(
                SidecarPath,
                new JObject
                {
                    ["addresses"] = new JArray(address),
                    ["manifestSha256"] = manifestHash,
                }.ToString(Formatting.Indented) + "\n"
            );
            AssetDatabase.ImportAsset(ManifestPath, ImportAssetOptions.ForceSynchronousImport);
            AssetDatabase.ImportAsset(SidecarPath, ImportAssetOptions.ForceSynchronousImport);
            return new GeneratedAsset(address, guid);
        }

        private static JObject Manifest(string address, string guid) =>
            new()
            {
                ["assets"] = new JArray(
                    new JObject
                    {
                        ["address"] = address,
                        ["cacheKey"] = new string('b', 64),
                        ["canonicalRequestSha256"] = RequestHash,
                        ["dependencies"] = new JArray(),
                        ["import"] = new JObject
                        {
                            ["alphaIsTransparency"] = true,
                            ["compression"] = "lossless",
                            ["filterMode"] = "bilinear",
                            ["mipmaps"] = false,
                            ["sRgb"] = true,
                            ["textureType"] = "default",
                            ["wrapMode"] = "clamp",
                        },
                        ["kind"] = "background",
                        ["logicalCanvas"] = new JObject { ["height"] = 2, ["width"] = 2 },
                        ["png"] = $"textures/{RequestHash}.png",
                        ["pngSha256"] = new string('c', 64),
                        ["rasterScale"] = 1,
                        ["rasterSize"] = new JObject { ["height"] = 2, ["width"] = 2 },
                        ["sliceInsets"] = null,
                        ["subjectBounds"] = new JObject
                        {
                            ["height"] = 2,
                            ["width"] = 2,
                            ["x"] = 0,
                            ["y"] = 0,
                        },
                        ["unityGuid"] = guid,
                        ["unityGuidDerivationSha256"] = new string('d', 64),
                    }
                ),
                ["browser"] = new JObject
                {
                    ["executableFileIdentity"] = new JObject
                    {
                        ["byteLength"] = 1,
                        ["fileId"] = "unix:1:1",
                        ["modifiedNanoseconds"] = 1,
                    },
                    ["executablePath"] = "/browser",
                    ["executableSha256"] = new string('e', 64),
                    ["product"] = "Chrome",
                    ["version"] = "1",
                },
                ["rendererIdentity"] = new string('f', 64),
            };

        private static AddressableAssetSettings SettingsWithGroup()
        {
            AssetDatabase.DeleteAsset(SettingsRoot);
            AddressableAssetSettings settings = CreateSettings();
            CreateGroup(settings);
            return settings;
        }

        private static AddressableAssetSettings CreateSettings()
        {
            Directory.CreateDirectory(SettingsRoot);
            return AddressableAssetSettings.Create(
                SettingsRoot,
                $"Settings-{Guid.NewGuid():N}",
                false,
                false
            );
        }

        private static AddressableAssetGroup CreateGroup(AddressableAssetSettings settings) =>
            settings.CreateGroup(
                "Fixture",
                false,
                false,
                false,
                null,
                typeof(BundledAssetGroupSchema)
            );

        private static string CreateTextAsset(string name)
        {
            string path = SettingsRoot + "/" + name;
            File.WriteAllText(path, name);
            AssetDatabase.ImportAsset(path, ImportAssetOptions.ForceSynchronousImport);
            return AssetDatabase.AssetPathToGUID(path);
        }

        private static void AssertImporter()
        {
            var importer = (TextureImporter)AssetImporter.GetAtPath(TexturePath);
            Assert.That(importer.textureType, Is.EqualTo(TextureImporterType.Default));
            Assert.That(importer.sRGBTexture, Is.True);
            Assert.That(importer.alphaIsTransparency, Is.True);
            Assert.That(importer.mipmapEnabled, Is.False);
            Assert.That(importer.filterMode, Is.EqualTo(FilterMode.Bilinear));
            Assert.That(importer.wrapMode, Is.EqualTo(TextureWrapMode.Clamp));
            Assert.That(
                importer.textureCompression,
                Is.EqualTo(TextureImporterCompression.Uncompressed)
            );
        }

        private static void CleanGeneratedAssets()
        {
            AssetDatabase.DeleteAsset(GeneratedRoot);
            if (
                Directory.Exists(GeneratedParent)
                && !Directory.EnumerateFileSystemEntries(GeneratedParent).Any()
            )
            {
                AssetDatabase.DeleteAsset(GeneratedParent);
            }
        }

        private static string Hex(byte[] bytes) =>
            string.Concat(bytes.Select(value => value.ToString("x2")));

        private sealed class GeneratedAsset
        {
            public GeneratedAsset(string address, string guid)
            {
                Address = address;
                Guid = guid;
            }

            public string Address { get; }

            public string Guid { get; }
        }
    }
}
