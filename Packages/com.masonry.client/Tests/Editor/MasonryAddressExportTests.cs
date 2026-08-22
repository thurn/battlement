#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using Masonry.Editor;
using NUnit.Framework;
using UnityEditor;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.AddressableAssets.Settings.GroupSchemas;
using UnityEngine;

namespace Masonry.Tests
{
    public sealed class MasonryAddressExportTests
    {
        private const string Root = "Assets/MasonryAddressExportTests";

        [SetUp]
        public void SetUp()
        {
            AssetDatabase.DeleteAsset(Root);
            Directory.CreateDirectory(Root);
            AssetDatabase.Refresh();
        }

        [TearDown]
        public void TearDown()
        {
            AssetDatabase.DeleteAsset(Root);
        }

        [Test]
        public void ClassifiesEverySupportedImportedType()
        {
            Type fontType =
                Type.GetType("TMPro.TMP_FontAsset, Unity.TextMeshPro")
                ?? throw new InvalidOperationException("TextMesh Pro is unavailable.");
            Assert.That(MasonryAddressExport.Classify(typeof(SceneAsset)), Is.EqualTo("Scene"));
            Assert.That(MasonryAddressExport.Classify(typeof(GameObject)), Is.EqualTo("Prefab"));
            Assert.That(MasonryAddressExport.Classify(typeof(Texture2D)), Is.EqualTo("Texture"));
            Assert.That(MasonryAddressExport.Classify(typeof(Material)), Is.EqualTo("Material"));
            Assert.That(MasonryAddressExport.Classify(typeof(AudioClip)), Is.EqualTo("AudioClip"));
            Assert.That(MasonryAddressExport.Classify(fontType), Is.EqualTo("Font"));
            Assert.That(MasonryAddressExport.Classify(typeof(TextAsset)), Is.EqualTo("Untyped"));
        }

        [Test]
        public void CollectExportsOnlyExplicitEntriesWithoutExpandingFolders()
        {
            AddressableAssetSettings settings = Settings();
            AddressableAssetGroup group = Group(settings);
            string folder = Root + "/Folder";
            Directory.CreateDirectory(folder);
            File.WriteAllText(folder + "/child.txt", "child");
            AssetDatabase.Refresh();
            Entry(settings, group, folder, "folder");
            Entry(settings, group, TextAsset("standalone.txt"), "standalone");

            List<MasonryAddressExport.ExportEntry> entries = MasonryAddressExport.Collect(settings);

            Assert.That(entries, Has.Count.EqualTo(2));
            Assert.That(entries[0].Address, Is.EqualTo("folder"));
            Assert.That(entries[1].Address, Is.EqualTo("standalone"));
        }

        [Test]
        public void CollectRejectsDuplicateAndMissingAddresses()
        {
            AddressableAssetSettings duplicates = Settings();
            AddressableAssetGroup duplicateGroup = Group(duplicates);
            Entry(duplicates, duplicateGroup, TextAsset("one.txt"), "same");
            Entry(duplicates, duplicateGroup, TextAsset("two.txt"), "same");

            Assert.That(
                () => MasonryAddressExport.Collect(duplicates),
                Throws.InvalidOperationException.With.Message.Contains("declared more than once")
            );

            AddressableAssetSettings missing = Settings();
            AddressableAssetGroup missingGroup = Group(missing);
            AddressableAssetEntry entry = missing.CreateOrMoveEntry(
                "00000000000000000000000000000001",
                missingGroup
            );
            entry.address = "missing";
            Assert.That(
                () => MasonryAddressExport.Collect(missing),
                Throws.InvalidOperationException.With.Message.Contains("missing asset")
            );

            AddressableAssetSettings empty = Settings();
            AddressableAssetGroup emptyGroup = Group(empty);
            AddressableAssetEntry emptyEntry = Entry(
                empty,
                emptyGroup,
                TextAsset("empty.txt"),
                "temporary"
            );
            FieldInfo addressField =
                typeof(AddressableAssetEntry).GetField(
                    "m_Address",
                    BindingFlags.Instance | BindingFlags.NonPublic
                ) ?? throw new InvalidOperationException("Address field is unavailable.");
            addressField.SetValue(emptyEntry, "");
            Assert.That(
                () => MasonryAddressExport.Collect(empty),
                Throws.InvalidOperationException.With.Message.Contains("empty address")
            );
        }

        [Test]
        public void CollectRejectsAddressesExcludedFromTheCatalog()
        {
            AddressableAssetSettings settings = Settings();
            AddressableAssetGroup group = Group(settings);
            group.GetSchema<BundledAssetGroupSchema>().IncludeAddressInCatalog = false;
            Entry(settings, group, TextAsset("excluded.txt"), "excluded");

            Assert.That(
                () => MasonryAddressExport.Collect(settings),
                Throws.InvalidOperationException.With.Message.Contains("excluded")
            );

            group.GetSchema<BundledAssetGroupSchema>().IncludeAddressInCatalog = true;
            group.IncludeInBuild = false;
            Assert.That(
                () => MasonryAddressExport.Collect(settings),
                Throws.InvalidOperationException.With.Message.Contains("excluded")
            );
        }

        private static AddressableAssetSettings Settings() =>
            AddressableAssetSettings.Create(Root, Guid.NewGuid().ToString(), false, false);

        private static AddressableAssetGroup Group(AddressableAssetSettings settings) =>
            settings.CreateGroup(
                "Fixture",
                false,
                false,
                false,
                null,
                typeof(BundledAssetGroupSchema)
            );

        private static AddressableAssetEntry Entry(
            AddressableAssetSettings settings,
            AddressableAssetGroup group,
            string path,
            string address
        )
        {
            AddressableAssetEntry entry = settings.CreateOrMoveEntry(
                AssetDatabase.AssetPathToGUID(path),
                group
            );
            entry.address = address;
            return entry;
        }

        private static string TextAsset(string name)
        {
            string path = Root + "/" + name;
            File.WriteAllText(path, name);
            AssetDatabase.ImportAsset(path);
            return path;
        }
    }
}
