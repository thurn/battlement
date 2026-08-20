#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Masonry.Integration;
using TMPro;
using UnityEditor;
using UnityEditor.AddressableAssets;
using UnityEditor.AddressableAssets.Build;
using UnityEditor.AddressableAssets.Build.DataBuilders;
using UnityEditor.AddressableAssets.Settings;
using UnityEditor.AddressableAssets.Settings.GroupSchemas;
using UnityEditor.Animations;
using UnityEditor.SceneManagement;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace Masonry.Editor
{
    /// <summary>Generates and validates the committed real-content fixture assets.</summary>
    public static class IntegrationFixtureAssets
    {
        public const string BootstrapScenePath = MasonryIntegrationFixture.BootstrapScenePath;
        public const string ContentScenePath =
            "Assets/MasonryIntegration/Content/IntegrationContent.unity";

        private const string GeneratedPath = "Assets/MasonryIntegration/Content";
        private const string PrefabPath = GeneratedPath + "/IntegrationAnimatedPrefab.prefab";
        private const string EffectPath = GeneratedPath + "/IntegrationEffect.prefab";
        private const string MaterialPath = GeneratedPath + "/IntegrationMaterial.mat";
        private const string TexturePath = GeneratedPath + "/IntegrationTexture.asset";
        private const string AudioPath = GeneratedPath + "/IntegrationAudio.asset";
        private const string AnimationPath = GeneratedPath + "/IntegrationIdle.anim";
        private const string ControllerPath = GeneratedPath + "/Integration.controller";
        private const string FontPath =
            "Assets/TextMesh Pro/Resources/Fonts & Materials/LiberationSans SDF.asset";
        private const string GroupName = "Masonry Integration Fixture";

        private static readonly IReadOnlyDictionary<string, (string Path, Type Type)> Entries =
            new Dictionary<string, (string, Type)>
            {
                [MasonryIntegrationFixture.SceneAddress] = (ContentScenePath, typeof(SceneAsset)),
                [MasonryIntegrationFixture.PrefabAddress] = (PrefabPath, typeof(GameObject)),
                [MasonryIntegrationFixture.EffectAddress] = (EffectPath, typeof(GameObject)),
                [MasonryIntegrationFixture.MaterialAddress] = (MaterialPath, typeof(Material)),
                [MasonryIntegrationFixture.TextureAddress] = (TexturePath, typeof(Texture)),
                [MasonryIntegrationFixture.AudioAddress] = (AudioPath, typeof(AudioClip)),
                [MasonryIntegrationFixture.FontAddress] = (FontPath, typeof(TMP_FontAsset)),
            };

        /// <summary>Creates the deterministic Unity and Addressables fixture assets.</summary>
        public static void Generate()
        {
            EnsureFolder("Assets/MasonryIntegration");
            EnsureFolder(GeneratedPath);
            DeleteGeneratedAssets();
            CreateMaterialAndTexture();
            CreateAudio();
            CreateAnimatedPrefab();
            CreateEffectPrefab();
            CreateContentScene();
            CreateBootstrapScene();
            ConfigureAddressables();
            AssetDatabase.SaveAssets();
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
            Validate();
            Debug.Log("MASONRY_INTEGRATION_ASSETS_GENERATED");
        }

        /// <summary>Validates asset types, component roots, registration, and fixtures.</summary>
        public static void Validate()
        {
            AddressableAssetSettings settings = AddressableAssetSettingsDefaultObject.Settings;
            if (!settings)
            {
                throw new InvalidOperationException(
                    "Addressables settings are missing for the Masonry Integration Fixture."
                );
            }
            foreach ((string address, (string path, Type type)) in Entries)
            {
                string guid = AssetDatabase.AssetPathToGUID(path);
                AddressableAssetEntry entry = settings.FindAssetEntry(guid);
                if (
                    entry == null
                    || !string.Equals(entry.address, address, StringComparison.Ordinal)
                )
                {
                    throw new InvalidOperationException(
                        $"Addressable '{address}' is missing or does not point to '{path}'."
                    );
                }

                Type actual = AssetDatabase.GetMainAssetTypeAtPath(path);
                if (actual == null || !type.IsAssignableFrom(actual))
                {
                    throw new InvalidOperationException(
                        $"Addressable '{address}' at '{path}' must resolve as {type.Name}; "
                            + $"found {actual?.Name ?? "no asset"}."
                    );
                }
            }

            ValidatePrefabRoots();
            ValidateBootstrap();
            ValidateProtocolFixture();
        }

        /// <summary>Builds a clean catalog or throws an actionable diagnostic.</summary>
        public static void BuildCatalog()
        {
            AddressableAssetSettings settings = AddressableAssetSettingsDefaultObject.Settings;
            SelectPackedBuilder(settings);
            SelectFastPlayMode(settings);
            AddressableAssetSettings.CleanPlayerContent();
            AddressableAssetSettings.BuildPlayerContent(out AddressablesPlayerBuildResult result);
            if (!string.IsNullOrEmpty(result.Error))
            {
                throw new InvalidOperationException(
                    $"Masonry Integration Fixture Addressables build failed: {result.Error}"
                );
            }
        }

        private static void DeleteGeneratedAssets()
        {
            foreach (
                string path in new[]
                {
                    PrefabPath,
                    EffectPath,
                    MaterialPath,
                    TexturePath,
                    AudioPath,
                    AnimationPath,
                    ControllerPath,
                    ContentScenePath,
                    BootstrapScenePath,
                }
            )
            {
                AssetDatabase.DeleteAsset(path);
            }
        }

        private static void CreateMaterialAndTexture()
        {
            Shader shader = Shader.Find("Universal Render Pipeline/Lit");
            if (!shader)
            {
                throw new InvalidOperationException("The fixture requires the URP Lit shader.");
            }
            var material = new Material(shader)
            {
                color = new UnityEngine.Color(0.08f, 0.55f, 0.88f),
            };
            AssetDatabase.CreateAsset(material, MaterialPath);

            var texture = new Texture2D(8, 8, TextureFormat.RGBA32, false)
            {
                name = "Integration Texture",
                filterMode = FilterMode.Point,
            };
            for (var y = 0; y < texture.height; y++)
            {
                for (var x = 0; x < texture.width; x++)
                {
                    bool accent = (x + y) % 2 == 0;
                    texture.SetPixel(
                        x,
                        y,
                        accent
                            ? new UnityEngine.Color(0.1f, 0.8f, 0.95f)
                            : new UnityEngine.Color(0.03f, 0.08f, 0.2f)
                    );
                }
            }
            texture.Apply();
            AssetDatabase.CreateAsset(texture, TexturePath);
        }

        private static void CreateAudio()
        {
            AudioClip clip = AudioClip.Create("Integration Chime", 2205, 1, 22050, false);
            var samples = new float[2205];
            for (var index = 0; index < samples.Length; index++)
            {
                samples[index] = Mathf.Sin(index * 0.08f) * (1 - (index / 2205f)) * 0.15f;
            }
            clip.SetData(samples, 0);
            AssetDatabase.CreateAsset(clip, AudioPath);
        }

        private static void CreateAnimatedPrefab()
        {
            var clip = new AnimationClip { name = "Integration Idle" };
            AssetDatabase.CreateAsset(clip, AnimationPath);
            AnimatorController controller = AnimatorController.CreateAnimatorControllerAtPath(
                ControllerPath
            );
            controller.layers[0].stateMachine.AddState("Idle").motion = clip;

            GameObject prefab = GameObject.CreatePrimitive(PrimitiveType.Cube);
            prefab.name = "Integration Animated Prefab";
            prefab.GetComponent<Renderer>().sharedMaterial =
                AssetDatabase.LoadAssetAtPath<Material>(MaterialPath);
            prefab.AddComponent<Animator>().runtimeAnimatorController = controller;
            PrefabUtility.SaveAsPrefabAsset(prefab, PrefabPath);
            UnityEngine.Object.DestroyImmediate(prefab);
        }

        private static void CreateEffectPrefab()
        {
            var effect = new GameObject("Integration Effect");
            ParticleSystem particles = effect.AddComponent<ParticleSystem>();
            ParticleSystem.MainModule main = particles.main;
            main.startColor = new ParticleSystem.MinMaxGradient(
                new UnityEngine.Color(0.1f, 0.75f, 1),
                new UnityEngine.Color(0.6f, 0.2f, 1)
            );
            main.startLifetime = 1.5f;
            PrefabUtility.SaveAsPrefabAsset(effect, EffectPath);
            UnityEngine.Object.DestroyImmediate(effect);
        }

        private static void CreateContentScene()
        {
            Scene scene = EditorSceneManager.NewScene(
                NewSceneSetup.EmptyScene,
                NewSceneMode.Single
            );
            var stage = new GameObject("Game-Owned Integration Stage");
            var platform = GameObject.CreatePrimitive(PrimitiveType.Cube);
            platform.name = "Game-Owned Stage — Not Masonry Owned";
            platform.transform.SetParent(stage.transform, false);
            platform.transform.localPosition = new UnityEngine.Vector3(0, -2.25f, 0.75f);
            platform.transform.localScale = new UnityEngine.Vector3(8, 0.25f, 4);
            platform.GetComponent<Renderer>().sharedMaterial =
                AssetDatabase.LoadAssetAtPath<Material>(MaterialPath);
            EditorSceneManager.SaveScene(scene, ContentScenePath);
        }

        private static void CreateBootstrapScene()
        {
            Scene scene = EditorSceneManager.NewScene(
                NewSceneSetup.EmptyScene,
                NewSceneMode.Single
            );
            var bootstrap = new GameObject("Game-Owned Integration Bootstrap — Not Masonry Owned");
            MasonryRunner runner = bootstrap.AddComponent<MasonryRunner>();
            MasonryIntegrationFixture fixture = bootstrap.AddComponent<MasonryIntegrationFixture>();
            var serializedFixture = new SerializedObject(fixture);
            serializedFixture.FindProperty("runner").objectReferenceValue = runner;
            serializedFixture.ApplyModifiedPropertiesWithoutUndo();

            var capture = new GameObject("Integration Capture Driver — Not Masonry Owned");
            capture.AddComponent<MasonryIntegrationCaptureScenario>();
            EditorSceneManager.SaveScene(scene, BootstrapScenePath);
        }

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
            foreach ((string address, (string path, Type _)) in Entries)
            {
                string guid = AssetDatabase.AssetPathToGUID(path);
                AddressableAssetEntry entry = settings.CreateOrMoveEntry(guid, group);
                entry.address = address;
            }
            SelectPackedBuilder(settings);
            SelectFastPlayMode(settings);
            settings.SetDirty(AddressableAssetSettings.ModificationEvent.EntryMoved, null, true);
        }

        private static void SelectPackedBuilder(AddressableAssetSettings settings)
        {
            int index = settings.DataBuilders.FindIndex(builder =>
                builder is BuildScriptPackedMode
            );
            if (index < 0)
            {
                throw new InvalidOperationException(
                    "Addressables has no packed-mode builder for the integration catalog."
                );
            }
            settings.ActivePlayerDataBuilderIndex = index;
        }

        private static void SelectFastPlayMode(AddressableAssetSettings settings)
        {
            int index = settings.DataBuilders.FindIndex(builder => builder is BuildScriptFastMode);
            if (index < 0)
            {
                throw new InvalidOperationException(
                    "Addressables has no fast-mode builder for Editor integration tests."
                );
            }
            settings.ActivePlayModeDataBuilderIndex = index;
        }

        private static void ValidatePrefabRoots()
        {
            GameObject prefab = RequiredAsset<GameObject>(PrefabPath);
            RequireRootCount<Renderer>(prefab, PrefabPath, 1);
            RequireRootCount<Collider>(prefab, PrefabPath, 1);
            RequireRootCount<Animator>(prefab, PrefabPath, 1);
            RequireRootCount<ParticleSystem>(RequiredAsset<GameObject>(EffectPath), EffectPath, 1);
        }

        private static void ValidateBootstrap()
        {
            Scene scene = EditorSceneManager.OpenScene(BootstrapScenePath, OpenSceneMode.Single);
            MasonryIntegrationFixture[] fixtures = scene
                .GetRootGameObjects()
                .SelectMany(root => root.GetComponentsInChildren<MasonryIntegrationFixture>(true))
                .ToArray();
            if (fixtures.Length != 1)
            {
                throw new InvalidOperationException(
                    $"'{BootstrapScenePath}' must contain one MasonryIntegrationFixture; "
                        + $"found {fixtures.Length}."
                );
            }
            if (
                !MasonryIntegrationFixture.CustomCommandType.StartsWith(
                    "fixture.",
                    StringComparison.Ordinal
                )
            )
            {
                throw new InvalidOperationException(
                    "The integration custom handler type is invalid."
                );
            }
        }

        private static void ValidateProtocolFixture()
        {
            const string fixturePath =
                "Packages/com.masonry.client/Tests/Fixtures/rust-response.msgpack";
            byte[] bytes = File.ReadAllBytes(fixturePath);
            try
            {
                MasonryMessagePack.DeserializeResponse(bytes);
            }
            catch (Exception exception)
            {
                throw new InvalidOperationException(
                    $"Protocol fixture '{fixturePath}' is incompatible: {exception.Message}",
                    exception
                );
            }
        }

        private static void RequireRootCount<T>(GameObject asset, string path, int expected)
            where T : Component
        {
            int count = asset.GetComponents<T>().Length;
            if (count != expected)
            {
                throw new InvalidOperationException(
                    $"Addressable '{path}' requires {expected} root {typeof(T).Name}; "
                        + $"found {count}."
                );
            }
        }

        private static T RequiredAsset<T>(string path)
            where T : UnityEngine.Object
        {
            T asset = AssetDatabase.LoadAssetAtPath<T>(path);
            if (!asset)
            {
                throw new InvalidOperationException(
                    $"Required integration asset is missing: {path}"
                );
            }
            return asset;
        }

        private static void EnsureFolder(string path)
        {
            if (AssetDatabase.IsValidFolder(path))
            {
                return;
            }

            string parent =
                Path.GetDirectoryName(path)?.Replace('\\', '/')
                ?? throw new InvalidOperationException($"Asset folder has no parent: {path}");
            EnsureFolder(parent);
            AssetDatabase.CreateFolder(parent, Path.GetFileName(path));
        }
    }
}
