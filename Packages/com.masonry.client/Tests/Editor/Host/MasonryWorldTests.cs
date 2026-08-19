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

            MasonryIdentity[] identities = Object.FindObjectsByType<MasonryIdentity>();
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
            MasonryIdentity first = Object.FindAnyObjectByType<MasonryIdentity>();
            var detachedHit = new GameObject("Detached hit");
            Object.DestroyImmediate(first.gameObject);

            Assert.That(MasonryIdentity.FindNearest(detachedHit), Is.Null);

            harness.Runner.Reconnect();

            MasonryIdentity second = Object.FindAnyObjectByType<MasonryIdentity>();
            Assert.That(second.Id, Is.EqualTo(objectId.Value));
            Assert.That(second, Is.Not.SameAs(first));
            Object.DestroyImmediate(detachedHit);
        }

        private static MasonryGameObject PersistentObject(ObjectId id, ObjectId? parentId = null) =>
            new(
                id,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                parentId,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );
    }
}
