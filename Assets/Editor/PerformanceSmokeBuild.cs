#nullable enable

using System;
using Battlement.Performance;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Battlement.Editor
{
    public static class PerformanceSmokeBuild
    {
        public const string ScenePath =
            "Assets/BattlementPerformance/BattlementPerformanceSmoke.unity";

        public static void Build()
        {
            string outputPath =
                Environment.GetEnvironmentVariable("BATTLEMENT_PERFORMANCE_BUILD_PATH")
                ?? throw new InvalidOperationException(
                    "BATTLEMENT_PERFORMANCE_BUILD_PATH must be set."
                );
            IntegrationFixtureAssets.Validate();
            IntegrationFixtureAssets.BuildCatalog();
            var options = new BuildPlayerOptions
            {
                scenes = new[] { ScenePath },
                locationPathName = outputPath,
                target = BuildTarget.StandaloneOSX,
                options = BuildOptions.Development,
            };
            BuildReport report = BuildPipeline.BuildPlayer(options);
            if (report.summary.result != BuildResult.Succeeded)
            {
                throw new InvalidOperationException(
                    $"Performance player build failed: {report.summary.result}."
                );
            }
            Debug.Log($"BATTLEMENT_PERFORMANCE_BUILD_OK:{outputPath}");
        }

        public static void CreateScene()
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var host = new GameObject("Battlement Performance Smoke");
            BattlementRunner runner = host.AddComponent<BattlementRunner>();
            BattlementPerformanceSmoke smoke = host.AddComponent<BattlementPerformanceSmoke>();
            var serialized = new SerializedObject(smoke);
            serialized.FindProperty("runner").objectReferenceValue = runner;
            serialized.ApplyModifiedPropertiesWithoutUndo();
            EditorSceneManager.SaveScene(scene, ScenePath);
            Debug.Log($"BATTLEMENT_PERFORMANCE_SCENE_OK:{ScenePath}");
        }
    }
}
