#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEditor;
using UnityEditor.Animations;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Masonry.Tests
{
    public sealed class MasonryMaterialAnimatorTests
    {
        [Test]
        public void SnapshotAppliesRootMaterialsAndStableAnimatorState()
        {
            using ControllerFixture controller = Controller();
            Material red = null!;
            Material blue = null!;
            Material childMaterial = null!;
            var prefabAddress = new PrefabAddress("game/animated-prefab");
            var redAddress = new MaterialAddress("game/red");
            var blueAddress = new MaterialAddress("game/blue");
            var primitiveId = NewObjectId();
            var prefabId = NewObjectId();
            try
            {
                using MasonryTestHarness harness = MasonryTestHarness.Create();
                red = Material(UnityEngine.Color.red);
                blue = Material(UnityEngine.Color.blue);
                childMaterial = Material(UnityEngine.Color.green);
                GameObject prefab = AnimatedPrefab(controller.Value, childMaterial);
                harness.AssetStorage.EnqueueValue(prefab);
                harness.AssetStorage.EnqueueValue(red);
                harness.AssetStorage.EnqueueValue(blue);
                harness.Transport.EnqueueConnect(
                    FakeMasonryTransport.SnapshotResponse(
                        preparedAssets: new PreparedAsset[]
                        {
                            new PreparedAsset.Prefab(prefabAddress),
                            new PreparedAsset.Material(redAddress),
                            new PreparedAsset.Material(blueAddress),
                        },
                        objects: new[]
                        {
                            Persistent(
                                primitiveId,
                                new GameObjectKind.Cube(
                                    new[] { new MaterialAssignment(0, redAddress) }
                                )
                            ),
                            Persistent(
                                prefabId,
                                new GameObjectKind.Prefab(
                                    prefabAddress,
                                    new[]
                                    {
                                        new MaterialAssignment(0, redAddress),
                                        new MaterialAssignment(1, blueAddress),
                                    },
                                    new AnimatorState(
                                        "Stable",
                                        0,
                                        0.25,
                                        new Dictionary<string, bool> { ["Visible"] = true },
                                        new Dictionary<string, int> { ["Index"] = 4 },
                                        new Dictionary<string, double> { ["Blend"] = 0.75 },
                                        0.5
                                    )
                                )
                            ),
                        }
                    )
                );

                harness.Runner.Connect();

                Assert.That(
                    Identity(primitiveId).GetComponent<Renderer>().sharedMaterial,
                    Is.SameAs(red)
                );
                GameObject instance = Identity(prefabId).gameObject;
                Assert.That(
                    instance.GetComponent<Renderer>().sharedMaterials,
                    Is.EqualTo(new[] { red, blue })
                );
                Assert.That(
                    instance.transform.GetChild(0).GetComponent<Renderer>().sharedMaterial,
                    Is.SameAs(childMaterial)
                );
                Animator animator = instance.GetComponent<Animator>();
                int visibleParameter = Animator.StringToHash("Visible");
                int indexParameter = Animator.StringToHash("Index");
                int blendParameter = Animator.StringToHash("Blend");
                Assert.That(animator.GetCurrentAnimatorStateInfo(0).IsName("Stable"), Is.True);
                Assert.That(
                    animator.GetCurrentAnimatorStateInfo(0).normalizedTime,
                    Is.EqualTo(0.25f).Within(0.001f)
                );
                Assert.That(animator.GetBool(visibleParameter), Is.True);
                Assert.That(animator.GetInteger(indexParameter), Is.EqualTo(4));
                Assert.That(animator.GetFloat(blendParameter), Is.EqualTo(0.75f).Within(0.001f));
                Assert.That(animator.speed, Is.EqualTo(0.5f));
            }
            finally
            {
                Destroy(red, blue, childMaterial);
            }
        }

        [Test]
        public void DistinctAssignedMaterialLeaseSurvivesSetReplacementUntilObjectDestruction()
        {
            Material material = Material(UnityEngine.Color.magenta);
            var address = new MaterialAddress("game/shared-material");
            var asset = new PreparedAsset.Material(address);
            SessionId session = new(Guid.NewGuid());
            try
            {
                using MasonryTestHarness harness = MasonryTestHarness.Create();
                harness.AssetStorage.EnqueueValue(material);
                harness.Transport.EnqueueConnect(
                    Response(
                        session,
                        new[] { asset },
                        new[]
                        {
                            Persistent(
                                NewObjectId(),
                                new GameObjectKind.Cube(
                                    new[] { new MaterialAssignment(0, address) }
                                )
                            ),
                        }
                    )
                );
                harness.Runner.Connect();
                FakeAssetHandle handle = harness.AssetStorage.Handles.Single(value =>
                    value.Asset == asset
                );
                harness.Transport.EnqueueSubmit(
                    Response(
                        session,
                        Array.Empty<PreparedAsset>(),
                        Array.Empty<MasonryGameObject>()
                    )
                );

                harness.Runner.Submit(new byte[] { 1 });

                Assert.That(handle.IsDisposed, Is.False);
                harness.Runner.Stop();
                Assert.That(handle.IsDisposed, Is.True);
            }
            finally
            {
                Destroy(material);
            }
        }

        [TestCase("duplicate-slot")]
        [TestCase("slot-out-of-range")]
        [TestCase("missing-renderer")]
        [TestCase("wrong-material-kind")]
        [TestCase("invalid-animator-layer")]
        [TestCase("invalid-animator-parameter")]
        public void InvalidMaterialOrAnimatorSnapshotStateStopsTheSession(string invalidCase)
        {
            using ControllerFixture controller = Controller();
            Material material = null!;
            var prefabAddress = new PrefabAddress("game/invalid-prefab");
            var materialAddress = new MaterialAddress("game/invalid-material");
            IReadOnlyList<MaterialAssignment> assignments = invalidCase switch
            {
                "duplicate-slot" => new[]
                {
                    new MaterialAssignment(0, materialAddress),
                    new MaterialAssignment(0, materialAddress),
                },
                "slot-out-of-range" => new[] { new MaterialAssignment(2, materialAddress) },
                "missing-renderer" => new[] { new MaterialAssignment(0, materialAddress) },
                "wrong-material-kind" => new[] { new MaterialAssignment(0, materialAddress) },
                _ => Array.Empty<MaterialAssignment>(),
            };
            AnimatorState? animator = invalidCase switch
            {
                "invalid-animator-layer" => new AnimatorState(
                    "Stable",
                    4,
                    0,
                    new Dictionary<string, bool>(),
                    new Dictionary<string, int>(),
                    new Dictionary<string, double>(),
                    1
                ),
                "invalid-animator-parameter" => new AnimatorState(
                    "Stable",
                    0,
                    0,
                    new Dictionary<string, bool> { ["Index"] = true },
                    new Dictionary<string, int>(),
                    new Dictionary<string, double>(),
                    1
                ),
                _ => null,
            };
            try
            {
                using MasonryTestHarness harness = MasonryTestHarness.Create();
                material = Material(UnityEngine.Color.white);
                GameObject prefab =
                    invalidCase == "missing-renderer"
                        ? new GameObject("Renderer-free prefab")
                        : AnimatedPrefab(controller.Value, material);
                harness.AssetStorage.EnqueueValue(prefab);
                PreparedAsset materialAsset =
                    invalidCase == "wrong-material-kind"
                        ? new PreparedAsset.Texture(new TextureAddress(materialAddress.Value))
                        : new PreparedAsset.Material(materialAddress);
                if (assignments.Count > 0)
                {
                    harness.AssetStorage.EnqueueValue(material);
                }

                harness.Transport.EnqueueConnect(
                    FakeMasonryTransport.SnapshotResponse(
                        preparedAssets: new PreparedAsset[]
                        {
                            new PreparedAsset.Prefab(prefabAddress),
                            materialAsset,
                        },
                        objects: new[]
                        {
                            Persistent(
                                NewObjectId(),
                                new GameObjectKind.Prefab(prefabAddress, assignments, animator)
                            ),
                        }
                    )
                );

                harness.Runner.Connect();

                Assert.That(Identities(), Is.Empty);
                Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
                Assert.That(harness.Logger.Records.Last().Message, Does.Contain("Snapshot"));
            }
            finally
            {
                Destroy(material);
            }
        }

        private static ControllerFixture Controller()
        {
            string path = $"Assets/MasonryTestAnimator-{Guid.NewGuid():N}.controller";
            AnimatorController controller = AnimatorController.CreateAnimatorControllerAtPath(path);
            UnityEditor.Animations.AnimatorState stable = controller
                .layers[0]
                .stateMachine.AddState("Stable");
            var clip = new AnimationClip { name = "Stable pose" };
            clip.SetCurve(
                string.Empty,
                typeof(Transform),
                "localPosition.x",
                AnimationCurve.Constant(0, 1, 1.5f)
            );
            stable.motion = clip;
            controller.AddParameter("Visible", AnimatorControllerParameterType.Bool);
            controller.AddParameter("Index", AnimatorControllerParameterType.Int);
            controller.AddParameter("Blend", AnimatorControllerParameterType.Float);
            AssetDatabase.AddObjectToAsset(clip, controller);
            AssetDatabase.SaveAssets();
            AssetDatabase.ImportAsset(path, ImportAssetOptions.ForceSynchronousImport);
            return new ControllerFixture(
                path,
                AssetDatabase.LoadAssetAtPath<AnimatorController>(path)
            );
        }

        private static GameObject AnimatedPrefab(
            RuntimeAnimatorController controller,
            Material childMaterial
        )
        {
            var prefab = new GameObject("Animated prefab");
            prefab.AddComponent<MeshFilter>();
            MeshRenderer renderer = prefab.AddComponent<MeshRenderer>();
            renderer.sharedMaterials = new[] { childMaterial, childMaterial };
            prefab.AddComponent<Animator>().runtimeAnimatorController = controller;
            var child = GameObject.CreatePrimitive(PrimitiveType.Cube);
            child.name = "Authored child";
            child.transform.SetParent(prefab.transform, false);
            child.GetComponent<Renderer>().sharedMaterial = childMaterial;
            return prefab;
        }

        private static Material Material(UnityEngine.Color color)
        {
            Shader shader = Shader.Find("Universal Render Pipeline/Lit");
            var material = new Material(shader) { color = color };
            return material;
        }

        private static MasonryGameObject Persistent(ObjectId id, GameObjectKind kind) =>
            new(
                id,
                kind,
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static ObjectId NewObjectId() => new(Guid.NewGuid());

        private static MasonryIdentity Identity(ObjectId id) =>
            Identities().Single(identity => identity.Id == id.Value);

        private static MasonryIdentity[] Identities() =>
            Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Include)
                .Where(identity => !FakeMasonryTransport.IsFixtureIdentity(identity))
                .ToArray();

        private static MasonryTransportResult Response(
            SessionId session,
            IReadOnlyList<PreparedAsset> assets,
            IReadOnlyList<MasonryGameObject> objects
        ) =>
            FakeMasonryTransport.ResponseResult(
                new Response(
                    session,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.SnapshotMessage(
                            FakeMasonryTransport.CompleteSnapshot(
                                session,
                                preparedAssets: assets,
                                objects: objects
                            )
                        ),
                    }
                )
            );

        private static void Destroy(params Object[] values)
        {
            foreach (Object value in values)
            {
                if (value != null)
                {
                    Object.DestroyImmediate(value);
                }
            }
        }

        private sealed class ControllerFixture : IDisposable
        {
            private readonly string path;

            public ControllerFixture(string path, AnimatorController value)
            {
                this.path = path;
                Value = value;
            }

            public AnimatorController Value { get; }

            public void Dispose() => AssetDatabase.DeleteAsset(path);
        }
    }
}
