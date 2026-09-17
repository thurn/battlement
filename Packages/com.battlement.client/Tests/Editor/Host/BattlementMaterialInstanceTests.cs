#nullable enable
using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementMaterialInstanceTests
    {
        [Test]
        public void SnapshotAndCommandsKeepOverridesIndependentAndRestoreSource()
        {
            Material shared = null!;
            try
            {
                using BattlementTestHarness harness = BattlementTestHarness.Create();
                shared = new Material(Shader.Find("Unlit/Color"));
                shared.SetColor("_Color", UnityEngine.Color.white);
                var address = new MaterialAddress("test/shared");
                var session = new SessionId(Guid.NewGuid());
                var left = new ObjectId(Guid.NewGuid());
                var right = new ObjectId(Guid.NewGuid());
                harness.AssetStorage.EnqueueValue(shared);
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(
                        responseSession: session,
                        preparedAssets: new PreparedAsset[]
                        {
                            new PreparedAsset.MaterialParameters(
                                address,
                                new[]
                                {
                                    new MaterialParameterDeclaration(
                                        "_Color",
                                        MaterialParameterKind.Color
                                    ),
                                }
                            ),
                        },
                        objects: new[] { Card(left, address, 1, 0), Card(right, address, 0, 1) }
                    )
                );
                harness.Runner.Connect();
                Renderer first = Identity(left).GetComponent<Renderer>();
                Renderer second = Identity(right).GetComponent<Renderer>();
                Assert.That(first.sharedMaterial, Is.SameAs(shared));
                Assert.That(second.sharedMaterial, Is.SameAs(shared));
                Assert.That(Color(first), Is.EqualTo(UnityEngine.Color.red));
                Assert.That(Color(second), Is.EqualTo(UnityEngine.Color.green));
                Submit(
                    harness,
                    session,
                    new CommandBody.Renderer.SetInstances(
                        left,
                        new[] { Instance(address, 0, 0, 1) }
                    )
                );
                Assert.That(Identity(left).GetComponent<Renderer>(), Is.SameAs(first));
                Assert.That(Color(first), Is.EqualTo(UnityEngine.Color.blue));
                Assert.That(Color(second), Is.EqualTo(UnityEngine.Color.green));
                Assert.That(shared.GetColor("_Color"), Is.EqualTo(UnityEngine.Color.white));
                Submit(
                    harness,
                    session,
                    new CommandBody.Renderer.SetInstances(left, Array.Empty<MaterialInstance>())
                );
                var block = new MaterialPropertyBlock();
                first.GetPropertyBlock(block, 0);
                Assert.That(block.isEmpty, Is.True);
                Assert.That(first.sharedMaterial, Is.SameAs(shared));
                Assert.That(Color(second), Is.EqualTo(UnityEngine.Color.green));
                Assert.That(
                    first.GetComponents<BattlementMaterialInstances>().Length,
                    Is.EqualTo(1)
                );
            }
            finally
            {
                Object.DestroyImmediate(shared);
            }
        }

        [TestCase("_Missing", MaterialParameterKind.Float)]
        [TestCase("_Color", MaterialParameterKind.Float)]
        public void PreparationRejectsMissingOrWrongShaderDeclarationBeforeCreatingObjects(
            string name,
            MaterialParameterKind kind
        )
        {
            Material shared = null!;
            try
            {
                using BattlementTestHarness harness = BattlementTestHarness.Create();
                shared = new Material(Shader.Find("Unlit/Color"));
                shared.SetColor("_Color", UnityEngine.Color.white);
                var address = new MaterialAddress("test/shared");
                var id = new ObjectId(Guid.NewGuid());
                harness.AssetStorage.EnqueueValue(shared);
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(
                        preparedAssets: new PreparedAsset[]
                        {
                            new PreparedAsset.MaterialParameters(
                                address,
                                new[] { new MaterialParameterDeclaration(name, kind) }
                            ),
                        },
                        objects: new[] { Card(id, address, 1, 0) }
                    )
                );
                harness.Runner.Connect();
                Assert.That(
                    Object.FindObjectsByType<BattlementIdentity>().Any(o => o.Id == id.Value),
                    Is.False
                );
                Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
                Assert.That(harness.Logger.Records.Last().Message, Does.Contain(name));
            }
            finally
            {
                Object.DestroyImmediate(shared);
            }
        }

        [TestCase(false, false)]
        [TestCase(true, false)]
        [TestCase(false, true)]
        [TestCase(true, true)]
        public void UndeclaredOverrideFailsBeforeSnapshotOrCommandPlayback(
            bool partial,
            bool command
        )
        {
            Material shared = null!;
            try
            {
                using BattlementTestHarness harness = BattlementTestHarness.Create();
                shared = new Material(Shader.Find("Sprites/Default"));
                shared.SetColor("_Color", UnityEngine.Color.white);
                var address = new MaterialAddress("test/shared");
                var id = new ObjectId(Guid.NewGuid());
                var session = new SessionId(Guid.NewGuid());
                PreparedAsset declaration = partial
                    ? new PreparedAsset.MaterialParameters(
                        address,
                        new[]
                        {
                            new MaterialParameterDeclaration(
                                "PixelSnap",
                                MaterialParameterKind.Float
                            ),
                        }
                    )
                    : new PreparedAsset.Material(address);
                BattlementGameObject card = Card(id, address, 1, 0);
                if (command)
                    card = card with { MaterialInstances = Array.Empty<MaterialInstance>() };
                harness.AssetStorage.EnqueueValue(shared);
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(
                        responseSession: session,
                        preparedAssets: new[] { declaration },
                        objects: new[] { card }
                    )
                );
                harness.Runner.Connect();
                if (command)
                {
                    Renderer renderer = Identity(id).GetComponent<Renderer>();
                    Submit(
                        harness,
                        session,
                        new CommandBody.Renderer.SetInstances(
                            id,
                            new[] { Instance(address, 1, 0, 0) }
                        )
                    );
                    var block = new MaterialPropertyBlock();
                    renderer.GetPropertyBlock(block, 0);
                    Assert.That(block.isEmpty, Is.True);
                    Assert.That(renderer.sharedMaterial, Is.SameAs(shared));
                }
                else
                    Assert.That(
                        Object.FindObjectsByType<BattlementIdentity>().Any(o => o.Id == id.Value),
                        Is.False
                    );
                Assert.That(shared.GetColor("_Color"), Is.EqualTo(UnityEngine.Color.white));
                Assert.That(
                    harness.Logger.Records.Any(record =>
                        record.Message.Contains("_Color") && record.Message.Contains("declared")
                    ),
                    Is.True
                );
            }
            finally
            {
                Object.DestroyImmediate(shared);
            }
        }

        private static BattlementGameObject Card(
            ObjectId id,
            MaterialAddress address,
            double r,
            double g
        ) =>
            new BattlementGameObject(
                id,
                new GameObjectKind.Quad(new[] { new MaterialAssignment(0, address) })
            )
            {
                ParentScene = new ParentScene.Persistent(),
                MaterialInstances = new[] { Instance(address, r, g, 0) },
            };

        private static MaterialInstance Instance(
            MaterialAddress address,
            double r,
            double g,
            double b
        ) =>
            new(
                address,
                0,
                new[]
                {
                    new MaterialParameterValue("_Color", MaterialParameterKind.Color, r, g, b, 1),
                }
            );

        private static UnityEngine.Color Color(Renderer renderer)
        {
            var block = new MaterialPropertyBlock();
            renderer.GetPropertyBlock(block, 0);
            return block.GetColor("_Color");
        }

        private static BattlementIdentity Identity(ObjectId id) =>
            Object.FindObjectsByType<BattlementIdentity>().Single(o => o.Id == id.Value);

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
