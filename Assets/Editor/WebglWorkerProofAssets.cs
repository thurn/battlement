#nullable enable

using Battlement.Integration;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>Generates the committed threaded WebGL proof scene.</summary>
    public static class WebglWorkerProofAssets
    {
        /// <summary>Recreates the deterministic worker-proof bootstrap scene.</summary>
        public static void Generate()
        {
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            var host = new GameObject("Battlement threaded WebGL proof");
            BattlementRunner runner = host.AddComponent<BattlementRunner>();
            BattlementWebglWorkerProof fixture = host.AddComponent<BattlementWebglWorkerProof>();
            var serialized = new SerializedObject(fixture);
            serialized.FindProperty("runner").objectReferenceValue = runner;
            serialized.ApplyModifiedPropertiesWithoutUndo();
            EditorSceneManager.SaveScene(scene, BattlementWebglWorkerProof.ScenePath);
            AssetDatabase.SaveAssets();
            Debug.Log("BATTLEMENT_WEBGL_WORKER_ASSETS_GENERATED");
        }
    }
}
