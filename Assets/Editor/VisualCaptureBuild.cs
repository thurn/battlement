#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using Battlement.VisualCapture;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Battlement.Editor
{
    public static class VisualCaptureBuild
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
            string outputPath = RequiredEnvironmentVariable("BATTLEMENT_CAPTURE_BUILD_PATH");
            string scenePath = RequiredEnvironmentVariable("BATTLEMENT_CAPTURE_SCENE_PATH");
            string scenarioName = RequiredEnvironmentVariable("BATTLEMENT_CAPTURE_SCENARIO");
            ValidateScene(scenePath, scenarioName);
            ValidateReusableAssets(scenePath);
            IntegrationFixtureAssets.Validate();
            IntegrationFixtureAssets.BuildCatalog();
            ConfigurePluginWhenPresent();

            var options = new BuildPlayerOptions
            {
                scenes = new[] { scenePath },
                locationPathName = outputPath,
                target = NativeBuildTarget,
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

            Debug.Log($"BATTLEMENT_CAPTURE_BUILD_OK:{outputPath}");
        }

        private static void ValidateScene(string scenePath, string scenarioName)
        {
            SceneAsset captureScene = AssetDatabase.LoadAssetAtPath<SceneAsset>(scenePath);
            if (!captureScene)
            {
                throw new InvalidOperationException($"Capture scene was not found: {scenePath}");
            }

            EditorSceneManager.OpenScene(scenePath, OpenSceneMode.Single);
            BattlementCaptureScenario[] matches = UnityEngine
                .Object.FindObjectsByType<BattlementCaptureScenario>(FindObjectsInactive.Include)
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
            importer.SetCompatibleWithPlatform(NativeBuildTarget, true);
            importer.SetPlatformData(NativeBuildTarget, "CPU", "AnyCPU");
            importer.SaveAndReimport();
        }

        private static void ValidateReusableAssets(string scenePath)
        {
            Shader shader = RequiredAsset<Shader>(VisualCaptureAssets.ShaderPath);
            if (!shader.isSupported || shader.name != "Battlement/Visual Capture Unlit")
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
            BattlementCaptureShell[] shells =
                UnityEngine.Object.FindObjectsByType<BattlementCaptureShell>(
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
