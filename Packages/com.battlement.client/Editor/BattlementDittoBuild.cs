#nullable enable

using System;
using System.Linq;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.Build;
using UnityEditor.Build.Reporting;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>Builds the fixed immutable Ditto macOS player.</summary>
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
            NamedBuildTarget namedTarget = NamedBuildTarget.Standalone;
            string previousDefines = PlayerSettings.GetScriptingDefineSymbols(namedTarget);
            PlayerSettings.SetScriptingDefineSymbols(
                namedTarget,
                ConfigureDiagnostics(previousDefines, diagnostics)
            );
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
                PlayerSettings.SetScriptingDefineSymbols(namedTarget, previousDefines);
                AssetDatabase.SaveAssets();
                EditorBuildSettings.RemoveConfigObject(
                    AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
                );
                BattlementAuthoring.ConfigureNativePlugin();
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

        private static string ConfigureDiagnostics(string definitions, bool enabled)
        {
            string[] values = definitions
                .Split(';', StringSplitOptions.RemoveEmptyEntries)
                .Where(value => value != DiagnosticsDefine)
                .Append(enabled ? DiagnosticsDefine : string.Empty)
                .Where(value => value.Length > 0)
                .Distinct()
                .OrderBy(value => value, StringComparer.Ordinal)
                .ToArray();
            return string.Join(";", values);
        }

        private static string Required(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
