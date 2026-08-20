#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Masonry.VisualCapture;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Masonry.Editor
{
    public static class VisualCaptureBuild
    {
        private const string PluginPath = "Assets/Plugins/macOS/libmasonry_rules.dylib";

        public static void Build()
        {
            string outputPath = RequiredEnvironmentVariable("MASONRY_CAPTURE_BUILD_PATH");
            string scenePath = RequiredEnvironmentVariable("MASONRY_CAPTURE_SCENE_PATH");
            string scenarioName = RequiredEnvironmentVariable("MASONRY_CAPTURE_SCENARIO");
            ValidateScene(scenePath, scenarioName);
            ValidateReusableAssets(scenePath);
            IntegrationFixtureAssets.Validate();
            IntegrationFixtureAssets.BuildCatalog();
            ConfigurePluginWhenPresent();

            var options = new BuildPlayerOptions
            {
                scenes = new[] { scenePath },
                locationPathName = outputPath,
                target = BuildTarget.StandaloneOSX,
                options = BuildOptions.None,
            };
            BuildReport report = BuildPipeline.BuildPlayer(options);
            if (report.summary.result != BuildResult.Succeeded)
            {
                throw new InvalidOperationException(
                    $"Release player build failed: {report.summary.result} "
                        + $"({report.summary.totalErrors} errors)."
                );
            }

            Debug.Log($"MASONRY_CAPTURE_BUILD_OK:{outputPath}");
        }

        private static void ValidateScene(string scenePath, string scenarioName)
        {
            SceneAsset captureScene = AssetDatabase.LoadAssetAtPath<SceneAsset>(scenePath);
            if (!captureScene)
            {
                throw new InvalidOperationException($"Capture scene was not found: {scenePath}");
            }

            EditorSceneManager.OpenScene(scenePath, OpenSceneMode.Single);
            MasonryCaptureScenario[] matches = UnityEngine
                .Object.FindObjectsByType<MasonryCaptureScenario>(FindObjectsInactive.Include)
                .Where(scenario =>
                    string.Equals(scenario.ScenarioName, scenarioName, StringComparison.Ordinal)
                )
                .ToArray();
            if (matches.Length != 1)
            {
                throw new InvalidOperationException(
                    $"Capture scene must contain exactly one '{scenarioName}' scenario; "
                        + $"found {matches.Length}."
                );
            }
        }

        private static void ConfigurePluginWhenPresent()
        {
            if (!System.IO.File.Exists(PluginPath))
            {
                return;
            }

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

        private static void ValidateReusableAssets(string scenePath)
        {
            Shader shader = RequiredAsset<Shader>(VisualCaptureAssets.ShaderPath);
            if (!shader.isSupported || shader.name != "Masonry/Visual Capture Unlit")
            {
                throw new InvalidOperationException(
                    $"Reusable capture shader is unsupported: {VisualCaptureAssets.ShaderPath}"
                );
            }

            string[] materialPaths =
            {
                VisualCaptureAssets.PrimaryMaterialPath,
                VisualCaptureAssets.AccentMaterialPath,
                VisualCaptureAssets.SuccessMaterialPath,
            };
            foreach (string materialPath in materialPaths)
            {
                Material material = RequiredAsset<Material>(materialPath);
                if (material.shader != shader)
                {
                    throw new InvalidOperationException(
                        $"Capture material does not reference {VisualCaptureAssets.ShaderPath}: "
                            + materialPath
                    );
                }
            }

            RequiredAsset<GameObject>(VisualCaptureAssets.ShellPrefabPath);
            MasonryCaptureShell[] shells =
                UnityEngine.Object.FindObjectsByType<MasonryCaptureShell>(
                    FindObjectsInactive.Include
                );
            if (shells.Length == 0)
            {
                return;
            }

            HashSet<string> dependencies = AssetDatabase
                .GetDependencies(scenePath, true)
                .ToHashSet(StringComparer.Ordinal);
            string[] requiredDependencies = materialPaths
                .Append(VisualCaptureAssets.ShaderPath)
                .Append(VisualCaptureAssets.ShellPrefabPath)
                .ToArray();
            foreach (string requiredPath in requiredDependencies)
            {
                if (!dependencies.Contains(requiredPath))
                {
                    throw new InvalidOperationException(
                        $"Capture scene shell is missing required asset reference: {requiredPath}"
                    );
                }
            }
        }

        private static T RequiredAsset<T>(string path)
            where T : UnityEngine.Object
        {
            T asset = AssetDatabase.LoadAssetAtPath<T>(path);
            if (!asset)
            {
                throw new InvalidOperationException($"Required capture asset is missing: {path}");
            }

            return asset;
        }

        private static string RequiredEnvironmentVariable(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
