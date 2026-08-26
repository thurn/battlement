#nullable enable

using System;
using System.Linq;
using Battlement.VisualCapture;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Build;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>Builds an isolated sample project with the repository capture harness.</summary>
    public static class SampleVisualCaptureBuild
    {
        private const string PluginPath = "Assets/Plugins/macOS/libbattlement_rules.dylib";

        public static void Build()
        {
            string output = Required("BATTLEMENT_CAPTURE_BUILD_PATH");
            string scenePath = Required("BATTLEMENT_CAPTURE_SCENE_PATH");
            string scenarioName = Required("BATTLEMENT_CAPTURE_SCENARIO");
            string captureScenePath = AddScenario(scenePath, scenarioName);
            ConfigurePlugin();
            BuildAddressables();

            BuildReport report = BuildPipeline.BuildPlayer(
                new BuildPlayerOptions
                {
                    scenes = new[] { captureScenePath },
                    locationPathName = output,
                    target = BuildTarget.StandaloneOSX,
                    options = BuildOptions.None,
                }
            );
            if (report.summary.result != BuildResult.Succeeded)
            {
                throw new InvalidOperationException(
                    $"Sample capture build failed with {report.summary.totalErrors} errors."
                );
            }

            EditorBuildSettings.RemoveConfigObject(
                AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
            );
            Debug.Log($"BATTLEMENT_CAPTURE_BUILD_OK:{output}");
        }

        private static string AddScenario(string scenePath, string scenarioName)
        {
            var scene = EditorSceneManager.OpenScene(scenePath, OpenSceneMode.Single);
            Type scenarioType = scenarioName switch
            {
                "basic-sample" => typeof(BasicSampleCaptureScenario),
                "tictactoe-sample" => typeof(TicTacToeSampleCaptureScenario),
                "chess-sample" => typeof(ChessSampleCaptureScenario),
                "chess-logging-sample" => typeof(ChessLoggingSampleCaptureScenario),
                "ui-sample" => typeof(UiSampleCaptureScenario),
                "ui-asset-gallery" => typeof(UiAssetGalleryCaptureScenario),
                "ui-asset-switch" => typeof(UiAssetSwitchCaptureScenario),
                "ui-layout" => typeof(UiLayoutCaptureScenario),
                "ui-appearance-matrix" => typeof(UiAppearanceMatrixCaptureScenario),
                "ui-appearance-visibility" => typeof(UiAppearanceVisibilityCaptureScenario),
                "ui-background-sources" => typeof(UiBackgroundSourcesCaptureScenario),
                "ui-background-modes" => typeof(UiBackgroundModesCaptureScenario),
                _ => throw new InvalidOperationException(
                    $"Unknown sample scenario: {scenarioName}"
                ),
            };

            var scenarioObject = new GameObject("Sample Visual Capture");
            scenarioObject.AddComponent(scenarioType);
            string captureScenePath = "Assets/Scenes/BattlementGeneratedCapture.unity";
            EditorSceneManager.SaveScene(scene, captureScenePath);
            AssetDatabase.ImportAsset(captureScenePath, ImportAssetOptions.ForceSynchronousImport);
            int matches = UnityEngine
                .Object.FindObjectsByType<BattlementCaptureScenario>(FindObjectsInactive.Include)
                .Count(value => value.ScenarioName == scenarioName);
            if (matches != 1)
            {
                throw new InvalidOperationException(
                    $"Capture scene must contain exactly one '{scenarioName}' scenario."
                );
            }
            return captureScenePath;
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
