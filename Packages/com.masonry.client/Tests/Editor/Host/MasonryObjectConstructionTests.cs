#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.SceneManagement;
using Object = UnityEngine.Object;

namespace Masonry.Tests
{
    public sealed class MasonryObjectConstructionTests
    {
        [Test]
        public void SnapshotConstructsBaseObjectsAcrossPlacementsAndHierarchy()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            MasonryScene scene = ContentScene("game/objects");
            var prefabAddress = new PrefabAddress("game/prefab");
            GameObject prefab = PrefabTemplate();
            harness.AssetStorage.EnqueueValue(prefab);
            var parentId = NewObjectId();
            var cubeId = NewObjectId();
            var sphereId = NewObjectId();
            var capsuleId = NewObjectId();
            var cylinderId = NewObjectId();
            var planeId = NewObjectId();
            var quadId = NewObjectId();
            var prefabId = NewObjectId();
            MasonryGameObject[] objects =
            {
                Describe(parentId, new GameObjectKind.Empty(), new ParentScene.Persistent(), false),
                Describe(
                    cubeId,
                    new GameObjectKind.Cube(),
                    new ParentScene.Persistent(),
                    true,
                    parentId,
                    new LocalTransform(
                        new Masonry.Vector3(1, 2, 3),
                        new Masonry.Quaternion(0, 0, 0, 2),
                        new Masonry.Vector3(2, 3, 4)
                    ),
                    new[] { PointerEvent.Click }
                ),
                Describe(sphereId, new GameObjectKind.Sphere(), new ParentScene.Primary()),
                Describe(
                    capsuleId,
                    new GameObjectKind.Capsule(),
                    new ParentScene.Specific(scene.Id)
                ),
                Describe(cylinderId, new GameObjectKind.Cylinder(), new ParentScene.Primary()),
                Describe(planeId, new GameObjectKind.Plane(), new ParentScene.Primary()),
                Describe(quadId, new GameObjectKind.Quad(), new ParentScene.Primary()),
                Describe(
                    prefabId,
                    new GameObjectKind.Prefab(prefabAddress),
                    new ParentScene.Primary()
                ),
            };
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: new PreparedAsset[]
                    {
                        new PreparedAsset.Prefab(prefabAddress),
                        new PreparedAsset.Scene(scene.Address),
                    },
                    scenes: new[] { scene },
                    objects: objects
                )
            );

            harness.Runner.Connect();

            Assert.That(Identities(), Has.Length.EqualTo(objects.Length));
            MasonryIdentity parent = Identity(parentId);
            MasonryIdentity cube = Identity(cubeId);
            Assert.That(parent.gameObject.activeSelf, Is.False);
            Assert.That(cube.gameObject.activeSelf, Is.True);
            Assert.That(cube.gameObject.activeInHierarchy, Is.False);
            Assert.That(cube.transform.parent, Is.EqualTo(parent.transform));
            Assert.That(cube.transform.localPosition, Is.EqualTo(new UnityEngine.Vector3(1, 2, 3)));
            Assert.That(cube.transform.localRotation, Is.EqualTo(UnityEngine.Quaternion.identity));
            Assert.That(cube.transform.localScale, Is.EqualTo(new UnityEngine.Vector3(2, 3, 4)));
            Assert.That(cube.GetComponent<BoxCollider>(), Is.Not.Null);

            foreach (ObjectId id in new[] { sphereId, capsuleId, cylinderId, planeId, quadId })
            {
                MasonryIdentity primitive = Identity(id);
                Assert.That(primitive.GetComponent<Renderer>(), Is.Not.Null);
                Assert.That(primitive.GetComponent<Collider>(), Is.Null);
                Assert.That(
                    primitive.transform.localPosition,
                    Is.EqualTo(UnityEngine.Vector3.zero)
                );
                Assert.That(primitive.transform.localScale, Is.EqualTo(UnityEngine.Vector3.one));
            }

            Assert.That(
                Identity(sphereId).gameObject.scene,
                Is.EqualTo(SceneManager.GetActiveScene())
            );
            Assert.That(
                Identity(capsuleId).gameObject.scene,
                Is.EqualTo(SceneManager.GetActiveScene())
            );
            Assert.That(parent.gameObject.scene, Is.EqualTo(harness.Scene));

            MasonryIdentity instance = Identity(prefabId);
            Assert.That(instance.gameObject.activeSelf, Is.True);
            Assert.That(instance.GetComponents<MasonryIdentity>(), Has.Length.EqualTo(1));
            Assert.That(instance.GetComponentsInChildren<Renderer>(), Has.Length.EqualTo(2));
            Assert.That(instance.transform.GetChild(0).GetComponent<MasonryIdentity>(), Is.Null);
            Assert.That(instance.transform.GetChild(0).GetComponent<BoxCollider>(), Is.Not.Null);
        }

        [Test]
        public void PrefabInstanceRetainsItsPreparedLeaseUntilDestruction()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            var address = new PrefabAddress("game/leased-prefab");
            var asset = new PreparedAsset.Prefab(address);
            var objectId = NewObjectId();
            harness.AssetStorage.EnqueueValue(PrefabTemplate());
            harness.Transport.EnqueueConnect(
                Response(session, new[] { asset }, new[] { PersistentPrefab(objectId, address) })
            );
            harness.Runner.Connect();
            FakeAssetHandle handle = harness.AssetStorage.Handles.Single(value =>
                value.Asset == asset
            );
            harness.Transport.EnqueueSubmit(
                Response(session, Array.Empty<PreparedAsset>(), Array.Empty<MasonryGameObject>())
            );

            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(handle.IsDisposed, Is.False);
            Assert.That(harness.Runner.TryGetPreparedAsset(asset, out _), Is.False);
            Assert.That(Identity(objectId), Is.Not.Null);
            harness.Runner.Stop();
            Assert.That(handle.IsDisposed, Is.True);
        }

        [TestCase(true)]
        [TestCase(false)]
        public void MissingOrWrongKindPrefabFailsWithoutCreatingObjects(bool wrongKind)
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            const string address = "game/missing-prefab";
            PreparedAsset[] assets = wrongKind
                ? new PreparedAsset[] { new PreparedAsset.Texture(new TextureAddress(address)) }
                : Array.Empty<PreparedAsset>();
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: assets,
                    objects: new[] { PersistentPrefab(NewObjectId(), new PrefabAddress(address)) }
                )
            );

            harness.Runner.Connect();

            Assert.That(Identities(), Is.Empty);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("prepared set"));
        }

        [Test]
        public void PrefabStateRejectsAnUnsupportedRootComponentCount()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var address = new PrefabAddress("game/invalid-prefab");
            harness.AssetStorage.EnqueueValue(PrefabTemplate());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: new PreparedAsset[] { new PreparedAsset.Prefab(address) },
                    objects: new[]
                    {
                        Describe(
                            NewObjectId(),
                            new GameObjectKind.Prefab(
                                address,
                                Array.Empty<MaterialAssignment>(),
                                new AnimatorState("Idle")
                            ),
                            new ParentScene.Persistent()
                        ),
                    }
                )
            );

            harness.Runner.Connect();

            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("no root Animator"));
        }

        private static MasonryGameObject Describe(
            ObjectId id,
            GameObjectKind kind,
            ParentScene parentScene,
            bool active = true,
            ObjectId? parentId = null,
            LocalTransform? transform = null,
            PointerEvent[]? pointerEvents = null
        ) =>
            new(
                id,
                kind,
                parentScene,
                parentId,
                active,
                transform ?? LocalTransform.Identity,
                pointerEvents ?? Array.Empty<PointerEvent>()
            );

        private static MasonryGameObject PersistentPrefab(ObjectId id, PrefabAddress address) =>
            Describe(id, new GameObjectKind.Prefab(address), new ParentScene.Persistent());

        private static MasonryScene ContentScene(string address) =>
            new(new SceneId(Guid.NewGuid()), new SceneAddress(address));

        private static ObjectId NewObjectId() => new(Guid.NewGuid());

        private static MasonryIdentity Identity(ObjectId id) =>
            Identities().Single(identity => identity.Id == id.Value);

        private static MasonryIdentity[] Identities() =>
            Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Include)
                .Where(identity => !FakeMasonryTransport.IsFixtureIdentity(identity))
                .ToArray();

        private static GameObject PrefabTemplate()
        {
            var prefab = new GameObject("Prepared prefab");
            prefab.SetActive(false);
            prefab.AddComponent<MeshFilter>();
            prefab.AddComponent<MeshRenderer>();
            var child = new GameObject("Authored child");
            child.transform.SetParent(prefab.transform, false);
            child.AddComponent<MeshFilter>();
            child.AddComponent<MeshRenderer>();
            child.AddComponent<BoxCollider>();
            return prefab;
        }

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
    }
}
