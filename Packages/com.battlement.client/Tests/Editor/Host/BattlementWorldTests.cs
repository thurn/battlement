#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementWorldTests
    {
        [Test]
        public void InitialSnapshotCreatesPersistentIdentitiesAndResolvesChildHits()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var unrelated = new GameObject("Unrelated bootstrap object");
            var parentId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            BattlementGameObject[] objects =
            {
                PersistentObject(parentId),
                PersistentObject(childId, parentId),
            };
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(objects: objects)
            );

            harness.Runner.Connect();

            BattlementIdentity[] identities = UserIdentities();
            Assert.That(
                identities.Select(identity => identity.Id),
                Is.EquivalentTo(new[] { parentId.Value, childId.Value })
            );
            BattlementIdentity parent = identities.Single(identity =>
                identity.Id == parentId.Value
            );
            BattlementIdentity child = identities.Single(identity => identity.Id == childId.Value);
            Assert.That(child.transform.parent, Is.EqualTo(parent.transform));
            Assert.That(parent.transform.parent.name, Is.EqualTo("Battlement Persistent"));

            var authoredChild = new GameObject("Authored collider child");
            authoredChild.transform.SetParent(parent.transform, false);
            Assert.That(BattlementIdentity.FindNearest(authoredChild), Is.SameAs(parent));

            harness.Runner.Dispose();

            Assert.That(unrelated != null, Is.True);
            Assert.That(Object.FindObjectsByType<BattlementIdentity>(), Is.Empty);
            Object.DestroyImmediate(unrelated);
        }

        [Test]
        public void DuplicateObjectUuidFailsBeforeChangingTheHierarchy()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var unrelated = new GameObject("Unrelated bootstrap object");
            var duplicateId = new ObjectId(Guid.NewGuid());
            BattlementGameObject[] objects =
            {
                PersistentObject(duplicateId),
                PersistentObject(duplicateId),
            };
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(objects: objects)
            );

            harness.Runner.Connect();

            Assert.That(Object.FindObjectsByType<BattlementIdentity>(), Is.Empty);
            Assert.That(unrelated != null, Is.True);
            Assert.That(
                harness.Logger.Records.Last().EventName,
                Is.EqualTo("battlement.session.failed")
            );
            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect", "stop" }));
            Object.DestroyImmediate(unrelated);
        }

        [Test]
        public void ReconnectRemovesDestroyedReferencesAndStartsANewUuidHistory()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var objectId = new ObjectId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    objects: new[] { PersistentObject(objectId) }
                )
            );
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    objects: new[] { PersistentObject(objectId) }
                )
            );

            harness.Runner.Connect();
            BattlementIdentity first = UserIdentities().Single();
            var detachedHit = new GameObject("Detached hit");
            Object.DestroyImmediate(first.gameObject);

            Assert.That(BattlementIdentity.FindNearest(detachedHit), Is.Null);

            harness.Runner.Reconnect();

            BattlementIdentity second = UserIdentities().Single();
            Assert.That(second.Id, Is.EqualTo(objectId.Value));
            Assert.That(second, Is.Not.SameAs(first));
            Object.DestroyImmediate(detachedHit);
        }

        [Test]
        public void ReplacementRecreatesEveryObjectAndAppliesFinalHierarchyAndValues()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            var parentId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    session,
                    objects: new[] { PersistentObject(parentId), PersistentObject(childId) }
                )
            );
            harness.Runner.Connect();
            BattlementIdentity oldParent = Identity(parentId);
            BattlementIdentity oldChild = Identity(childId);
            FakeSceneHandle sceneHandle = harness.AssetStorage.SceneHandles.Single();
            var childTransform = new LocalTransform(
                new Vector3(4, 5, 6),
                Quaternion.Identity,
                new Vector3(2, 3, 4)
            );
            harness.Transport.EnqueueSubmit(
                FakeBattlementTransport.SnapshotResponse(
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

            BattlementIdentity parent = Identity(parentId);
            BattlementIdentity child = Identity(childId);
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
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            var objectId = new ObjectId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    session,
                    objects: new[] { PersistentObject(objectId) }
                )
            );
            harness.Runner.Connect();
            BattlementIdentity initial = Identity(objectId);
            Snapshot first = FakeBattlementTransport.CompleteSnapshot(
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
            Snapshot second = FakeBattlementTransport.CompleteSnapshot(
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
                FakeBattlementTransport.ResponseResult(
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

            BattlementIdentity final = Identity(objectId);
            Assert.That(final, Is.Not.SameAs(initial));
            Assert.That(final.transform.localPosition.x, Is.EqualTo(2));
            Assert.That(UserIdentities(), Has.Length.EqualTo(1));
        }

        private static BattlementGameObject PersistentObject(
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

        private static BattlementIdentity Identity(ObjectId id) =>
            UserIdentities().Single(identity => identity.Id == id.Value);

        private static BattlementIdentity[] UserIdentities() =>
            Object
                .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Include)
                .Where(identity => !FakeBattlementTransport.IsFixtureIdentity(identity))
                .ToArray();
    }
}
