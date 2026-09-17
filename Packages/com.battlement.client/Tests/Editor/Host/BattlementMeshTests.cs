#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementMeshTests
    {
        [Test]
        public void SnapshotAndQueuedCreationSharePreparedGeometryAndReleaseUsage()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            GameObject source = GameObject.CreatePrimitive(PrimitiveType.Cube);
            Mesh geometry = Object.Instantiate(source.GetComponent<MeshFilter>().sharedMesh);
            Object.DestroyImmediate(source);
            var material = new Material(Shader.Find("Universal Render Pipeline/Lit"));
            var mesh = new PreparedAsset.Mesh(new MeshAddress("game/mesh"));
            var paint = new PreparedAsset.Material(new MaterialAddress("game/material"));
            var first = new ObjectId(Guid.NewGuid());
            var second = new ObjectId(Guid.NewGuid());
            var session = new SessionId(Guid.NewGuid());
            try
            {
                harness.AssetStorage.EnqueueValue(geometry);
                harness.AssetStorage.EnqueueValue(material);
                harness.Transport.EnqueueConnect(
                    FakeBattlementTransport.SnapshotResponse(
                        responseSession: session,
                        preparedAssets: new PreparedAsset[] { mesh, paint },
                        objects: new[] { Description(first, mesh.Address, paint.Address) }
                    )
                );
                harness.Runner.Connect();
                Assert.That(
                    harness.Runner.IsInputAvailable,
                    Is.True,
                    string.Join("\n", harness.Logger.Records.Select(value => value.Message))
                );
                GameObject instance = Identity(first).gameObject;
                Assert.That(instance.GetComponent<MeshFilter>().sharedMesh, Is.SameAs(geometry));
                Assert.That(
                    instance.GetComponent<MeshRenderer>().sharedMaterial,
                    Is.SameAs(material)
                );
                Assert.That(
                    instance.transform.localScale,
                    Is.EqualTo(new UnityEngine.Vector3(2, 3, 4))
                );
                Assert.That(
                    instance.transform.localRotation,
                    Is.EqualTo(new UnityEngine.Quaternion(0, 1, 0, 0))
                );
                Assert.That(instance.GetComponent<MeshCollider>().sharedMesh, Is.SameAs(geometry));

                Submit(
                    harness,
                    session,
                    new CommandBody.Object.Create(Description(second, mesh.Address, paint.Address)),
                    new CommandBody.Object.Destroy(first)
                );
                Assert.That(
                    Identity(second).GetComponent<MeshFilter>().sharedMesh,
                    Is.SameAs(geometry)
                );
                Assert.That(
                    harness.AssetStorage.PrepareCalls.Count(value => value == mesh),
                    Is.EqualTo(1)
                );
                Assert.That(
                    harness.AssetStorage.PrepareCalls.Count(value => value == paint),
                    Is.EqualTo(1)
                );
                FakeAssetHandle retained = harness.AssetStorage.Handles.Single(value =>
                    value.Asset == mesh
                );
                Assert.That(retained.IsDisposed, Is.False);

                Submit(
                    harness,
                    session,
                    new CommandBody.Object.Destroy(second),
                    new CommandBody.Assets.ReplaceSet(
                        new PreparedAsset[]
                        {
                            new PreparedAsset.Scene(
                                new SceneAddress("battlement/tests/default-scene")
                            ),
                        }
                    )
                );
                Assert.That(retained.IsDisposed, Is.True);
                Assert.That(geometry != null, Is.True);
            }
            finally
            {
                Object.DestroyImmediate(geometry);
                Object.DestroyImmediate(material);
            }
        }

        [TestCase(false)]
        [TestCase(true)]
        public void MissingOrWrongTypedMeshRejectsSnapshotBeforePublishingObjects(bool declared)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            var address = new MeshAddress("game/missing");
            if (declared)
                harness.AssetStorage.EnqueueFailure(
                    new BattlementAssetException(
                        CoreErrorCode.AssetTypeMismatch,
                        "mesh asset has the wrong Unity type"
                    )
                );
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    preparedAssets: declared
                        ? new PreparedAsset[] { new PreparedAsset.Mesh(address) }
                        : Array.Empty<PreparedAsset>(),
                    objects: new[]
                    {
                        new BattlementGameObject(
                            new ObjectId(Guid.NewGuid()),
                            new GameObjectKind.Mesh(address, Array.Empty<MaterialAssignment>()),
                            new ParentScene.Persistent(),
                            null,
                            true,
                            LocalTransform.Identity,
                            Array.Empty<PointerEvent>()
                        ),
                    }
                )
            );
            harness.Runner.Connect();
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(Object.FindObjectsByType<BattlementIdentity>(), Is.Empty);
        }

        private static BattlementGameObject Description(
            ObjectId id,
            MeshAddress mesh,
            MaterialAddress material
        ) =>
            new(
                id,
                new GameObjectKind.Mesh(mesh, new[] { new MaterialAssignment(0, material) }),
                new ParentScene.Persistent(),
                null,
                true,
                new LocalTransform(
                    Battlement.Vector3.Zero,
                    new Battlement.Quaternion(0, 1, 0, 0),
                    new Battlement.Vector3(2, 3, 4)
                ),
                new[] { PointerEvent.Click }
            );

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
