#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.AddressableAssets.Settings.GroupSchemas;
using UnityEngine;

namespace Masonry.Editor
{
    /// <summary>Exports explicit Addressables entries for Masonry's Rust generator.</summary>
    public static class MasonryAddressExport
    {
        private const string OutputVariable = "MASONRY_ADDRESS_EXPORT_PATH";

        /// <summary>Writes the active project's explicit Addressables entries as JSON.</summary>
        public static void Export()
        {
            string output =
                Environment.GetEnvironmentVariable(OutputVariable)
                ?? throw new InvalidOperationException($"{OutputVariable} is not set.");
            AddressableAssetSettings? settings = FindSettings();
            if (settings == null)
            {
                throw new InvalidOperationException(
                    "The Unity project has no default Addressables settings."
                );
            }

            List<ExportEntry> entries = Collect(settings);
            File.WriteAllText(output, JsonUtility.ToJson(new ExportFile(entries.ToArray())));
            Debug.Log($"MASONRY_ADDRESS_EXPORT_OK:{entries.Count}");
        }

        internal static List<ExportEntry> Collect(AddressableAssetSettings settings)
        {
            var addresses = new HashSet<string>(StringComparer.Ordinal);
            var entries = new List<ExportEntry>();
            foreach (AddressableAssetGroup? group in settings.groups)
            {
                if (group == null)
                {
                    continue;
                }

                foreach (AddressableAssetEntry entry in group.entries)
                {
                    string address = entry.address;
                    if (string.IsNullOrEmpty(address))
                    {
                        throw new InvalidOperationException(
                            $"Addressables group '{group.Name}' contains an empty address."
                        );
                    }
                    if (!addresses.Add(address))
                    {
                        throw new InvalidOperationException(
                            $"Addressables key '{address}' is declared more than once."
                        );
                    }

                    string path = entry.AssetPath;
                    if (string.IsNullOrEmpty(path) || !File.Exists(path) && !Directory.Exists(path))
                    {
                        throw new InvalidOperationException(
                            $"Addressables key '{address}' refers to missing asset GUID "
                                + $"'{entry.guid}'."
                        );
                    }
                    if (!AddressIsIncluded(group, entry))
                    {
                        throw new InvalidOperationException(
                            $"Addressables key '{address}' is excluded from the runtime catalog "
                                + $"by group '{group.Name}'."
                        );
                    }

                    Type type = AssetDatabase.GetMainAssetTypeAtPath(path);
                    if (type == null)
                    {
                        throw new InvalidOperationException(
                            $"Unity could not determine the imported type of '{path}' "
                                + $"for '{address}'."
                        );
                    }
                    entries.Add(
                        new ExportEntry(address, Classify(type), group.Name, path, type.FullName)
                    );
                }
            }

            entries.Sort((left, right) => string.CompareOrdinal(left.Address, right.Address));
            return entries;
        }

        private static AddressableAssetSettings? FindSettings()
        {
            AddressableAssetSettings? settings = AddressableAssetSettingsDefaultObject.GetSettings(
                false
            );
            if (settings != null)
            {
                return settings;
            }

            string defaultObjectPath =
                AddressableAssetSettingsDefaultObject.kDefaultConfigFolder + "/DefaultObject.asset";
            AddressableAssetSettingsDefaultObject defaultObject =
                AssetDatabase.LoadAssetAtPath<AddressableAssetSettingsDefaultObject>(
                    defaultObjectPath
                );
            if (defaultObject == null)
            {
                return null;
            }
            var serialized = new SerializedObject(defaultObject);
            SerializedProperty guid = serialized.FindProperty("m_AddressableAssetSettingsGuid");
            if (guid == null || string.IsNullOrEmpty(guid.stringValue))
            {
                return null;
            }
            return AssetDatabase.LoadAssetAtPath<AddressableAssetSettings>(
                AssetDatabase.GUIDToAssetPath(guid.stringValue)
            );
        }

        private static bool AddressIsIncluded(
            AddressableAssetGroup group,
            AddressableAssetEntry entry
        )
        {
            if (!group.IncludeInBuild)
            {
                return false;
            }
            bool folder = AssetDatabase.IsValidFolder(entry.AssetPath);
            foreach (AddressableAssetGroupSchema schema in group.Schemas)
            {
                if (!schema.IsEnabled)
                {
                    continue;
                }
                if (schema is BundledAssetGroupSchema bundled)
                {
                    return folder
                        ? bundled.IncludeFolderKeysInCatalog
                        : bundled.IncludeAddressInCatalog;
                }
                if (schema is ContentDirectoryGroupSchema contentDirectory)
                {
                    return !folder || contentDirectory.IncludeFolderKeysInCatalog;
                }
            }
            return false;
        }

        internal static string Classify(Type type)
        {
            if (typeof(SceneAsset).IsAssignableFrom(type))
            {
                return "Scene";
            }
            if (typeof(GameObject).IsAssignableFrom(type))
            {
                return "Prefab";
            }
            if (typeof(Material).IsAssignableFrom(type))
            {
                return "Material";
            }
            if (typeof(Texture).IsAssignableFrom(type))
            {
                return "Texture";
            }
            if (typeof(AudioClip).IsAssignableFrom(type))
            {
                return "AudioClip";
            }
            if (type.FullName == "TMPro.TMP_FontAsset" || IsSubclassOf(type, "TMPro.TMP_FontAsset"))
            {
                return "Font";
            }
            return "Untyped";
        }

        private static bool IsSubclassOf(Type type, string fullName)
        {
            for (Type? current = type.BaseType; current != null; current = current.BaseType)
            {
                if (current.FullName == fullName)
                {
                    return true;
                }
            }
            return false;
        }

        [Serializable]
        private sealed class ExportFile
        {
            public ExportFile(ExportEntry[] entries) => Entries = entries;

            public ExportEntry[] Entries;
        }

        [Serializable]
        internal sealed class ExportEntry
        {
            public ExportEntry(
                string address,
                string kind,
                string group,
                string assetPath,
                string unityType
            )
            {
                Address = address;
                Kind = kind;
                Group = group;
                AssetPath = assetPath;
                UnityType = unityType;
            }

            public string Address;
            public string Kind;
            public string Group;
            public string AssetPath;
            public string UnityType;
        }
    }
}
