#nullable enable

using System.IO;
using NUnit.Framework;
using UnityEditor;
using UnityEngine;

namespace Battlement.Integration.EditorTests
{
    [Parallelizable(ParallelScope.None)]
    public sealed class ReactantGeneratedAssetImportTests
    {
        private const string FixtureDirectory =
            "Assets/BattlementIntegration/ReactantGeneratedImportFixture";
        private const string TexturePath = FixtureDirectory + "/texture.png";
        private const string SidecarPath = FixtureDirectory + "/catalog.json";

        [SetUp]
        public void SetUp()
        {
            AssetDatabase.DeleteAsset(FixtureDirectory);
            Directory.CreateDirectory(FixtureDirectory);
        }

        [TearDown]
        public void TearDown()
        {
            AssetDatabase.DeleteAsset(FixtureDirectory);
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
        }

        [Test]
        public void GeneratedMetadataImportsPngAndRuntimeSidecarWithExactTypesAndSettings()
        {
            string sourceTexture = Path.GetFullPath(
                Path.Combine(
                    Application.dataPath,
                    "../samples/ui/Assets/Original/Signal Texture.png"
                )
            );
            File.WriteAllBytes(TexturePath, File.ReadAllBytes(sourceTexture));
            File.WriteAllText(TexturePath + ".meta", TextureMetadata);
            File.WriteAllText(
                SidecarPath,
                "{\"addresses\":[],\"manifestSha256\":\"" + new string('0', 64) + "\"}\n"
            );
            File.WriteAllText(SidecarPath + ".meta", TextMetadata);
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);

            Texture2D texture = AssetDatabase.LoadAssetAtPath<Texture2D>(TexturePath);
            Assert.That(texture, Is.Not.Null);
            Assert.That(texture.width, Is.EqualTo(128));
            Assert.That(texture.height, Is.EqualTo(96));
            Assert.That(AssetDatabase.GetLabels(texture), Is.Empty);
            TextureImporter importer = (TextureImporter)AssetImporter.GetAtPath(TexturePath);
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

            TextAsset sidecar = AssetDatabase.LoadAssetAtPath<TextAsset>(SidecarPath);
            Assert.That(sidecar, Is.Not.Null);
            Assert.That(sidecar.text, Does.Contain("manifestSha256"));
            Assert.That(AssetDatabase.GetLabels(sidecar), Is.Empty);
        }

        private const string TextMetadata =
            "fileFormatVersion: 2\n"
            + "guid: 11111111111111111111111111111111\n"
            + "TextScriptImporter:\n"
            + "  externalObjects: {}\n"
            + "  userData:\n"
            + "  assetBundleName:\n"
            + "  assetBundleVariant:\n";

        private const string TextureMetadata =
            "fileFormatVersion: 2\n"
            + "guid: 22222222222222222222222222222222\n"
            + "TextureImporter:\n"
            + "  serializedVersion: 13\n"
            + "  mipmaps:\n"
            + "    enableMipMap: 0\n"
            + "    sRGBTexture: 1\n"
            + "  textureSettings:\n"
            + "    serializedVersion: 2\n"
            + "    filterMode: 1\n"
            + "    aniso: 1\n"
            + "    mipBias: 0\n"
            + "    wrapU: 1\n"
            + "    wrapV: 1\n"
            + "    wrapW: 1\n"
            + "  nPOTScale: 0\n"
            + "  alphaUsage: 1\n"
            + "  alphaIsTransparency: 1\n"
            + "  textureType: 0\n"
            + "  platformSettings:\n"
            + "  - serializedVersion: 4\n"
            + "    buildTarget: DefaultTexturePlatform\n"
            + "    maxTextureSize: 16384\n"
            + "    textureFormat: -1\n"
            + "    textureCompression: 0\n"
            + "    compressionQuality: 50\n"
            + "    crunchedCompression: 0\n"
            + "    overridden: 0\n"
            + "  userData:\n"
            + "  assetBundleName:\n"
            + "  assetBundleVariant:\n";
    }
}
