#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using UnityEngine;

namespace Masonry.Tests
{
    public sealed class MasonrySnapshotValidationTests
    {
        [TestCase("cycle")]
        [TestCase("missing-scene")]
        [TestCase("wrong-asset-kind")]
        [TestCase("disabled-camera")]
        [TestCase("duplicate-material-slot")]
        [TestCase("non-finite-transform")]
        public void MalformedReplacementStopsBeforePreparingOrLoadingAnything(string invalidCase)
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(session, session)
            );
            harness.Runner.Connect();
            int preparedBefore = harness.AssetStorage.PrepareCalls.Count;
            int scenesBefore = harness.AssetStorage.SceneLoadCalls.Count;
            Snapshot invalid = InvalidSnapshot(session, invalidCase);
            harness.Transport.EnqueueSubmit(Response(invalid));

            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(harness.AssetStorage.PrepareCalls.Count, Is.EqualTo(preparedBefore));
            Assert.That(harness.AssetStorage.SceneLoadCalls.Count, Is.EqualTo(scenesBefore));
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("Snapshot validation"));
        }

        [Test]
        public void PrefabRootAndSlotValidationFinishesBeforeSceneTransition()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(session, session)
            );
            harness.Runner.Connect();
            int scenesBefore = harness.AssetStorage.SceneLoadCalls.Count;
            var prefabAddress = new PrefabAddress("game/invalid-root");
            var materialAddress = new MaterialAddress("game/material");
            var sceneAddress = new SceneAddress("game/replacement-scene");
            var sceneId = new SceneId(Guid.NewGuid());
            var cameraId = new ObjectId(Guid.NewGuid());
            var prefab = new GameObject("One-slot prefab");
            var material = new Material(Shader.Find("Universal Render Pipeline/Lit"));
            prefab.AddComponent<MeshRenderer>().sharedMaterial = material;
            harness.AssetStorage.EnqueueValue(prefab);
            harness.AssetStorage.EnqueueValue(material);
            var snapshot = new Snapshot(
                session,
                new PreparedAsset[]
                {
                    new PreparedAsset.Prefab(prefabAddress),
                    new PreparedAsset.Material(materialAddress),
                    new PreparedAsset.Scene(sceneAddress),
                },
                new[] { new MasonryScene(sceneId, sceneAddress) },
                new[]
                {
                    Object(
                        cameraId,
                        new GameObjectKind.Camera(new CameraState()),
                        new ParentScene.Specific(sceneId)
                    ),
                    Object(
                        new ObjectId(Guid.NewGuid()),
                        new GameObjectKind.Prefab(
                            prefabAddress,
                            new[] { new MaterialAssignment(1, materialAddress) }
                        ),
                        new ParentScene.Specific(sceneId)
                    ),
                },
                cameraId
            );
            harness.Transport.EnqueueSubmit(Response(snapshot));

            try
            {
                harness.Runner.Submit(new byte[] { 2 });

                Assert.That(harness.AssetStorage.SceneLoadCalls.Count, Is.EqualTo(scenesBefore));
                Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
                Assert.That(harness.Logger.Records.Last().Message, Does.Contain("material slot"));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(prefab);
                UnityEngine.Object.DestroyImmediate(material);
            }
        }

        [Test]
        public void ObjectCountLimitIsCheckedWithoutSerializingTheFixture()
        {
            SessionId session = new(Guid.NewGuid());
            var objects = Enumerable
                .Range(0, 100_001)
                .Select(_ =>
                    Object(
                        new ObjectId(Guid.NewGuid()),
                        new GameObjectKind.Empty(),
                        new ParentScene.Persistent()
                    )
                )
                .ToArray();
            Snapshot snapshot = FakeMasonryTransport.CompleteSnapshot(session) with
            {
                Objects = objects,
            };
            using MasonryTestHarness harness = MasonryTestHarness.Create(
                protocolCodec: new FixedResponseCodec(
                    new Response(
                        session,
                        new ResponseMessage<Command>[]
                        {
                            new ResponseMessage<Command>.SnapshotMessage(snapshot),
                        }
                    )
                )
            );
            harness.Transport.EnqueueConnect(
                new MasonryTransportResult(MasonryTransportStatus.Success, new byte[] { 1 })
            );

            harness.Runner.Connect();

            Assert.That(harness.AssetStorage.PrepareCalls, Is.Empty);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            Assert.That(harness.Logger.Records.Last().Message, Does.Contain("100000"));
        }

        private static Snapshot InvalidSnapshot(SessionId session, string invalidCase)
        {
            ObjectId first = new(Guid.NewGuid());
            ObjectId second = new(Guid.NewGuid());
            var materialAddress = new MaterialAddress("game/material");
            Snapshot valid = FakeMasonryTransport.CompleteSnapshot(session);
            return invalidCase switch
            {
                "cycle" => valid with
                {
                    Objects = valid
                        .Objects.Concat(
                            new[]
                            {
                                Object(
                                    first,
                                    new GameObjectKind.Empty(),
                                    new ParentScene.Persistent(),
                                    second
                                ),
                                Object(
                                    second,
                                    new GameObjectKind.Empty(),
                                    new ParentScene.Persistent(),
                                    first
                                ),
                            }
                        )
                        .ToArray(),
                },
                "missing-scene" => valid with
                {
                    Objects = valid
                        .Objects.Append(
                            Object(
                                first,
                                new GameObjectKind.Empty(),
                                new ParentScene.Specific(new SceneId(Guid.NewGuid()))
                            )
                        )
                        .ToArray(),
                },
                "wrong-asset-kind" => valid with
                {
                    PreparedAssets = valid
                        .PreparedAssets.Append(new PreparedAsset.Material(materialAddress))
                        .ToArray(),
                    Objects = valid
                        .Objects.Append(
                            Object(
                                first,
                                new GameObjectKind.Image(
                                    new ImageState(new TextureAddress(materialAddress.Value), 1, 1)
                                ),
                                new ParentScene.Persistent()
                            )
                        )
                        .ToArray(),
                },
                "disabled-camera" => valid with
                {
                    InputCameraId = first,
                    Objects = valid
                        .Objects.Append(
                            Object(
                                first,
                                new GameObjectKind.Camera(
                                    new CameraState() with
                                    {
                                        IsEnabled = false,
                                    }
                                ),
                                new ParentScene.Persistent()
                            )
                        )
                        .ToArray(),
                },
                "duplicate-material-slot" => valid with
                {
                    PreparedAssets = valid
                        .PreparedAssets.Append(new PreparedAsset.Material(materialAddress))
                        .ToArray(),
                    Objects = valid
                        .Objects.Append(
                            Object(
                                first,
                                new GameObjectKind.Cube(
                                    new[]
                                    {
                                        new MaterialAssignment(0, materialAddress),
                                        new MaterialAssignment(0, materialAddress),
                                    }
                                ),
                                new ParentScene.Persistent()
                            )
                        )
                        .ToArray(),
                },
                "non-finite-transform" => valid with
                {
                    Objects = valid
                        .Objects.Append(
                            new MasonryGameObject(
                                first,
                                new GameObjectKind.Empty(),
                                new ParentScene.Persistent(),
                                null,
                                true,
                                new LocalTransform(
                                    new Vector3(double.NaN, 0, 0),
                                    Quaternion.Identity,
                                    Vector3.One
                                ),
                                Array.Empty<PointerEvent>()
                            )
                        )
                        .ToArray(),
                },
                _ => throw new ArgumentOutOfRangeException(nameof(invalidCase)),
            };
        }

        private static MasonryGameObject Object(
            ObjectId id,
            GameObjectKind kind,
            ParentScene scene,
            ObjectId? parentId = null
        ) =>
            new(
                id,
                kind,
                scene,
                parentId,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static MasonryTransportResult Response(Snapshot snapshot) =>
            FakeMasonryTransport.ResponseResult(
                new Response(
                    snapshot.SessionId,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.SnapshotMessage(snapshot),
                    }
                )
            );

        private sealed class FixedResponseCodec : IMasonryProtocolCodec
        {
            private readonly Response response;

            public FixedResponseCodec(Response response) => this.response = response;

            public byte[] SerializeConnect(Connect value) => new byte[] { 1 };

            public byte[] SerializeBatchFailure(BatchFailed<CoreErrorCode> value) =>
                throw new NotSupportedException();

            public byte[] SerializeOperationFailure(OperationFailed<CoreErrorCode> value) =>
                throw new NotSupportedException();

            public byte[] SerializeAction(Action value) => throw new NotSupportedException();

            public Response DeserializeResponse(ReadOnlyMemory<byte> bytes) => response;
        }
    }
}
