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
#if UNITY_EDITOR_WIN
        private const string PluginPath = "Assets/Plugins/x86_64/battlement_rules.dll";
        private const BuildTarget NativeBuildTarget = BuildTarget.StandaloneWindows64;
#else
        private const string PluginPath = "Assets/Plugins/macOS/libbattlement_rules.dylib";
        private const BuildTarget NativeBuildTarget = BuildTarget.StandaloneOSX;
#endif

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
                    target = NativeBuildTarget,
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
                "fps-viewer" => typeof(BattlementFpsViewerCaptureScenario),
                "reactant-events-changed" => typeof(ReactantEventsChangedCaptureScenario),
                "reactant-events-restored" => typeof(ReactantEventsRestoredCaptureScenario),
                "reactant-state-initial" => typeof(ReactantStateInitialCaptureScenario),
                "reactant-state-updated" => typeof(ReactantStateUpdatedCaptureScenario),
                "reactant-state-reordered" => typeof(ReactantStateReorderedCaptureScenario),
                "reactant-state-restored" => typeof(ReactantStateRestoredCaptureScenario),
                "reactant-context-outer" => typeof(ReactantContextOuterCaptureScenario),
                "reactant-context-unrelated" => typeof(ReactantContextUnrelatedCaptureScenario),
                "reactant-context-overridden" => typeof(ReactantContextOverriddenCaptureScenario),
                "reactant-context-restored" => typeof(ReactantContextRestoredCaptureScenario),
                "reactant-effects-disconnected" =>
                    typeof(ReactantEffectsDisconnectedCaptureScenario),
                "reactant-effects-connected" => typeof(ReactantEffectsConnectedCaptureScenario),
                "reactant-effects-restored" => typeof(ReactantEffectsRestoredCaptureScenario),
                "reactant-store-swapped" => typeof(ReactantStoreSwappedCaptureScenario),
                "reactant-store-updated" => typeof(ReactantStoreUpdatedCaptureScenario),
                "reactant-store-restored" => typeof(ReactantStoreRestoredCaptureScenario),
                "reactant-resources-initial" => typeof(ReactantResourcesInitialCaptureScenario),
                "reactant-resources-pending" => typeof(ReactantResourcesPendingCaptureScenario),
                "reactant-resources-ready" => typeof(ReactantResourcesReadyCaptureScenario),
                "reactant-resources-error" => typeof(ReactantResourcesErrorCaptureScenario),
                "reactant-resources-restored" => typeof(ReactantResourcesRestoredCaptureScenario),
                "ui-sample" => typeof(UiSampleCaptureScenario),
                "ui-asset-gallery" => typeof(UiAssetGalleryCaptureScenario),
                "ui-asset-switch" => typeof(UiAssetSwitchCaptureScenario),
                "ui-layout" => typeof(UiLayoutCaptureScenario),
                "geometry-screen-space" => typeof(GeometryScreenSpaceCaptureScenario),
                "ui-appearance-matrix" => typeof(UiAppearanceMatrixCaptureScenario),
                "ui-appearance-visibility" => typeof(UiAppearanceVisibilityCaptureScenario),
                "ui-background-sources" => typeof(UiBackgroundSourcesCaptureScenario),
                "ui-background-modes" => typeof(UiBackgroundModesCaptureScenario),
                "ui-transforms" => typeof(UiTransformsCaptureScenario),
                "ui-typography" => typeof(UiTypographyCaptureScenario),
                "ui-buttons" => typeof(UiButtonsCaptureScenario),
                "ui-containers" => typeof(UiContainersCaptureScenario),
                "ui-scroll-controls" => typeof(UiScrollControlsCaptureScenario),
                "ui-tabs" => typeof(UiTabsCaptureScenario),
                "ui-text-fields" => typeof(UiTextFieldsCaptureScenario),
                "ui-boolean-controls" => typeof(UiBooleanControlsCaptureScenario),
                "ui-choice-groups" => typeof(UiChoiceGroupsCaptureScenario),
                "ui-dropdown" => typeof(UiDropdownCaptureScenario),
                "ui-sliders" => typeof(UiSlidersCaptureScenario),
                "ui-ranges" => typeof(UiRangesCaptureScenario),
                "ui-parts" => typeof(UiPartsCaptureScenario),
                "ui-complex-parts-before" => typeof(UiComplexPartsBeforeCaptureScenario),
                "ui-complex-parts-after" => typeof(UiComplexPartsAfterCaptureScenario),
                "ui-pointer-routing" => typeof(UiPointerRoutingCaptureScenario),
                "ui-keyboard-navigation" => typeof(UiKeyboardNavigationCaptureScenario),
                "ui-remaining-link" => typeof(UiRemainingLinkCaptureScenario),
                "ui-remaining-lifecycle" => typeof(UiRemainingLifecycleCaptureScenario),
                "ui-actions-console" => typeof(UiActionsConsoleCaptureScenario),
                "ui-input-cleanup" => typeof(UiInputCleanupCaptureScenario),
                "ui-panel-target" => typeof(UiPanelTargetCaptureScenario),
                "ui-world-space" => typeof(UiWorldSpaceCaptureScenario),
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
            importer.SetCompatibleWithPlatform(NativeBuildTarget, true);
            importer.SetPlatformData(NativeBuildTarget, "CPU", "AnyCPU");
            importer.SaveAndReimport();
        }

        private static string Required(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
