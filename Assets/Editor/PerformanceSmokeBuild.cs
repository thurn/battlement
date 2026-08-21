#nullable enable

using System;
using Masonry.Performance;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Masonry.Editor
{
    public static class PerformanceSmokeBuild
    {
        public const string ScenePath = "Assets/MasonryPerformance/MasonryPerformanceSmoke.unity";

        public static void Build()
        {
            string outputPath =
                Environment.GetEnvironmentVariable("MASONRY_PERFORMANCE_BUILD_PATH")
                ?? throw new InvalidOperationException(
                    "MASONRY_PERFORMANCE_BUILD_PATH must be set."
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
            Debug.Log($"MASONRY_PERFORMANCE_BUILD_OK:{outputPath}");
        }

        public static void CreateScene()
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var host = new GameObject("Masonry Performance Smoke");
            MasonryRunner runner = host.AddComponent<MasonryRunner>();
            MasonryPerformanceSmoke smoke = host.AddComponent<MasonryPerformanceSmoke>();
            var serialized = new SerializedObject(smoke);
            serialized.FindProperty("runner").objectReferenceValue = runner;
            serialized.ApplyModifiedPropertiesWithoutUndo();
            EditorSceneManager.SaveScene(scene, ScenePath);
            Debug.Log($"MASONRY_PERFORMANCE_SCENE_OK:{ScenePath}");
        }
    }
}
