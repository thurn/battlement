#nullable enable

using System;
using System.Linq;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;
using UQuaternion = UnityEngine.Quaternion;
using UVector3 = UnityEngine.Vector3;

namespace Masonry.Tests
{
    public sealed class MasonryTransformCommandTests
    {
        [Test]
        public void ImmediateCommandsApplyLocalAndWorldValuesWithNormalizedRotations()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var parentId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(
                harness,
                Empty(
                    parentId,
                    transform: new LocalTransform(
                        new Vector3(10, 0, 0),
                        Quaternion.Identity,
                        new Vector3(2, 2, 2)
                    )
                ),
                Empty(childId, parentId)
            );
            Transform parent = Find(parentId);
            Transform child = Find(childId);

            Submit(
                harness,
                session,
                Body(new CommandBody.Transform.SetWorldPosition(childId, new Vector3(16, 0, 0)))
            );
            Assert.That(child.localPosition.x, Is.EqualTo(3).Within(0.0001f));

            Submit(
                harness,
                session,
                Body(new CommandBody.Transform.SetLocalPosition(childId, new Vector3(4, 1, -2))),
                Body(new CommandBody.Transform.SetLocalScale(childId, new Vector3(2, 3, 4)))
            );
            AssertVector(child.position, new UVector3(18, 2, -4));
            AssertVector(child.localScale, new UVector3(2, 3, 4));

            parent.rotation = UQuaternion.Euler(0, 0, 30);
            UQuaternion worldRotation = UQuaternion.Euler(0, 0, 90);
            Submit(
                harness,
                session,
                Body(
                    new CommandBody.Transform.SetWorldRotation(
                        childId,
                        ProtocolRotation(worldRotation, 4)
                    )
                )
            );
            Assert.That(UQuaternion.Angle(child.rotation, worldRotation), Is.Zero.Within(0.001f));
            Assert.That(
                UQuaternion.Angle(child.localRotation, UQuaternion.Euler(0, 0, 60)),
                Is.Zero.Within(0.001f)
            );

            UQuaternion localRotation = UQuaternion.Euler(20, 10, -15);
            Submit(
                harness,
                session,
                Body(
                    new CommandBody.Transform.SetLocalRotation(
                        childId,
                        ProtocolRotation(localRotation, 3)
                    )
                )
            );
            Assert.That(
                UQuaternion.Angle(child.localRotation, localRotation),
                Is.Zero.Within(0.001f)
            );
            Assert.That(
                UQuaternion.Dot(child.localRotation, child.localRotation),
                Is.EqualTo(1).Within(0.0001f)
            );
        }

        [Test]
        public void LocalAndWorldTweensConflictContinuouslyWhileScaleRemainsIndependent()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create(
                useInstantAnimations: false
            );
            var parentId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(
                harness,
                Empty(
                    parentId,
                    transform: new LocalTransform(
                        new Vector3(10, 0, 0),
                        Quaternion.Identity,
                        Vector3.One
                    )
                ),
                Empty(childId, parentId)
            );
            Transform child = Find(childId);
            Tween linear = new(
                TimeSpan.FromSeconds(1),
                TimeSpan.Zero,
                Easing.Linear,
                new TweenRepeat.Once()
            );

            Submit(
                harness,
                session,
                Body(
                        new CommandBody.Transform.TweenLocalPosition(
                            childId,
                            new Vector3(10, 0, 0),
                            linear
                        )
                    )
                    .Nonblocking(),
                Body(
                        new CommandBody.Transform.TweenLocalScale(
                            childId,
                            new Vector3(3, 3, 3),
                            linear
                        )
                    )
                    .Nonblocking()
            );
            Advance(harness, 400);
            Assert.That(child.localPosition.x, Is.EqualTo(4).Within(0.001f));
            Assert.That(child.localScale.x, Is.EqualTo(1.8f).Within(0.001f));

            Submit(
                harness,
                session,
                Body(
                        new CommandBody.Transform.TweenWorldPosition(
                            childId,
                            new Vector3(18, 0, 0),
                            linear
                        )
                    )
                    .Nonblocking(),
                Body(new CommandBody.Transform.SetLocalScale(childId, new Vector3(5, 5, 5)))
            );
            Advance(harness, 100);

