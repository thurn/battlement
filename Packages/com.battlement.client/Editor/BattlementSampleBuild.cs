#nullable enable

using System;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Build;
using UnityEditor.AddressableAssets.Build.DataBuilders;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.Build.Reporting;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>Builds a convention-based standalone Battlement sample.</summary>
    public static class BattlementSampleBuild
    {
        private const string NativePluginPath = "Assets/Plugins/macOS/libbattlement_rules.dylib";
        private const string WebPluginPath = "Assets/Plugins/WebGL/libbattlement_rules.a";

        // init.js selects a current-thread pool on mobile and reserves dedicated
        // Rayon workers on desktop. Emscripten evaluates this expression only after
        // the page initializer has established that runtime policy.
        private const string WebThreadPool =
            "-sPTHREAD_POOL_SIZE=globalThis.battlementWebThreads.pthreadPoolSize";

        // Make development builds fail loudly if code exceeds the prestarted pool;
        // release builds avoid the debug instrumentation and its console overhead.
        private const string WebThreadPoolStrict = "-sPTHREAD_POOL_SIZE_STRICT=2";
        private const string WebThreadDebug = "-sPTHREADS_DEBUG=1";
        private const string WebScriptStart = "    <script>\n      var canvas =";
        private const string WebScriptEnd = "\n    </script>";
        private const string WebThreadGuard =
            "    <script>\n"
            // init.js owns compatibility detection and presentation. This minimal
            // branch must remain in Unity's generated entry point so an unsupported
            // browser never requests the threaded loader or starts its Wasm module.
            + "      if (!window.battlementWebThreads.isSupported) {\n"
            + "        window.battlementWebThreads.showUnsupportedError();\n"
            + "      } else {\n"
            + "        var canvas =";

        public static void Build()
        {
            string output = Required("BATTLEMENT_SAMPLE_BUILD_PATH");
            string scene = Required("BATTLEMENT_SAMPLE_SCENE_PATH");
            bool web = Environment.GetEnvironmentVariable("BATTLEMENT_SAMPLE_PLATFORM") == "web";
            bool webThreads =
                Environment.GetEnvironmentVariable("BATTLEMENT_SAMPLE_WEB_THREADS") == "1";
            bool release = Environment.GetEnvironmentVariable("BATTLEMENT_SAMPLE_RELEASE") == "1";
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
                emscriptenArgs = RemoveArgumentsWithPrefix(
                    emscriptenArgs,
                    "-sPTHREAD_POOL_SIZE_STRICT="
                );
                emscriptenArgs = RemoveArgumentsWithPrefix(emscriptenArgs, "-sPTHREADS_DEBUG=");
                emscriptenArgs = AppendArgument(emscriptenArgs, "-fwasm-exceptions");
                if (webThreads)
                {
                    emscriptenArgs = AppendArgument(emscriptenArgs, "-pthread");
                    emscriptenArgs = AppendArgument(emscriptenArgs, WebThreadPool);
                    if (!release)
                    {
                        emscriptenArgs = AppendArgument(emscriptenArgs, WebThreadPoolStrict);
                        emscriptenArgs = AppendArgument(emscriptenArgs, WebThreadDebug);
                    }
                }
                PlayerSettings.WebGL.emscriptenArgs = emscriptenArgs;
                PlayerSettings.WebGL.decompressionFallback = true;
                PlayerSettings.WebGL.threadsSupport = webThreads;
            }

            try
            {
                AddressableAssetSettings settings = AddressableSettings();
                using (OpusBuildAssets.Prepare(settings))
                {
                    BuildAddressables();
                    BuildReport report = BuildPipeline.BuildPlayer(
                        new BuildPlayerOptions
                        {
                            scenes = new[] { scene },
                            locationPathName = output,
                            target = target,
                            options = release ? BuildOptions.None : BuildOptions.Development,
                        }
                    );
                    if (report.summary.result != BuildResult.Succeeded)
                    {
                        throw new InvalidOperationException(
                            "Battlement sample build failed with "
                                + $"{report.summary.totalErrors} errors."
                        );
                    }
                    if (web)
                    {
                        if (webThreads)
                        {
                            AddWebThreadGuard(output);
                        }
                        SetWebDevicePixelRatio(output);
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
                if (!web)
                {
                    BattlementAuthoring.ConfigureNativePlugin();
                }
            }

            Debug.Log($"BATTLEMENT_SAMPLE_BUILD_OK:{output}");
        }

        private static void BuildAddressables()
        {
            AddressableAssetSettings.BuildPlayerContent(out AddressablesPlayerBuildResult result);
            if (!string.IsNullOrEmpty(result.Error))
            {
                throw new InvalidOperationException(result.Error);
            }
        }

        private static AddressableAssetSettings AddressableSettings()
        {
            AddressableAssetSettingsDefaultObject.GetSettings(true);
            AssetDatabase.SaveAssets();
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
            AddressableAssetSettings settings = AddressableAssetSettingsDefaultObject.GetSettings(
                false
            );
            if (!settings)
            {
                throw new InvalidOperationException("Addressables settings were not created.");
            }
            int packed = settings.DataBuilders.FindIndex(builder =>
                builder is BuildScriptPackedMode
            );
            if (packed < 0)
            {
                throw new InvalidOperationException("Addressables has no packed-mode builder.");
            }
            settings.ActivePlayerDataBuilderIndex = packed;
            return settings;
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

        private static void AddWebThreadGuard(string output)
        {
            string indexPath = Path.Combine(output, "index.html");
            string html = File.ReadAllText(indexPath);
            if (!html.Contains(WebScriptStart, StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"Unity Web template in {indexPath} does not contain the expected script."
                );
            }

            html = html.Replace(WebScriptStart, WebThreadGuard, StringComparison.Ordinal);
            int scriptEnd = html.LastIndexOf(WebScriptEnd, StringComparison.Ordinal);
            if (scriptEnd < 0)
            {
                throw new InvalidOperationException(
                    $"Unity Web template in {indexPath} does not contain a closing script tag."
                );
            }
            html = html.Insert(scriptEnd, "\n      }");
            File.WriteAllText(indexPath, html);
        }

        private static void SetWebDevicePixelRatio(string output)
        {
            string indexPath = Path.Combine(output, "index.html");
            string html = File.ReadAllText(indexPath);
            const string configStart = "      var config = {\n";
            if (!html.Contains(configStart, StringComparison.Ordinal))
            {
                throw new InvalidOperationException(
                    $"Unity Web template in {indexPath} does not contain the expected config."
                );
            }
            html = html.Replace(
                configStart,
                configStart + "        devicePixelRatio: 1,\n",
                StringComparison.Ordinal
            );
            File.WriteAllText(indexPath, html);
        }

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
