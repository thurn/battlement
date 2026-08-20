#nullable enable

using System;
using System.IO;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Build;
using UnityEditor.AddressableAssets.Build.DataBuilders;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.AddressableAssets.Settings.GroupSchemas;
using UnityEditor.Build.Reporting;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Masonry.BasicSample.Editor
{
    /// <summary>Generates sample assets and builds its standalone macOS player.</summary>
    public static class BasicSampleBuild
    {
        private const string Generated = "Assets/Generated";
        private const string BootstrapScene = Generated + "/BasicSample.unity";
        private const string ContentScene = Generated + "/Content.unity";
        private const string Plugin = "Assets/Plugins/macOS/libmasonry_rules.dylib";
        private const string GroupName = "Masonry Basic Sample";

        public static void Build()
        {
            GenerateAssets();
            BuildAddressables();
            ConfigurePlugin();
            PlayerSettings.runInBackground = true;
            string output =
                Environment.GetEnvironmentVariable("MASONRY_SAMPLE_BUILD_PATH")
                ?? Required("MASONRY_CAPTURE_BUILD_PATH");
            bool release =
                Environment.GetEnvironmentVariable("MASONRY_SAMPLE_RELEASE") == "1"
                || Environment.GetEnvironmentVariable("MASONRY_CAPTURE_BUILD_PATH") != null;
            var options = new BuildPlayerOptions
            {
                scenes = new[] { BootstrapScene },
                locationPathName = output,
                target = BuildTarget.StandaloneOSX,
                options = release ? BuildOptions.None : BuildOptions.Development,
            };
            BuildReport report = BuildPipeline.BuildPlayer(options);
            if (report.summary.result != BuildResult.Succeeded)
            {
                throw new InvalidOperationException(
                    $"Basic sample build failed with {report.summary.totalErrors} errors."
                );
            }
            EditorBuildSettings.RemoveConfigObject(
                AddressableAssetSettingsDefaultObject.kDefaultConfigObjectName
            );
            Debug.Log($"MASONRY_SAMPLE_BUILD_OK:{output}");
            if (Environment.GetEnvironmentVariable("MASONRY_CAPTURE_BUILD_PATH") != null)
            {
                Debug.Log($"MASONRY_CAPTURE_BUILD_OK:{output}");
            }
        }

        private static void GenerateAssets()
        {
            EnsureFolder("Assets", "Generated");
            CreateMaterial("Gray", new UnityEngine.Color(0.24f, 0.29f, 0.38f));
            CreateMaterial("Yellow", new UnityEngine.Color(1f, 0.72f, 0.08f));
            CreateMaterial("Blue", new UnityEngine.Color(0.05f, 0.48f, 1f));
            CreateMaterial("Marker", new UnityEngine.Color(0.10f, 0.14f, 0.22f));

            Scene content = EditorSceneManager.NewScene(
                NewSceneSetup.EmptyScene,
                NewSceneMode.Single
            );
            var floor = GameObject.CreatePrimitive(PrimitiveType.Plane);
            floor.name = "Authored Position Markers";
            floor.transform.position = new UnityEngine.Vector3(0, -1.05f, 1);
            floor.transform.localScale = new UnityEngine.Vector3(0.65f, 1, 0.45f);
            floor.GetComponent<Renderer>().sharedMaterial = LoadMaterial("Gray");
            for (int cube = 0; cube < 3; cube++)
            {
                for (int position = 0; position < 2; position++)
                {
                    var marker = GameObject.CreatePrimitive(PrimitiveType.Cube);
                    marker.name = $"Cube {(char)('A' + cube)} Position {position + 1}";
                    marker.transform.position = new UnityEngine.Vector3(
                        -2 + cube * 2,
                        -0.94f,
                        position * 2
                    );
                    marker.transform.localScale = new UnityEngine.Vector3(1.1f, 0.08f, 1.1f);
                    marker.GetComponent<Renderer>().sharedMaterial = LoadMaterial("Marker");
                }
            }
            EditorSceneManager.SaveScene(content, ContentScene);

            Scene bootstrap = EditorSceneManager.NewScene(
                NewSceneSetup.EmptyScene,
                NewSceneMode.Single
            );
            var root = new GameObject("Basic Sample Bootstrap — Game Owned");
            root.AddComponent<BasicSample>();
            root.AddComponent<BasicSampleCaptureScenario>();
            EditorSceneManager.SaveScene(bootstrap, BootstrapScene);
            ConfigureAddressables();
            AssetDatabase.SaveAssets();
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
        }

        private static void CreateMaterial(string name, UnityEngine.Color color)
        {
            string path = $"{Generated}/{name}.mat";
            AssetDatabase.DeleteAsset(path);
            Shader shader = Shader.Find("Unlit/Color");
            if (!shader)
            {
                shader = Shader.Find("Universal Render Pipeline/Unlit");
            }
            if (!shader)
            {
                throw new InvalidOperationException("No supported unlit shader was found.");
            }
            AssetDatabase.CreateAsset(new Material(shader) { name = name, color = color }, path);
        }

        private static Material LoadMaterial(string name) =>
            AssetDatabase.LoadAssetAtPath<Material>($"{Generated}/{name}.mat");

        private static void ConfigureAddressables()
        {
            AddressableAssetSettings settings = AddressableAssetSettingsDefaultObject.GetSettings(
                true
            );
            AddressableAssetGroup group = settings.FindGroup(GroupName);
            if (!group)
            {
                group = settings.CreateGroup(
                    GroupName,
                    false,
                    false,
                    false,
                    null,
                    typeof(BundledAssetGroupSchema),
                    typeof(ContentUpdateGroupSchema)
                );
            }
            Address(settings, group, ContentScene, "basic/content");
            Address(settings, group, $"{Generated}/Gray.mat", "basic/material/gray");
            Address(settings, group, $"{Generated}/Yellow.mat", "basic/material/yellow");
            Address(settings, group, $"{Generated}/Blue.mat", "basic/material/blue");
            int packed = settings.DataBuilders.FindIndex(builder =>
                builder is BuildScriptPackedMode
            );
            if (packed < 0)
            {
                throw new InvalidOperationException("Addressables has no packed-mode builder.");
            }
            settings.ActivePlayerDataBuilderIndex = packed;
            settings.SetDirty(AddressableAssetSettings.ModificationEvent.EntryMoved, null, true);
        }

        private static void Address(
            AddressableAssetSettings settings,
            AddressableAssetGroup group,
            string path,
            string address
        ) =>
            settings.CreateOrMoveEntry(AssetDatabase.AssetPathToGUID(path), group).address =
                address;

        private static void BuildAddressables()
        {
            AddressableAssetSettings.CleanPlayerContent();
            AddressableAssetSettings.BuildPlayerContent(out AddressablesPlayerBuildResult result);
            if (!string.IsNullOrEmpty(result.Error))
            {
                throw new InvalidOperationException(result.Error);
            }
        }

        private static void ConfigurePlugin()
        {
            AssetDatabase.ImportAsset(Plugin, ImportAssetOptions.ForceSynchronousImport);
            if (AssetImporter.GetAtPath(Plugin) is not PluginImporter importer)
            {
                throw new InvalidOperationException($"Native plugin was not imported: {Plugin}");
            }
            importer.SetCompatibleWithAnyPlatform(false);
            importer.SetCompatibleWithEditor(false);
            importer.SetCompatibleWithPlatform(BuildTarget.StandaloneOSX, true);
            importer.SetPlatformData(BuildTarget.StandaloneOSX, "CPU", "AnyCPU");
            importer.SaveAndReimport();
        }

        private static void EnsureFolder(string parent, string child)
        {
            string path = $"{parent}/{child}";
            if (!AssetDatabase.IsValidFolder(path))
            {
                AssetDatabase.CreateFolder(parent, child);
            }
        }

        private static string Required(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