            Assert.That(child.position.x, Is.EqualTo(14.4f).Within(0.001f));
            AssertVector(child.localScale, new UVector3(5, 5, 5));
            Advance(harness, 900);
            Assert.That(child.position.x, Is.EqualTo(18).Within(0.001f));
        }

        [Test]
        public void ReparentCancelsEveryRootTransformOperationAtDisplayedValues()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create(
                useInstantAnimations: false
            );
            var firstParentId = new ObjectId(Guid.NewGuid());
            var secondParentId = new ObjectId(Guid.NewGuid());
            var childId = new ObjectId(Guid.NewGuid());
            SessionId session = Connect(
                harness,
                Empty(firstParentId),
                Empty(
                    secondParentId,
                    transform: new LocalTransform(
                        new Vector3(20, 0, 0),
                        Quaternion.Identity,
                        Vector3.One
                    )
                ),
                Empty(childId, firstParentId)
            );
            Transform child = Find(childId);
            Tween linear = new(
                TimeSpan.FromSeconds(1),
                TimeSpan.Zero,
                Easing.Linear,
                new TweenRepeat.Once()
            );
            Submit(
                harness,
                session,
                Body(
                        new CommandBody.Transform.TweenLocalPosition(
                            childId,
                            new Vector3(10, 0, 0),
                            linear
                        )
                    )
                    .Nonblocking(),
                Body(
                        new CommandBody.Transform.TweenLocalRotation(
                            childId,
                            ProtocolRotation(UQuaternion.Euler(0, 90, 0)),
                            linear
                        )
                    )
                    .Nonblocking(),
                Body(
                        new CommandBody.Transform.TweenLocalScale(
                            childId,
                            new Vector3(3, 3, 3),
                            linear
                        )
                    )
                    .Nonblocking()
            );
            Advance(harness, 300);
            child.GetPositionAndRotation(
                out UVector3 displayedPosition,
                out UQuaternion displayedRotation
            );
            UVector3 displayedScale = child.lossyScale;

            Submit(
                harness,
                session,
                Body(new CommandBody.Object.Reparent(childId, secondParentId, true))
            );
            Advance(harness, 1_000);

            Assert.That(child.parent, Is.SameAs(Find(secondParentId)));
            AssertVector(child.position, displayedPosition);
            Assert.That(
                UQuaternion.Angle(child.rotation, displayedRotation),
                Is.Zero.Within(0.001f)
            );
            AssertVector(child.lossyScale, displayedScale);
        }

        [TestCase(0)]
        [TestCase(1)]
        [TestCase(2)]
        [TestCase(3)]
        public void BillboardControlledImagesRejectEveryRotationCommand(int variant)
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            var texture = new Texture2D(2, 2);
            var address = new TextureAddress("game/transform-billboard");
            var imageId = new ObjectId(Guid.NewGuid());
            harness.AssetStorage.EnqueueValue(texture);
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(
                    session,
                    preparedAssets: new PreparedAsset[] { new PreparedAsset.Texture(address) },
                    objects: new[]
                    {
                        new MasonryGameObject(
                            imageId,
                            new GameObjectKind.Image(
                                new ImageState(
                                    address,
                                    2,
                                    2,
                                    ImageFit.Stretch,
                                    RgbColor.White,
                                    1,
                                    true
                                )
                            ),
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

            Submit(harness, session, RotationCommand(variant, imageId), reportsFailure: true);

            Assert.That(
                Failures(harness).Single().ErrorCode,
                Is.EqualTo(CoreErrorCode.PropertyControlledByBillboard)
            );
            Object.DestroyImmediate(texture);
        }

        private static SessionId Connect(
            MasonryTestHarness harness,
            params MasonryGameObject[] objects
        )
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(session, objects: objects)
            );
            harness.Runner.Connect();
            return session;
        }

        private static MasonryGameObject Empty(
            ObjectId id,
            ObjectId? parentId = null,
            LocalTransform? transform = null
        ) =>
            new(
                id,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                parentId,
                true,
                transform ?? LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static Command Body(CommandBody body) => new(new CommandId(Guid.NewGuid()), body);

        private static Command RotationCommand(int variant, ObjectId objectId)
        {
            var rotation = new Quaternion(0, 0, 1, 1);
            Tween tween = new(TimeSpan.FromSeconds(1));
            CommandBody body = variant switch
            {
                0 => new CommandBody.Transform.SetLocalRotation(objectId, rotation),
                1 => new CommandBody.Transform.SetWorldRotation(objectId, rotation),
                2 => new CommandBody.Transform.TweenLocalRotation(objectId, rotation, tween),
                3 => new CommandBody.Transform.TweenWorldRotation(objectId, rotation, tween),
                _ => throw new ArgumentOutOfRangeException(nameof(variant)),
            };
            return Body(body);
        }

        private static Quaternion ProtocolRotation(UQuaternion value, double scale = 1) =>
            new(value.x * scale, value.y * scale, value.z * scale, value.w * scale);

        private static void Submit(
            MasonryTestHarness harness,
            SessionId session,
            params Command[] commands
        ) => Submit(harness, session, commands, false);

        private static void Submit(
            MasonryTestHarness harness,
            SessionId session,
            Command command,
            bool reportsFailure
        ) => Submit(harness, session, new[] { command }, reportsFailure);

        private static void Submit(
            MasonryTestHarness harness,
            SessionId session,
            Command[] commands,
            bool reportsFailure
        )
        {
            var batch = new Batch(
                new BatchId(Guid.NewGuid()),
                session,
                new[] { new ParallelCommandGroup<Command>(commands) }
            );
            var response = new Response(
                session,
                new ResponseMessage<Command>[] { new ResponseMessage<Command>.BatchMessage(batch) }
            );
            harness.Transport.EnqueueSubmit(FakeMasonryTransport.ResponseResult(response));
            if (reportsFailure)
            {
                harness.Transport.EnqueueSubmit(
                    FakeMasonryTransport.ResponseResult(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    )
                );
            }

            harness.Runner.Submit(new byte[] { 1 });
        }

        private static void Advance(MasonryTestHarness harness, double milliseconds)
        {
            harness.Clock.Advance(TimeSpan.FromMilliseconds(milliseconds));
            harness.Runner.RunFrame();
        }

        private static Transform Find(ObjectId id) =>
            Object
                .FindObjectsByType<MasonryIdentity>(FindObjectsInactive.Include)
                .Single(value => value.Id == id.Value)
                .transform;

        private static void AssertVector(UVector3 actual, UVector3 expected)
        {
            Assert.That(actual.x, Is.EqualTo(expected.x).Within(0.001f));
            Assert.That(actual.y, Is.EqualTo(expected.y).Within(0.001f));
            Assert.That(actual.z, Is.EqualTo(expected.z).Within(0.001f));
        }

        private static BatchFailed<CoreErrorCode>[] Failures(MasonryTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Where(bytes => bytes.Length > 1)
                .Select(bytes =>
                    MasonryMessagePack.DeserializeClientMessage(
                        bytes,
                        new CoreErrorFormatter(),
                        new UnusedPayloadFormatter()
                    )
                )
                .OfType<ClientMessage<CoreErrorCode, byte>.BatchFailedMessage>()
                .Select(message => message.Failure)
                .ToArray();

        private sealed class CoreErrorFormatter : IMessagePackFormatter<CoreErrorCode>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                CoreErrorCode value,
                MessagePackSerializerOptions options
            ) => writer.Write(value.ToString());

            public CoreErrorCode Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => Enum.Parse<CoreErrorCode>(reader.ReadString()!);
        }

        private sealed class UnusedPayloadFormatter : IMessagePackFormatter<byte>
        {
            public void Serialize(
                ref MessagePackWriter writer,
                byte value,
                MessagePackSerializerOptions options
            ) => writer.Write(value);

            public byte Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => reader.ReadByte();
        }
    }
}
