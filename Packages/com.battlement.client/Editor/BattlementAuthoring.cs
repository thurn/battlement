#nullable enable

using System;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Build.DataBuilders;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.SceneManagement;

namespace Battlement.Editor
{
    /// <summary>Opens and plays a Battlement game in the Unity Editor.</summary>
    public static class BattlementAuthoring
    {
        private const string NativePluginPath = "Assets/Plugins/macOS/libbattlement_rules.dylib";

        [MenuItem("Battlement/Play Game")]
        public static void Play()
        {
            ConfigureNativePlugin();
            SelectFastPlayMode();
            EditorApplication.isPlaying = true;
        }

        public static void OpenAndPlay()
        {
            EditorSceneManager.OpenScene(Required("BATTLEMENT_AUTHOR_SCENE_PATH"));
            Play();
        }

        internal static void ConfigureNativePlugin()
        {
            AssetDatabase.ImportAsset(NativePluginPath, ImportAssetOptions.ForceSynchronousImport);
            if (AssetImporter.GetAtPath(NativePluginPath) is not PluginImporter importer)
            {
                throw new InvalidOperationException(
                    $"Native plugin was not imported: {NativePluginPath}"
                );
            }

            importer.SetCompatibleWithAnyPlatform(false);
            importer.SetCompatibleWithEditor(true);
            importer.SetCompatibleWithPlatform(BuildTarget.StandaloneOSX, true);
            importer.SetPlatformData(BuildTarget.StandaloneOSX, "CPU", "AnyCPU");
            importer.SaveAndReimport();
        }

        private static void SelectFastPlayMode()
        {
            AddressableAssetSettings settings = AddressableAssetSettingsDefaultObject.GetSettings(
                true
            );
            int index = settings.DataBuilders.FindIndex(builder => builder is BuildScriptFastMode);
            if (index < 0)
            {
                throw new InvalidOperationException(
                    "Addressables has no fast-mode builder for Editor play."
                );
            }

            settings.ActivePlayModeDataBuilderIndex = index;
        }

        private static string Required(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
