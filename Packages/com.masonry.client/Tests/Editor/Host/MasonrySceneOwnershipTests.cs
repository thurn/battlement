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
    public sealed class MasonrySceneOwnershipTests
    {
        [Test]
        public void SnapshotLoadsAdditiveScenesAndSelectsTheDeclaredPrimary()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            MasonryScene first = ContentScene("game/forest");
            MasonryScene second = ContentScene("game/castle");
            var sceneObjectId = new ObjectId(Guid.NewGuid());
            var persistentObjectId = new ObjectId(Guid.NewGuid());
            MasonryGameObject[] objects =
            {
                EmptyObject(sceneObjectId, new ParentScene.Specific(second.Id)),
                EmptyObject(persistentObjectId, new ParentScene.Persistent()),
            };
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: SceneAssets(first, second),
                    scenes: new[] { first, second },
                    primarySceneId: second.Id,
                    objects: objects
                )
            );

            harness.Runner.Connect();

            Assert.That(
                harness.Logger.Records.Where(record => record.Severity == MasonryLogSeverity.Error),
                Is.Empty,
                string.Join(" | ", harness.Logger.Records.Select(record => record.Message))
            );
            Assert.That(harness.AssetStorage.SceneLoadCalls, Has.Count.EqualTo(2));
            Assert.That(
                harness.AssetStorage.SceneLoadCalls,
                Is.EqualTo(SceneAssets(first, second)),
                "Scenes must begin loading in snapshot order."
            );
            Assert.That(
                SceneManager.GetActiveScene(),
                Is.EqualTo(HandleFor(harness, second).Scene)
            );
            MasonryIdentity sceneObject = Identity(sceneObjectId);
            MasonryIdentity persistentObject = Identity(persistentObjectId);
            Assert.That(sceneObject.gameObject.scene, Is.EqualTo(SceneManager.GetActiveScene()));
            Assert.That(sceneObject.transform.parent.name, Is.EqualTo("Masonry Scene"));
            Assert.That(persistentObject.gameObject.scene, Is.EqualTo(harness.Scene));
            Assert.That(persistentObject.transform.parent.name, Is.EqualTo("Masonry Persistent"));
            Assert.That(
                harness.Scene.GetRootGameObjects().Select(value => value.name),
                Does.Contain("Masonry host")
            );
        }

        [Test]
        public void ReplacementReusesExactScenesAndUnloadsOnlyRemovedContent()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            MasonryScene retained = ContentScene("game/retained");
            MasonryScene removed = ContentScene("game/removed");
            var removedObjectId = new ObjectId(Guid.NewGuid());
            var persistentObjectId = new ObjectId(Guid.NewGuid());
            var bootstrapObject = new GameObject("Unrelated bootstrap content");
            SceneManager.MoveGameObjectToScene(bootstrapObject, harness.Scene);
            harness.Transport.EnqueueConnect(
                Response(
                    session,
                    SceneAssets(retained, removed),
                    new[] { retained, removed },
                    retained.Id,
                    new[]
                    {
                        EmptyObject(removedObjectId, new ParentScene.Specific(removed.Id)),
                        EmptyObject(persistentObjectId, new ParentScene.Persistent()),
                    }
                )
            );
            harness.Runner.Connect();
            FakeSceneHandle retainedHandle = HandleFor(harness, retained);
            FakeSceneHandle removedHandle = HandleFor(harness, removed);
            GameObject authoredRemoved = AuthoredObject(removedHandle);
            harness.Transport.EnqueueSubmit(
                Response(
                    session,
                    SceneAssets(retained),
                    new[] { retained },
                    null,
                    Array.Empty<MasonryGameObject>()
                )
            );

            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(
                harness.Logger.Records.Where(record => record.Severity == MasonryLogSeverity.Error),
                Is.Empty,
                string.Join(" | ", harness.Logger.Records.Select(record => record.Message))
            );
            Assert.That(harness.AssetStorage.SceneLoadCalls, Has.Count.EqualTo(2));
            Assert.That(HandleFor(harness, retained), Is.SameAs(retainedHandle));
            Assert.That(authoredRemoved == null, Is.True);
            Assert.That(
                Object
                    .FindObjectsByType<MasonryIdentity>()
                    .Any(identity => identity.Id == removedObjectId.Value),
                Is.False
            );
            Assert.That(Identity(persistentObjectId), Is.Not.Null);
            Assert.That(bootstrapObject != null, Is.True);
        }

        [Test]
        public void ChangedUuidAtTheSameAddressWaitsForUnloadBeforeReloading()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            MasonryScene original = ContentScene("game/shared");
            MasonryScene replacement = new(new SceneId(Guid.NewGuid()), original.Address);
            harness.AssetStorage.EnqueueSceneUnloadPending();
            harness.Transport.EnqueueConnect(
                Response(
                    session,
                    SceneAssets(original),
                    new[] { original },
                    null,
                    Array.Empty<MasonryGameObject>()
                )
            );
            harness.Runner.Connect();
            FakeSceneHandle originalHandle = HandleFor(harness, original);
            GameObject authored = AuthoredObject(originalHandle);
            harness.Transport.EnqueueSubmit(
                Response(
                    session,
                    SceneAssets(replacement),
                    new[] { replacement },
                    null,
                    Array.Empty<MasonryGameObject>()
                )
            );

            harness.Runner.Submit(new byte[] { 2 });

            Assert.That(originalHandle.UnloadCallCount, Is.EqualTo(1));
            Assert.That(harness.AssetStorage.SceneLoadCalls, Has.Count.EqualTo(1));
            Assert.That(authored != null, Is.True);

            originalHandle.CompleteUnload();
            harness.Runner.RunFrame();

            Assert.That(harness.AssetStorage.SceneLoadCalls, Has.Count.EqualTo(2));
            Assert.That(HandleFor(harness, replacement), Is.Not.SameAs(originalHandle));
            Assert.That(authored == null, Is.True);
        }

        [Test]
        public void PrimaryCutoverReusesScenesAndLeavesBootstrapContentAlone()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            MasonryScene first = ContentScene("game/day");
            MasonryScene second = ContentScene("game/night");
            var bootstrapObject = new GameObject("Bootstrap sky");
            SceneManager.MoveGameObjectToScene(bootstrapObject, harness.Scene);
            harness.Transport.EnqueueConnect(
                Response(
                    session,
                    SceneAssets(first, second),
                    new[] { first, second },
                    first.Id,
                    Array.Empty<MasonryGameObject>()
                )
            );
            harness.Runner.Connect();
            FakeSceneHandle firstHandle = HandleFor(harness, first);
            FakeSceneHandle secondHandle = HandleFor(harness, second);
            harness.Transport.EnqueueSubmit(
                Response(
                    session,
                    SceneAssets(first, second),
                    new[] { first, second },
                    second.Id,
                    Array.Empty<MasonryGameObject>()
                )
            );

            harness.Runner.Submit(new byte[] { 3 });

            Assert.That(SceneManager.GetActiveScene(), Is.EqualTo(secondHandle.Scene));
            Assert.That(HandleFor(harness, first), Is.SameAs(firstHandle));
            Assert.That(HandleFor(harness, second), Is.SameAs(secondHandle));
            Assert.That(harness.AssetStorage.SceneLoadCalls, Has.Count.EqualTo(2));
            Assert.That(bootstrapObject != null, Is.True);
            Assert.That(bootstrapObject!.scene, Is.EqualTo(harness.Scene));
        }

        [Test]
        public void RemovedPreparedSceneStaysLeasedUntilUnloadCompletes()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            MasonryScene scene = ContentScene("game/slow-unload");
            harness.AssetStorage.EnqueueSceneUnloadPending();
            harness.Transport.EnqueueConnect(
                Response(
                    session,
                    SceneAssets(scene),
                    new[] { scene },
                    null,
                    Array.Empty<MasonryGameObject>()
                )
            );
            harness.Runner.Connect();
            FakeAssetHandle assetHandle = harness.AssetStorage.Handles.Single();
            FakeSceneHandle sceneHandle = HandleFor(harness, scene);
            GameObject authored = AuthoredObject(sceneHandle);
            MasonryScene replacement = ContentScene("game/replacement-after-slow-unload");
            harness.Transport.EnqueueSubmit(
                Response(
                    session,
                    SceneAssets(replacement),
                    new[] { replacement },
                    null,
                    Array.Empty<MasonryGameObject>()
                )
            );

            harness.Runner.Submit(new byte[] { 4 });

            Assert.That(assetHandle.IsDisposed, Is.False);
            Assert.That(authored != null, Is.True);

            sceneHandle.CompleteUnload();
            harness.Runner.RunFrame();

            Assert.That(assetHandle.IsDisposed, Is.True);
            Assert.That(authored == null, Is.True);
        }

        [Test]
        public void ReplacementGatesInputUntilDelayedSceneLoadCompletes()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            MasonryScene retained = ContentScene("game/retained-during-load");
            MasonryScene added = ContentScene("game/delayed-load");
            harness.Transport.EnqueueConnect(
                Response(
                    session,
                    SceneAssets(retained),
                    new[] { retained },
                    null,
                    Array.Empty<MasonryGameObject>()
                )
            );
            harness.Runner.Connect();
            FakeSceneHandle retainedHandle = HandleFor(harness, retained);
            harness.AssetStorage.EnqueueSceneLoadPending();
            harness.Transport.EnqueueSubmit(
                Response(
                    session,
                    SceneAssets(retained, added),
                    new[] { retained, added },
                    added.Id,
                    Array.Empty<MasonryGameObject>()
                )
            );

            harness.Runner.Submit(new byte[] { 4 });

            FakeSceneHandle addedHandle = HandleFor(harness, added);
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(addedHandle.IsLoaded, Is.False);
            Assert.That(HandleFor(harness, retained), Is.SameAs(retainedHandle));

            addedHandle.CompleteLoad();
            harness.Runner.RunFrame();

            Assert.That(harness.Runner.IsInputAvailable, Is.True);
            Assert.That(SceneManager.GetActiveScene(), Is.EqualTo(addedHandle.Scene));
        }

        [Test]
        public void FailedSceneLoadReleasesNewAndRetainedSceneHandles()
        {
            MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            MasonryScene retained = ContentScene("game/retained-before-failure");
            MasonryScene failed = ContentScene("game/failed-load");
            try
            {
                harness.Transport.EnqueueConnect(
                    Response(
                        session,
                        SceneAssets(retained),
                        new[] { retained },
                        null,
                        Array.Empty<MasonryGameObject>()
                    )
                );
                harness.Runner.Connect();
                harness.AssetStorage.EnqueueSceneFailure(
                    new InvalidOperationException("load failed")
                );
                harness.Transport.EnqueueSubmit(
                    Response(
                        session,
                        SceneAssets(retained, failed),
                        new[] { retained, failed },
                        retained.Id,
                        Array.Empty<MasonryGameObject>()
                    )
                );

                harness.Runner.Submit(new byte[] { 5 });

                Assert.That(harness.Runner.IsInputAvailable, Is.False);
                Assert.That(
                    harness.AssetStorage.SceneHandles.Select(handle => handle.Asset),
                    Is.EqualTo(new[] { SceneAsset(retained) }),
                    "The failed new load must be released while the old scene finishes unloading."
                );
                Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
                Assert.That(harness.Logger.Records.Last().Message, Does.Contain("load failed"));
            }
            finally
            {
                harness.Dispose();
            }

            Assert.That(harness.AssetStorage.SceneHandles, Is.Empty);
            Assert.That(harness.AssetStorage.LiveHandleCount, Is.Zero);
        }

        [Test]
        public void InvalidSceneSetsFailBeforeLoadingAnything()
        {
            using MasonryTestHarness duplicateHarness = MasonryTestHarness.Create();
            MasonryScene first = ContentScene("game/duplicate");
            MasonryScene duplicate = new(new SceneId(Guid.NewGuid()), first.Address);
            duplicateHarness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: SceneAssets(first),
                    scenes: new[] { first, duplicate },
                    primarySceneId: first.Id
                )
            );

            duplicateHarness.Runner.Connect();

            Assert.That(duplicateHarness.AssetStorage.SceneLoadCalls, Is.Empty);
            Assert.That(duplicateHarness.Transport.Calls.Last(), Is.EqualTo("stop"));

            using MasonryTestHarness limitHarness = MasonryTestHarness.Create();
            MasonryScene[] tooMany = Enumerable
                .Range(0, 33)
                .Select(index => ContentScene($"game/scene-{index}"))
                .ToArray();
            limitHarness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    preparedAssets: tooMany.Select(SceneAsset).ToArray(),
                    scenes: tooMany,
                    primarySceneId: tooMany[0].Id
                )
            );

            limitHarness.Runner.Connect();

            Assert.That(limitHarness.AssetStorage.SceneLoadCalls, Is.Empty);
            Assert.That(limitHarness.Transport.Calls.Last(), Is.EqualTo("stop"));
        }

        private static MasonryScene ContentScene(string address) =>
            new(new SceneId(Guid.NewGuid()), new SceneAddress(address));

        private static PreparedAsset[] SceneAssets(params MasonryScene[] scenes) =>
            scenes.Select(SceneAsset).ToArray();

        private static PreparedAsset SceneAsset(MasonryScene scene) =>
            new PreparedAsset.Scene(scene.Address);

        private static MasonryGameObject EmptyObject(ObjectId id, ParentScene parentScene) =>
            new(
                id,
                new GameObjectKind.Empty(),
                parentScene,
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static MasonryIdentity Identity(ObjectId id) =>
            Object.FindObjectsByType<MasonryIdentity>().Single(identity => identity.Id == id.Value);

        private static FakeSceneHandle HandleFor(MasonryTestHarness harness, MasonryScene scene) =>
            harness.AssetStorage.SceneHandles.Single(handle =>
                handle.Asset.Address == scene.Address
            );

        private static GameObject AuthoredObject(FakeSceneHandle handle) =>
            handle.Scene.GetRootGameObjects().Single(root => root.name.StartsWith("Authored"));

        private static MasonryTransportResult Response(
            SessionId session,
            IReadOnlyList<PreparedAsset> assets,
            IReadOnlyList<MasonryScene> scenes,
            SceneId? primarySceneId,
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
                                scenes: scenes,
                                primarySceneId: primarySceneId,
                                objects: objects
                            )
                        ),
                    }
                )
            );
    }
}
