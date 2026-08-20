#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Masonry.Tests
{
    public sealed class MasonryWorldTests
    {
        [Test]
        public void InitialSnapshotCreatesPersistentIdentitiesAndResolvesChildHits()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var unrelated = new GameObject("Unrelated bootstrap object");
            var parentId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            MasonryGameObject[] objects =
            {
                PersistentObject(parentId),
                PersistentObject(childId, parentId),
            };
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(objects: objects)
            );

            harness.Runner.Connect();

            MasonryIdentity[] identities = UserIdentities();
            Assert.That(
                identities.Select(identity => identity.Id),
                Is.EquivalentTo(new[] { parentId.Value, childId.Value })
            );
            MasonryIdentity parent = identities.Single(identity => identity.Id == parentId.Value);
            MasonryIdentity child = identities.Single(identity => identity.Id == childId.Value);
            Assert.That(child.transform.parent, Is.EqualTo(parent.transform));
            Assert.That(parent.transform.parent.name, Is.EqualTo("Masonry Persistent"));

            var authoredChild = new GameObject("Authored collider child");
            authoredChild.transform.SetParent(parent.transform, false);
            Assert.That(MasonryIdentity.FindNearest(authoredChild), Is.SameAs(parent));

            harness.Runner.Dispose();

            Assert.That(unrelated != null, Is.True);
            Assert.That(Object.FindObjectsByType<MasonryIdentity>(), Is.Empty);
            Object.DestroyImmediate(unrelated);
        }

        [Test]
        public void DuplicateObjectUuidFailsBeforeChangingTheHierarchy()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var unrelated = new GameObject("Unrelated bootstrap object");
            var duplicateId = new ObjectId(Guid.NewGuid());
            MasonryGameObject[] objects =
            {
                PersistentObject(duplicateId),
                PersistentObject(duplicateId),
            };
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(objects: objects)
            );

            harness.Runner.Connect();

            Assert.That(Object.FindObjectsByType<MasonryIdentity>(), Is.Empty);
            Assert.That(unrelated != null, Is.True);
            Assert.That(
                harness.Logger.Records.Last().EventName,
                Is.EqualTo("masonry.session.failed")
            );
            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect", "stop" }));
            Object.DestroyImmediate(unrelated);
        }

        [Test]
        public void ReconnectRemovesDestroyedReferencesAndStartsANewUuidHistory()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var objectId = new ObjectId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(objects: new[] { PersistentObject(objectId) })
            );
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(objects: new[] { PersistentObject(objectId) })
            );

            harness.Runner.Connect();
            MasonryIdentity first = UserIdentities().Single();
            var detachedHit = new GameObject("Detached hit");
            Object.DestroyImmediate(first.gameObject);

            Assert.That(MasonryIdentity.FindNearest(detachedHit), Is.Null);

            harness.Runner.Reconnect();

            MasonryIdentity second = UserIdentities().Single();
            Assert.That(second.Id, Is.EqualTo(objectId.Value));
            Assert.That(second, Is.Not.SameAs(first));
            Object.DestroyImmediate(detachedHit);
        }

        [Test]
        public void ReplacementRecreatesEveryObjectAndAppliesFinalHierarchyAndValues()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            var parentId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    session,
                    session,
                    objects: new[] { PersistentObject(parentId), PersistentObject(childId) }
                )
            );
            harness.Runner.Connect();
            MasonryIdentity oldParent = Identity(parentId);
            MasonryIdentity oldChild = Identity(childId);
            FakeSceneHandle sceneHandle = harness.AssetStorage.SceneHandles.Single();
            var childTransform = new LocalTransform(
                new Vector3(4, 5, 6),
                Quaternion.Identity,
                new Vector3(2, 3, 4)
            );
            harness.Transport.EnqueueSubmit(
                FakeMasonryTransport.SnapshotResponse(
                    session,
                    session,
                    objects: new[]
                    {
                        PersistentObject(childId, parentId, childTransform, false),
                        PersistentObject(parentId),
                    }
                )
            );

            harness.Runner.Submit(new byte[] { 1 });

            MasonryIdentity parent = Identity(parentId);
            MasonryIdentity child = Identity(childId);
            Assert.That(parent, Is.Not.SameAs(oldParent));
            Assert.That(child, Is.Not.SameAs(oldChild));
            Assert.That(child.transform.parent, Is.EqualTo(parent.transform));
            Assert.That(
                child.transform.localPosition,
                Is.EqualTo(new UnityEngine.Vector3(4, 5, 6))
            );
            Assert.That(child.transform.localScale, Is.EqualTo(new UnityEngine.Vector3(2, 3, 4)));
            Assert.That(child.gameObject.activeSelf, Is.False);
            Assert.That(harness.AssetStorage.SceneHandles.Single(), Is.SameAs(sceneHandle));
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
        }

        [Test]
        public void LaterSnapshotWaitsForReplacementAndDoesNotReuseIntermediateObjects()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            var objectId = new ObjectId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    session,
                    session,
                    objects: new[] { PersistentObject(objectId) }
                )
            );
            harness.Runner.Connect();
            MasonryIdentity initial = Identity(objectId);
            Snapshot first = FakeMasonryTransport.CompleteSnapshot(
                session,
                objects: new[]
                {
                    PersistentObject(
                        objectId,
                        transform: new LocalTransform(
                            new Vector3(1, 0, 0),
                            Quaternion.Identity,
                            Vector3.One
                        )
                    ),
                }
            );
            Snapshot second = FakeMasonryTransport.CompleteSnapshot(
                session,
                objects: new[]
                {
                    PersistentObject(
                        objectId,
                        transform: new LocalTransform(
                            new Vector3(2, 0, 0),
                            Quaternion.Identity,
                            Vector3.One
                        )
                    ),
                }
            );
            harness.Transport.EnqueueSubmit(
                FakeMasonryTransport.ResponseResult(
                    new Response(
                        session,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.SnapshotMessage(first),
                            new ResponseMessage<Command>.SnapshotMessage(second),
                        }
                    )
                )
            );

            harness.Runner.Submit(new byte[] { 2 });

            MasonryIdentity final = Identity(objectId);
            Assert.That(final, Is.Not.SameAs(initial));
            Assert.That(final.transform.localPosition.x, Is.EqualTo(2));
            Assert.That(UserIdentities(), Has.Length.EqualTo(1));
        }

        private static MasonryGameObject PersistentObject(
            ObjectId id,
            ObjectId? parentId = null,
            LocalTransform? transform = null,
            bool active = true
        ) =>
            new(
                id,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                parentId,
                active,
                transform ?? LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static MasonryIdentity Identity(ObjectId id) =>
            UserIdentities().Single(identity => identity.Id == id.Value);

        private static MasonryIdentity[] UserIdentities() =>
            Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Include)
                .Where(identity => !FakeMasonryTransport.IsFixtureIdentity(identity))
                .ToArray();
    }
}
