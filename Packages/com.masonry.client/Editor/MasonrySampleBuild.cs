#nullable enable

using System;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Build;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.Build.Reporting;
using UnityEngine;

namespace Masonry.Editor
{
    /// <summary>Builds a convention-based standalone Masonry sample.</summary>
    public static class MasonrySampleBuild
    {
        private const string PluginPath = "Assets/Plugins/macOS/libmasonry_rules.dylib";

        public static void Build()
        {
            string output = Required("MASONRY_SAMPLE_BUILD_PATH");
            string scene = Required("MASONRY_SAMPLE_SCENE_PATH");
            ConfigurePlugin();
            BuildAddressables();

            BuildReport report = BuildPipeline.BuildPlayer(
                new BuildPlayerOptions
                {
                    scenes = new[] { scene },
                    locationPathName = output,
                    target = BuildTarget.StandaloneOSX,
                    options =
                        Environment.GetEnvironmentVariable("MASONRY_SAMPLE_RELEASE") == "1"
                            ? BuildOptions.None
                            : BuildOptions.Development,
                }
            );
            if (report.summary.result != BuildResult.Succeeded)
            {
                throw new InvalidOperationException(
                    $"Masonry sample build failed with {report.summary.totalErrors} errors."
                );
            }

            EditorBuildSettings.RemoveConfigObject(
                AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
            );
            Debug.Log($"MASONRY_SAMPLE_BUILD_OK:{output}");
        }

        private static void BuildAddressables()
        {
            AddressableAssetSettingsDefaultObject.GetSettings(true);
            AddressableAssetSettings.CleanPlayerContent();
            AddressableAssetSettings.BuildPlayerContent(out AddressablesPlayerBuildResult result);
            if (!string.IsNullOrEmpty(result.Error))
            {
                throw new InvalidOperationException(result.Error);
            }
        }

        private static void ConfigurePlugin()
        {
            AssetDatabase.ImportAsset(PluginPath, ImportAssetOptions.ForceSynchronousImport);
            if (AssetImporter.GetAtPath(PluginPath) is not PluginImporter importer)
            {
                throw new InvalidOperationException(
                    $"Native plugin was not imported: {PluginPath}"
                );
            }

            importer.SetCompatibleWithAnyPlatform(false);
            importer.SetCompatibleWithEditor(false);
            importer.SetCompatibleWithPlatform(BuildTarget.StandaloneOSX, true);
            importer.SetPlatformData(BuildTarget.StandaloneOSX, "CPU", "AnyCPU");
            importer.SaveAndReimport();
        }

        private static string Required(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
