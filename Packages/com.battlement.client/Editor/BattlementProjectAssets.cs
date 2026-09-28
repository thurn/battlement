#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using Newtonsoft.Json.Linq;
using UnityEditor;
using UnityEditor.AddressableAssets.Settings.GroupSchemas;
using UnityEditor.SceneManagement;
using UnityEngine;

namespace Battlement.Editor
{
    /// <summary>Prepares imported assets and a native shell from project-assets.json.</summary>
    public static class BattlementProjectAssets
    {
        public static void Generate()
        {
            JObject manifest = JObject.Parse(File.ReadAllText("project-assets.json"));
            string output =
                Environment.GetEnvironmentVariable("BATTLEMENT_PROJECT_ASSETS_OUTPUT")
                ?? throw new InvalidOperationException("Project asset output is required.");
            var retained = new HashSet<string>(StringComparer.Ordinal);
            foreach (JToken item in manifest["materials"]!)
            {
                string path = AssetPath(item, "path");
                Directory.CreateDirectory(Path.GetDirectoryName(path)!);
                string shaderName = item["shader"]?.Value<string>() ?? "Standard";
                Shader shader = Shader.Find(shaderName);
                if (!shader)
                {
                    throw new InvalidOperationException($"Missing shader {shaderName} for {path}");
                }
                Material material = AssetDatabase.LoadAssetAtPath<Material>(path);
                if (!material)
                {
                    material = new Material(shader);
                    AssetDatabase.CreateAsset(material, path);
                }
                material.shader = shader;
                Texture2D? texture = null;
                if (item["texture"] != null)
                {
                    texture = AssetDatabase.LoadAssetAtPath<Texture2D>(AssetPath(item, "texture"));
                    if (!texture)
                        throw new InvalidOperationException($"Missing texture for {path}");
                }
                material.mainTexture = texture;
                string color = item["color"]?.Value<string>() ?? "#FFFFFF";
                if (!ColorUtility.TryParseHtmlString(color, out UnityEngine.Color tint))
                    throw new InvalidOperationException(
                        $"Invalid material color for {path}: {color}"
                    );
                material.color = tint;
                if (material.HasProperty("_Metallic"))
                    material.SetFloat("_Metallic", 0);
                if (material.HasProperty("_Glossiness"))
                    material.SetFloat("_Glossiness", 0);
                EditorUtility.SetDirty(material);
                retained.Add(path);
            }
            AssetDatabase.SaveAssets();
            foreach (JToken item in manifest["particles"] ?? new JArray())
                CreateParticles(item, retained);
            foreach (JToken item in manifest["models"]!)
            {
                string path = AssetPath(item, "path");
                var importer = AssetImporter.GetAtPath(path) as ModelImporter;
                if (!importer)
                    throw new InvalidOperationException($"Missing model {path}");
                foreach (JProperty mapping in ((JObject)item["materials"]!).Properties())
                {
                    string materialPath = ValidAssetPath(mapping.Value.Value<string>()!);
                    Material material = AssetDatabase.LoadAssetAtPath<Material>(materialPath);
                    if (!material)
                        throw new InvalidOperationException($"Missing material {materialPath}");
                    importer.AddRemap(
                        new AssetImporter.SourceAssetIdentifier(typeof(Material), mapping.Name),
                        material
                    );
                }
                importer.importAnimation = false;
                importer.SaveAndReimport();
                retained.Add(path + ".meta");
                foreach (
                    Renderer renderer in AssetDatabase
                        .LoadAssetAtPath<GameObject>(path)
                        .GetComponentsInChildren<Renderer>()
                )
                    if (
                        renderer.sharedMaterials.Any(material => !material || !material.mainTexture)
                    )
                        throw new InvalidOperationException($"Unmapped model material in {path}");
            }

            string bootstrap = AssetPath(manifest, "bootstrap");
            string content = AssetPath(manifest, "content");
            CreateScene(bootstrap, true);
            CreateScene(content, false);
            retained.Add(bootstrap);
            retained.Add(content);
            var settings = BattlementSampleBuild.AddressableSettings();
            string groupName = manifest["name"]!.Value<string>()!;
            var group = settings.FindGroup(groupName);
            if (!group)
                group = settings.CreateGroup(
                    groupName,
                    false,
                    false,
                    true,
                    null,
                    typeof(BundledAssetGroupSchema),
                    typeof(ContentUpdateGroupSchema)
                );
            var declared = new HashSet<string>(StringComparer.Ordinal);
            foreach (JToken item in manifest["addresses"]!)
            {
                string address = item["address"]!.Value<string>()!;
                if (!declared.Add(address))
                    throw new InvalidOperationException($"Duplicate address {address}");
                string path = AssetPath(item, "path");
                string guid = AssetDatabase.AssetPathToGUID(path);
                if (string.IsNullOrEmpty(guid))
                    throw new InvalidOperationException($"Missing address asset {path}");
                settings.CreateOrMoveEntry(guid, group).address = address;
                retained.Add(path + ".meta");
            }
            foreach (var entry in group.entries.ToArray())
                if (!declared.Contains(entry.address))
                    settings.RemoveAssetEntry(entry.guid);
            PlayerSettings.productName = groupName;
            PlayerSettings.companyName = "Battlement";
            PlayerSettings.runInBackground = true;
            EditorBuildSettings.scenes = new[] { new EditorBuildSettingsScene(bootstrap, true) };
            AssetDatabase.SaveAssets();
            AssetDatabase.Refresh(ImportAssetOptions.ForceSynchronousImport);
            foreach (
                string path in Directory.GetFiles(
                    "Assets/AddressableAssetsData",
                    "*",
                    SearchOption.AllDirectories
                )
            )
                retained.Add(path.Replace('\\', '/'));
            retained.Add("Assets/AddressableAssetsData.meta");
            retained.Add("ProjectSettings/ProjectSettings.asset");
            retained.Add("ProjectSettings/EditorBuildSettings.asset");
            foreach (string path in retained.ToArray())
            {
                if (File.Exists(path + ".meta"))
                    retained.Add(path + ".meta");
                for (
                    string? parent = Path.GetDirectoryName(path);
                    parent?.StartsWith("Assets/") == true;
                    parent = Path.GetDirectoryName(parent)
                )
                    if (File.Exists(parent + ".meta"))
                        retained.Add(parent + ".meta");
            }
            foreach (string path in retained)
            {
                string destination = Path.Combine(output, path);
                Directory.CreateDirectory(Path.GetDirectoryName(destination)!);
                File.Copy(path, destination, true);
            }
            File.WriteAllText(
                Path.Combine(output, "inventory.json"),
                new JObject(
                    new JProperty("addresses", declared.OrderBy(value => value)),
                    new JProperty("files", retained.OrderBy(value => value))
                ).ToString()
            );
            Debug.Log($"BATTLEMENT_PROJECT_ASSETS_OK:{declared.Count}");
        }

