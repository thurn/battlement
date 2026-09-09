#nullable enable

using System;
using System.Collections;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using UnityEngine;
using UnityEngine.AddressableAssets;
using UnityEngine.Rendering;
using UnityEngine.ResourceManagement.AsyncOperations;
using UnityEngine.ResourceManagement.ResourceProviders;
using UnityEngine.SceneManagement;

namespace Battlement
{
    /// <summary>Loads sample content into the reusable macOS player shell.</summary>
    public sealed class BattlementShellBootstrap : MonoBehaviour
    {
        private const string RenderPipelineAddress = "__battlement_render_pipeline";
        private const string StartupSceneAddress = "__battlement_startup";

        private AsyncOperationHandle<RenderPipelineAsset> _pipeline;
        private bool _hasPipeline;
        private RenderPipelineGlobalSettings? _pipelineGlobalSettings;

        internal static string? AssemblyIdentityJson { get; private set; }
        internal static string? ReactantAssetCatalogJson { get; private set; }

        private void Awake()
        {
            _pipelineGlobalSettings =
                typeof(GraphicsSettings)
                    .GetProperty(
                        "currentRenderPipelineGlobalSettings",
                        BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static
                    )
                    ?.GetValue(null) as RenderPipelineGlobalSettings;
            GraphicsSettings.defaultRenderPipeline = null;
            QualitySettings.renderPipeline = null;
            string path = FindBattlementFile("assembly.json");
            if (!File.Exists(path))
            {
                Debug.LogError(
                    $"Battlement shell could not find assembly.json from '{Application.dataPath}'."
                );
                return;
            }
            AssemblyIdentityJson = File.ReadAllText(path);
            string assetCatalog = FindBattlementFile("reactant-assets.json");
            ReactantAssetCatalogJson = File.Exists(assetCatalog)
                ? File.ReadAllText(assetCatalog)
                : null;
            Debug.Log($"[Battlement/Shell] assembly={path}");
            DontDestroyOnLoad(gameObject);
        }

        private static string FindBattlementFile(string name)
        {
            string[] candidates =
            {
                Path.Combine(Application.dataPath, "Resources", "Battlement", name),
                Path.Combine(Application.dataPath, "..", "Battlement", name),
            };
            foreach (string candidate in candidates)
            {
                string path = Path.GetFullPath(candidate);
                if (File.Exists(path))
                {
                    return path;
                }
            }
            return Path.GetFullPath(candidates[0]);
        }

        private IEnumerator Start()
        {
            try
            {
                Addressables.InitializeAsync().WaitForCompletion();
            }
            catch (Exception exception)
            {
                Debug.LogError(
                    $"Battlement shell could not initialize Addressables: {exception.Message}"
                );
                yield break;
            }
            Debug.Log("[Battlement/Shell] addressables=initialized");
            AsyncOperationHandle<
                IList<UnityEngine.ResourceManagement.ResourceLocations.IResourceLocation>
            > locations = Addressables.LoadResourceLocationsAsync(
                RenderPipelineAddress,
                typeof(RenderPipelineAsset)
            );
            yield return locations;
            RenderPipelineAsset? pipeline = null;
            if (locations.Status == AsyncOperationStatus.Succeeded && locations.Result.Count > 0)
            {
                if (_pipelineGlobalSettings == null || !RestoreRenderPipelineGlobalSettings())
                {
                    Debug.LogError(
                        "Battlement shell could not restore render pipeline global settings."
                    );
                    Addressables.Release(locations);
                    yield break;
                }
                _pipeline = Addressables.LoadAssetAsync<RenderPipelineAsset>(locations.Result[0]);
                yield return _pipeline;
                if (_pipeline.Status != AsyncOperationStatus.Succeeded)
                {
                    string detail =
                        _pipeline.OperationException?.Message ?? "unknown Addressables error";
                    Debug.LogError(
                        $"Battlement shell could not load '{RenderPipelineAddress}': {detail}"
                    );
                    Addressables.Release(locations);
                    yield break;
                }
                _hasPipeline = true;
                pipeline = _pipeline.Result;
            }
            Addressables.Release(locations);
            GraphicsSettings.defaultRenderPipeline = pipeline;
            QualitySettings.renderPipeline = pipeline;
            string pipelineName = pipeline == null ? "builtin" : pipeline.name;
            Debug.Log($"[Battlement/Shell] render-pipeline={pipelineName}");
            Debug.Log($"[Battlement/Shell] loading={StartupSceneAddress}");
            AsyncOperationHandle<SceneInstance> load = Addressables.LoadSceneAsync(
                StartupSceneAddress,
                LoadSceneMode.Single,
                true
            );
            yield return load;
            if (load.Status != AsyncOperationStatus.Succeeded)
            {
                string detail = load.OperationException?.Message ?? "unknown Addressables error";
                Debug.LogError(
                    $"Battlement shell could not load '{StartupSceneAddress}': {detail}"
                );
                enabled = false;
                yield break;
            }
            Debug.Log($"[Battlement/Shell] loaded={StartupSceneAddress}");
        }

        private bool RestoreRenderPipelineGlobalSettings()
        {
            MethodInfo? restore = typeof(GraphicsSettings).GetMethod(
                "TrySetCurrentRenderPipelineGlobalSettings",
                BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Static
            );
            return restore?.Invoke(null, new object[] { _pipelineGlobalSettings! }) as bool?
                ?? false;
        }

        private void OnDestroy()
        {
            if (_hasPipeline)
            {
                Addressables.Release(_pipeline);
            }
        }
    }
}
