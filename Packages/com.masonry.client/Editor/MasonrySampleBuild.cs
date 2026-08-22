#nullable enable

using System;
using System.Linq;
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
        private const string NativePluginPath = "Assets/Plugins/macOS/libmasonry_rules.dylib";
        private const string WebPluginPath = "Assets/Plugins/WebGL/libmasonry_rules.a";
        private const string WebThreadPool = "-sPTHREAD_POOL_SIZE=navigator.hardwareConcurrency+6";

        public static void Build()
        {
            string output = Required("MASONRY_SAMPLE_BUILD_PATH");
            string scene = Required("MASONRY_SAMPLE_SCENE_PATH");
            bool web = Environment.GetEnvironmentVariable("MASONRY_SAMPLE_PLATFORM") == "web";
            bool webThreads =
                Environment.GetEnvironmentVariable("MASONRY_SAMPLE_WEB_THREADS") == "1";
            BuildTarget target = web ? BuildTarget.WebGL : BuildTarget.StandaloneOSX;
            BuildTargetGroup group = web ? BuildTargetGroup.WebGL : BuildTargetGroup.Standalone;
            if (!EditorUserBuildSettings.SwitchActiveBuildTarget(group, target))
            {
                throw new InvalidOperationException($"Could not activate Unity target {target}.");
            }

            ConfigurePlugin(web);
            string previousEmscriptenArgs = PlayerSettings.WebGL.emscriptenArgs;
            bool previousDecompressionFallback = PlayerSettings.WebGL.decompressionFallback;
            bool previousThreadsSupport = PlayerSettings.WebGL.threadsSupport;
            if (web)
            {
                string emscriptenArgs = RemoveArgument(previousEmscriptenArgs, "-pthread");
                emscriptenArgs = RemoveArgumentsWithPrefix(emscriptenArgs, "-sPTHREAD_POOL_SIZE=");
                emscriptenArgs = AppendArgument(emscriptenArgs, "-fwasm-exceptions");
                if (webThreads)
                {
                    emscriptenArgs = AppendArgument(emscriptenArgs, "-pthread");
                    emscriptenArgs = AppendArgument(emscriptenArgs, WebThreadPool);
                }
                PlayerSettings.WebGL.emscriptenArgs = emscriptenArgs;
                PlayerSettings.WebGL.decompressionFallback = true;
                PlayerSettings.WebGL.threadsSupport = webThreads;
            }

            try
            {
                AddressableAssetSettings settings =
                    AddressableAssetSettingsDefaultObject.GetSettings(true);
                using (OpusBuildAssets.Prepare(settings))
                {
                    BuildAddressables();
                    BuildReport report = BuildPipeline.BuildPlayer(
                        new BuildPlayerOptions
                        {
                            scenes = new[] { scene },
                            locationPathName = output,
                            target = target,
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
                }
            }
            finally
            {
                PlayerSettings.WebGL.emscriptenArgs = previousEmscriptenArgs;
                PlayerSettings.WebGL.decompressionFallback = previousDecompressionFallback;
                PlayerSettings.WebGL.threadsSupport = previousThreadsSupport;
                AssetDatabase.SaveAssets();
                EditorBuildSettings.RemoveConfigObject(
                    AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
                );
            }

            Debug.Log($"MASONRY_SAMPLE_BUILD_OK:{output}");
        }

        private static void BuildAddressables()
        {
            AddressableAssetSettings.CleanPlayerContent();
            AddressableAssetSettings.BuildPlayerContent(out AddressablesPlayerBuildResult result);
            if (!string.IsNullOrEmpty(result.Error))
            {
                throw new InvalidOperationException(result.Error);
            }
        }

        private static void ConfigurePlugin(bool web)
        {
            string pluginPath = web ? WebPluginPath : NativePluginPath;
            AssetDatabase.ImportAsset(pluginPath, ImportAssetOptions.ForceSynchronousImport);
            if (AssetImporter.GetAtPath(pluginPath) is not PluginImporter importer)
            {
                throw new InvalidOperationException(
                    $"Native plugin was not imported: {pluginPath}"
                );
            }

            importer.SetCompatibleWithAnyPlatform(false);
            importer.SetCompatibleWithEditor(false);
            importer.SetCompatibleWithPlatform(BuildTarget.StandaloneOSX, !web);
            importer.SetCompatibleWithPlatform(BuildTarget.WebGL, web);
            if (!web)
            {
                importer.SetPlatformData(BuildTarget.StandaloneOSX, "CPU", "AnyCPU");
            }
            importer.SaveAndReimport();
        }

        private static string AppendArgument(string existing, string argument) =>
            existing.Contains(argument, StringComparison.Ordinal) ? existing
            : string.IsNullOrWhiteSpace(existing) ? argument
            : $"{existing} {argument}";

        private static string RemoveArgument(string existing, string argument) =>
            string.Join(
                " ",
                existing
                    .Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries)
                    .Where(candidate => candidate != argument)
            );

        private static string RemoveArgumentsWithPrefix(string existing, string prefix) =>
            string.Join(
                " ",
                existing
                    .Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries)
                    .Where(candidate => !candidate.StartsWith(prefix, StringComparison.Ordinal))
            );

        private static string Required(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