        private static void CreateParticles(JToken item, HashSet<string> retained)
        {
            string path = AssetPath(item, "path");
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            var root = new GameObject(Path.GetFileNameWithoutExtension(path));
            try
            {
                ParticleSystem particles = root.AddComponent<ParticleSystem>();
                particles.Stop(false, ParticleSystemStopBehavior.StopEmittingAndClear);
                ParticleSystem.MainModule main = particles.main;
                main.playOnAwake = false;
                main.loop = item["loop"]!.Value<bool>();
                main.prewarm = main.loop;
                main.duration = item["lifetime"]!.Value<float>();
                main.startLifetime = main.duration;
                main.startSize = item["size"]!.Value<float>();
                main.startSpeed = item["speed"]!.Value<float>();
                main.maxParticles = 48;
                main.simulationSpace = ParticleSystemSimulationSpace.Local;
                particles.useAutoRandomSeed = false;
                particles.randomSeed = 1;
                ParticleSystem.EmissionModule emission = particles.emission;
                emission.rateOverTime = main.loop ? item["rate"]!.Value<float>() : 0;
                emission.SetBursts(
                    main.loop
                        ? Array.Empty<ParticleSystem.Burst>()
                        : new[] { new ParticleSystem.Burst(0, item["count"]!.Value<short>()) }
                );
                ParticleSystem.ShapeModule shape = particles.shape;
                shape.shapeType = ParticleSystemShapeType.Box;
                JArray box = (JArray)item["box"]!;
                shape.scale = new UnityEngine.Vector3(
                    box[0].Value<float>(),
                    box[1].Value<float>(),
                    box[2].Value<float>()
                );
                ParticleSystemRenderer renderer = root.GetComponent<ParticleSystemRenderer>();
                Material material = AssetDatabase.LoadAssetAtPath<Material>(
                    AssetPath(item, "material")
                );
                if (!material)
                    throw new InvalidOperationException($"Missing particle material for {path}");
                renderer.sharedMaterial = material;
                renderer.renderMode = ParticleSystemRenderMode.Billboard;
                renderer.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
                renderer.receiveShadows = false;
                PrefabUtility.SaveAsPrefabAsset(root, path);
                retained.Add(path);
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(root);
            }
        }

        private static void CreateScene(string path, bool bootstrap)
        {
            if (File.Exists(path))
                return;
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
            if (bootstrap)
                new GameObject("Battlement", typeof(BattlementBootstrap));
            EditorSceneManager.SaveScene(scene, path);
        }

        private static string AssetPath(JToken item, string key) =>
            ValidAssetPath(item[key]!.Value<string>()!);

        private static string ValidAssetPath(string path)
        {
            if (
                !path.StartsWith("Assets/", StringComparison.Ordinal)
                || path.Split('/').Contains("..")
            )
                throw new InvalidOperationException($"Expected a project asset path: {path}");
            return path;
        }
    }
}
