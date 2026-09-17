#nullable enable
using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.Rendering;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementRenderOrderTests
    {
        [Test]
        public void SnapshotAndQueuedOrdersPreserveNestedHierarchyAndRestoreDefaults()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var session = new SessionId(Guid.NewGuid());
            var groupId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            var nestedId = new ObjectId(Guid.NewGuid());
            BattlementGameObject group = new(groupId, new GameObjectKind.Empty());
            BattlementGameObject child = new(
                childId,
                new GameObjectKind.Cube(Array.Empty<MaterialAssignment>())
            );
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    responseSession: session,
                    objects: new[]
                    {
                        group with
                        {
                            ParentScene = new ParentScene.Persistent(),
                            RenderOrder = new RenderOrder(RenderOrderKind.Group, 12),
                        },
                        child with
                        {
                            ParentScene = new ParentScene.Persistent(),
                            ParentId = groupId,
                            RenderOrder = new RenderOrder(RenderOrderKind.Layer, -3),
                        },
                        new BattlementGameObject(nestedId, new GameObjectKind.Empty()) with
                        {
                            ParentScene = new ParentScene.Persistent(),
                            ParentId = groupId,
                            RenderOrder = new RenderOrder(RenderOrderKind.Group, 2),
                        },
                    }
                )
            );
            harness.Runner.Connect();
            Assert.That(
                harness.Runner.IsInputAvailable,
                Is.True,
                string.Join("\n", harness.Logger.Records.Select(value => value.Message))
            );
            GameObject parent = Identity(groupId).gameObject;
            GameObject visual = Identity(childId).gameObject;
            SortingGroup nested = Identity(nestedId).GetComponent<SortingGroup>();
            Assert.That(parent.GetComponent<SortingGroup>().sortingOrder, Is.EqualTo(12));
            Assert.That(nested.sortAtRoot, Is.False);
            Assert.That(nested.sortingOrder, Is.EqualTo(2));
            Assert.That(visual.GetComponent<Renderer>().sortingOrder, Is.EqualTo(-3));
            Assert.That(visual.transform.parent, Is.SameAs(parent.transform));
            Submit(
                harness,
                session,
                new CommandBody.Object.SetRenderOrder(groupId, null),
                new CommandBody.Object.SetRenderOrder(
                    childId,
                    new RenderOrder(RenderOrderKind.Layer, 7)
                )
            );
            Assert.That(Identity(childId).gameObject, Is.SameAs(visual));
            Assert.That(parent.GetComponent<SortingGroup>().enabled, Is.False);
            Assert.That(visual.GetComponent<Renderer>().sortingOrder, Is.EqualTo(7));
            Submit(
                harness,
                session,
                new CommandBody.Object.SetRenderOrder(
                    groupId,
                    new RenderOrder(RenderOrderKind.Group, -4)
                ),
                new CommandBody.Object.SetRenderOrder(childId, null)
            );
            Assert.That(parent.GetComponents<SortingGroup>().Length, Is.EqualTo(1));
            Assert.That(parent.GetComponent<SortingGroup>().enabled, Is.True);
            Assert.That(parent.GetComponent<SortingGroup>().sortingOrder, Is.EqualTo(-4));
            Assert.That(visual.GetComponent<Renderer>().sortingOrder, Is.Zero);
        }

        [Test]
        public void ClearingOverridesRestoresAuthoredGroupAndRenderer()
        {
            GameObject owner = GameObject.CreatePrimitive(PrimitiveType.Quad);
            try
            {
                SortingGroup group = owner.AddComponent<SortingGroup>();
                group.sortingOrder = 14;
                group.sortAtRoot = true;
                owner.GetComponent<Renderer>().sortingOrder = 6;
                BattlementRenderOrder.Apply(owner, new RenderOrder(RenderOrderKind.Group, 2));
                Assert.That(group.sortAtRoot, Is.False);
                BattlementRenderOrder.Apply(owner, new RenderOrder(RenderOrderKind.Layer, 9));
                Assert.That(group.sortingOrder, Is.EqualTo(14));
                Assert.That(group.sortAtRoot, Is.True);
                Assert.That(owner.GetComponent<Renderer>().sortingOrder, Is.EqualTo(9));
                BattlementRenderOrder.Apply(owner, null);
                Assert.That(group.enabled, Is.True);
                Assert.That(owner.GetComponent<Renderer>().sortingOrder, Is.EqualTo(6));
            }
            finally
            {
                Object.DestroyImmediate(owner);
            }
        }

        private static BattlementIdentity Identity(ObjectId id) =>
            Object.FindObjectsByType<BattlementIdentity>().Single(value => value.Id == id.Value);

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            params CommandBody[] bodies
        )
        {
            var group = new ParallelCommandGroup<Command>(
                bodies.Select(body => new Command(new CommandId(Guid.NewGuid()), body)).ToArray()
            );
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[] { group },
                Start: BatchStart.Now
            );
            var response = new Response(
                session,
                new ResponseMessage<Command>[] { new ResponseMessage<Command>.BatchMessage(batch) }
            );
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            harness.Runner.Submit(new byte[] { 1 });
        }
    }
}
