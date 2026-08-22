#nullable enable

using System;
using Battlement.VisualCapture;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Battlement.Editor
{
    public static class VisualCaptureScaffold
    {
        public static void CreateScene()
        {
            string scenePath = RequiredEnvironmentVariable("BATTLEMENT_CAPTURE_SCAFFOLD_SCENE");
            string scriptPath = RequiredEnvironmentVariable("BATTLEMENT_CAPTURE_SCAFFOLD_SCRIPT");
            string typeName = RequiredEnvironmentVariable("BATTLEMENT_CAPTURE_SCAFFOLD_TYPE");
            MonoScript script = AssetDatabase.LoadAssetAtPath<MonoScript>(scriptPath);
            Type? scenarioType = script ? script.GetClass() : null;
            if (
                scenarioType is null
                || scenarioType.Name != typeName
                || !typeof(BattlementCaptureScenario).IsAssignableFrom(scenarioType)
            )
            {
                throw new InvalidOperationException(
                    $"Scenario script does not define {typeName}: {scriptPath}"
                );
            }

            GameObject shellPrefab = AssetDatabase.LoadAssetAtPath<GameObject>(
                VisualCaptureAssets.ShellPrefabPath
            );
            if (!shellPrefab)
            {
                throw new InvalidOperationException(
                    $"Reusable capture shell is missing: {VisualCaptureAssets.ShellPrefabPath}"
                );
            }

            Scene scene = EditorSceneManager.NewScene(
                NewSceneSetup.EmptyScene,
                NewSceneMode.Single
            );
            PrefabUtility.InstantiatePrefab(shellPrefab, scene);
            GameObject scenarioObject = new(typeName);
            SceneManager.MoveGameObjectToScene(scenarioObject, scene);
            scenarioObject.AddComponent(scenarioType);
            if (!EditorSceneManager.SaveScene(scene, scenePath))
            {
                throw new InvalidOperationException($"Could not save capture scene: {scenePath}");
            }

            AssetDatabase.SaveAssets();
            Debug.Log($"BATTLEMENT_CAPTURE_SCAFFOLD_OK:{scenePath}");
        }

        private static string RequiredEnvironmentVariable(string name) =>
            Environment.GetEnvironmentVariable(name)
            ?? throw new InvalidOperationException($"{name} must be set.");
    }
}
