#nullable enable

using System;
using System.IO;
using Masonry.VisualCapture;
using UnityEditor;
using UnityEngine;
using UnityColor = UnityEngine.Color;
using UnityQuaternion = UnityEngine.Quaternion;
using UnityVector3 = UnityEngine.Vector3;

namespace Masonry.Editor
{
    public static class VisualCaptureAssets
    {
        public const string AccentMaterialPath = "Assets/VisualCapture/Materials/CaptureAccent.mat";
        public const string PrimaryMaterialPath =
            "Assets/VisualCapture/Materials/CapturePrimary.mat";
        public const string ShaderPath = "Assets/VisualCapture/VisualCaptureUnlit.shader";
        public const string ShellPrefabPath = "Assets/VisualCapture/MasonryCaptureShell.prefab";
        public const string SuccessMaterialPath =
            "Assets/VisualCapture/Materials/CaptureSuccess.mat";

        [MenuItem("Masonry/Visual Capture/Rebuild Reusable Assets")]
        public static void Rebuild()
        {
            Shader shader = AssetDatabase.LoadAssetAtPath<Shader>(ShaderPath);
            if (!shader)
            {
                throw new InvalidOperationException($"Capture shader is missing: {ShaderPath}");
            }

            Directory.CreateDirectory("Assets/VisualCapture/Materials");
            Material primary = CreateMaterial(
                PrimaryMaterialPath,
                shader,
                new UnityColor(0.12f, 0.32f, 0.58f)
            );
            Material accent = CreateMaterial(
                AccentMaterialPath,
                shader,
                new UnityColor(0.15f, 0.72f, 0.91f)
            );
            Material success = CreateMaterial(
                SuccessMaterialPath,
                shader,
                new UnityColor(0.18f, 0.72f, 0.42f)
            );
            CreateShellPrefab(primary, accent, success);
            AssetDatabase.SaveAssets();
            AssetDatabase.Refresh();
            Debug.Log("MASONRY_CAPTURE_ASSETS_OK");
        }

        private static Material CreateMaterial(string path, Shader shader, UnityColor color)
        {
            Material? material = AssetDatabase.LoadAssetAtPath<Material>(path);
            if (!material)
            {
                material = new Material(shader);
                AssetDatabase.CreateAsset(material, path);
            }

            material.shader = shader;
            material.SetColor("_BaseColor", color);
            EditorUtility.SetDirty(material);
            return material;
        }

        private static void CreateShellPrefab(Material primary, Material accent, Material success)
        {
            GameObject root = new("Masonry Capture Shell");
            try
            {
                MasonryCaptureShell shell = root.AddComponent<MasonryCaptureShell>();
                Camera camera = CreateCamera(root.transform);
                Light light = CreateLight(root.transform);
                CreateSwatch(root.transform, "Primary", new UnityVector3(-2.4f, -0.6f, 0), primary);
                CreateSwatch(root.transform, "Accent", new UnityVector3(0, -0.6f, 0), accent);
                CreateSwatch(root.transform, "Success", new UnityVector3(2.4f, -0.6f, 0), success);
                SerializedObject serialized = new(shell);
                serialized.FindProperty("captureCamera").objectReferenceValue = camera;
                serialized.FindProperty("keyLight").objectReferenceValue = light;
                serialized.FindProperty("primaryMaterial").objectReferenceValue = primary;
                serialized.FindProperty("accentMaterial").objectReferenceValue = accent;
                serialized.FindProperty("successMaterial").objectReferenceValue = success;
                serialized.FindProperty("legend").arraySize = 2;
                serialized.FindProperty("legend").GetArrayElementAtIndex(0).stringValue =
                    "Stable initial frame";
                serialized.FindProperty("legend").GetArrayElementAtIndex(1).stringValue =
                    "Scenario-owned interaction";
                serialized.ApplyModifiedPropertiesWithoutUndo();
                PrefabUtility.SaveAsPrefabAsset(root, ShellPrefabPath);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(root);
            }
        }

        private static Camera CreateCamera(Transform parent)
        {
            GameObject cameraObject = new("Capture Camera");
            cameraObject.transform.SetParent(parent);
            cameraObject.transform.SetPositionAndRotation(
                new UnityVector3(0, 0, -10),
                UnityQuaternion.identity
            );
            return cameraObject.AddComponent<Camera>();
        }

        private static Light CreateLight(Transform parent)
        {
            GameObject lightObject = new("Capture Key Light");
            lightObject.transform.SetParent(parent);
            lightObject.transform.rotation = UnityQuaternion.Euler(40, -30, 0);
            return lightObject.AddComponent<Light>();
        }

        private static void CreateSwatch(
            Transform parent,
            string name,
            UnityVector3 position,
            Material material
        )
        {
            GameObject swatch = GameObject.CreatePrimitive(PrimitiveType.Cube);
            swatch.name = name;
            swatch.transform.SetParent(parent);
            swatch.transform.localPosition = position;
            swatch.transform.localScale = new UnityVector3(1.8f, 1.8f, 0.3f);
            swatch.GetComponent<MeshRenderer>().sharedMaterial = material;
        }
    }
}
