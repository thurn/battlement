#nullable enable

using System;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.Build.Reporting;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>Builds fixed immutable Ditto players.</summary>
    public static class BattlementDittoBuild
    {
        private const string DiagnosticsDefine = "BATTLEMENT_DITTO_DIAGNOSTICS";

        /// <summary>Builds one release player from validated host-provided inputs.</summary>
        public static void BuildMacos()
        {
            string output = Required("BATTLEMENT_DITTO_BUILD_PATH");
            string scene = Required("BATTLEMENT_DITTO_SCENE_PATH");
            bool diagnostics = Diagnostics();
            if (
                !EditorUserBuildSettings.SwitchActiveBuildTarget(
                    BuildTargetGroup.Standalone,
                    BuildTarget.StandaloneOSX
                )
            )
            {
                throw new InvalidOperationException("Could not activate the macOS build target.");
            }

            BattlementSampleBuild.ConfigurePlugin(false);
            try
            {
                AddressableAssetSettings settings = BattlementSampleBuild.AddressableSettings();
                using (OpusBuildAssets.Prepare(settings))
                {
                    BattlementSampleBuild.BuildAddressables();
                    BuildReport report = BuildPipeline.BuildPlayer(
                        new BuildPlayerOptions
                        {
                            scenes = new[] { scene },
                            locationPathName = output,
                            target = BuildTarget.StandaloneOSX,
                            options = BuildOptions.None,
                            extraScriptingDefines = DiagnosticsDefines(diagnostics),
                        }
                    );
                    if (report.summary.result != BuildResult.Succeeded)
                    {
                        throw new InvalidOperationException(
                            $"Ditto player build failed with {report.summary.totalErrors} errors."
                        );
                    }
                }
            }
            finally
            {
                AssetDatabase.SaveAssets();
                EditorBuildSettings.RemoveConfigObject(
                    AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
                );
                BattlementAuthoring.ConfigureNativePlugin();
            }
            Debug.Log($"BATTLEMENT_DITTO_BUILD_OK:{output}");
        }

        /// <summary>Builds one release WebGL player from validated host-provided inputs.</summary>
        public static void BuildWebgl()
        {
            string output = Required("BATTLEMENT_DITTO_BUILD_PATH");
            string scene = Required("BATTLEMENT_DITTO_SCENE_PATH");
            bool diagnostics = Diagnostics();
            if (
                !EditorUserBuildSettings.SwitchActiveBuildTarget(
                    BuildTargetGroup.WebGL,
                    BuildTarget.WebGL
                )
            )
            {
                throw new InvalidOperationException("Could not activate the WebGL build target.");
            }

            BattlementSampleBuild.ConfigurePlugin(true);
            string previousEmscriptenArgs = PlayerSettings.WebGL.emscriptenArgs;
            bool previousFallback = PlayerSettings.WebGL.decompressionFallback;
            bool previousThreads = PlayerSettings.WebGL.threadsSupport;
            WebGLCompressionFormat previousCompression = PlayerSettings.WebGL.compressionFormat;
            PlayerSettings.WebGL.emscriptenArgs = "-fwasm-exceptions";
            PlayerSettings.WebGL.compressionFormat = WebGLCompressionFormat.Gzip;
            PlayerSettings.WebGL.decompressionFallback = false;
            PlayerSettings.WebGL.threadsSupport = false;
            try
            {
                AddressableAssetSettings settings = BattlementSampleBuild.AddressableSettings();
                using (OpusBuildAssets.Prepare(settings))
                {
                    BattlementSampleBuild.BuildAddressables();
                    BuildReport report = BuildPipeline.BuildPlayer(
                        new BuildPlayerOptions
                        {
                            scenes = new[] { scene },
                            locationPathName = output,
                            target = BuildTarget.WebGL,
                            options = BuildOptions.None,
                            extraScriptingDefines = DiagnosticsDefines(diagnostics),
                        }
                    );
                    if (report.summary.result != BuildResult.Succeeded)
                    {
                        throw new InvalidOperationException(
                            $"Ditto WebGL build failed with {report.summary.totalErrors} errors."
                        );
                    }
                    BattlementSampleBuild.SetWebDevicePixelRatio(output);
                }
            }
            finally
            {
                PlayerSettings.WebGL.emscriptenArgs = previousEmscriptenArgs;
                PlayerSettings.WebGL.compressionFormat = previousCompression;
                PlayerSettings.WebGL.decompressionFallback = previousFallback;
                PlayerSettings.WebGL.threadsSupport = previousThreads;
                AssetDatabase.SaveAssets();
                EditorBuildSettings.RemoveConfigObject(
                    AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
                );
            }
            Debug.Log($"BATTLEMENT_DITTO_BUILD_OK:{output}");
        }

        private static bool Diagnostics() =>
            Environment.GetEnvironmentVariable("BATTLEMENT_DITTO_DIAGNOSTICS") switch
            {
                "1" => true,
                "0" => false,
                _ => throw new InvalidOperationException(
                    "BATTLEMENT_DITTO_DIAGNOSTICS must be 0 or 1."
                ),
            };

        private static string[] DiagnosticsDefines(bool enabled) =>
            enabled ? new[] { DiagnosticsDefine } : Array.Empty<string>();

        private static string Required(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
