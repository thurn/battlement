#nullable enable

using System;
using System.Linq;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;
using UnityEngine;
using Object = UnityEngine.Object;
using UQuaternion = UnityEngine.Quaternion;

namespace Battlement.Tests
{
    public sealed class BattlementTweenTests
    {
        [Test]
        public void DelayAppliesOnceAndTweenUsesTheInjectedUnscaledClock()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, Transform target) = ConnectObject(harness);
            float originalTimeScale = Time.timeScale;
            try
            {
                Time.timeScale = 0f;
                Submit(
                    harness,
                    session,
                    TweenPosition(
                        objectId,
                        8,
                        new Tween(
                            TimeSpan.FromMilliseconds(800),
                            TimeSpan.FromMilliseconds(200),
                            Easing.Linear,
                            new TweenRepeat.Once()
                        )
                    )
                );

                Advance(harness, 100);
                Assert.That(target.localPosition.x, Is.Zero.Within(0.001f));
                Advance(harness, 500);
                Assert.That(target.localPosition.x, Is.EqualTo(4f).Within(0.001f));
                Advance(harness, 400);
                Assert.That(target.localPosition.x, Is.EqualTo(8f).Within(0.001f));
            }
            finally
            {
                Time.timeScale = originalTimeScale;
            }
        }

        [TestCase(Easing.InQuad, 0.25f)]
        [TestCase(Easing.OutQuad, 0.75f)]
        [TestCase(Easing.InOutSine, 0.5f)]
        [TestCase(Easing.OutBounce, 0.765625f)]
        public void EasingNamesProducePrimeTweenSamples(Easing easing, float expected)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, Transform target) = ConnectObject(harness);
            Submit(
                harness,
                session,
                TweenPosition(
                    objectId,
                    1,
                    new Tween(
                        TimeSpan.FromSeconds(1),
                        TimeSpan.Zero,
                        easing,
                        new TweenRepeat.Once()
                    )
                )
            );

            Advance(harness, 500);

            Assert.That(target.localPosition.x, Is.EqualTo(expected).Within(0.0001f));
        }

        [Test]
        public void RestartAndPingPongCountAdditionalTraversals()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            var restartId = new ObjectId(Guid.NewGuid());
            var pingPongId = new ObjectId(Guid.NewGuid());
            SessionId session = ConnectObjects(harness, restartId, pingPongId);
            Transform restart = Find(restartId);
            Transform pingPong = Find(pingPongId);
            Submit(
                harness,
                session,
                TweenPosition(restartId, 10, LinearRepeat(2, RepeatMode.Restart)).Nonblocking(),
                TweenPosition(pingPongId, 10, LinearRepeat(1, RepeatMode.PingPong)).Nonblocking()
            );

            Advance(harness, 150);
            Assert.That(restart.localPosition.x, Is.EqualTo(5f).Within(0.001f));
            Assert.That(pingPong.localPosition.x, Is.EqualTo(5f).Within(0.001f));
            Advance(harness, 50);
            Assert.That(restart.localPosition.x, Is.Zero.Within(0.001f));
            Assert.That(pingPong.localPosition.x, Is.Zero.Within(0.001f));
            Advance(harness, 100);
            Assert.That(restart.localPosition.x, Is.EqualTo(10f).Within(0.001f));
        }

        [Test]
        public void RotationNormalizesTheTargetAndUsesTheShortestArc()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, Transform target) = ConnectObject(harness);
            UQuaternion requested = UQuaternion.Euler(0f, 270f, 0f);
            var protocolRotation = new Quaternion(
                requested.x * 2,
                requested.y * 2,
                requested.z * 2,
                requested.w * 2
            );
            Submit(
                harness,
                session,
                new Command(
                    new CommandId(Guid.NewGuid()),
                    new CommandBody.Transform.TweenLocalRotation(
                        objectId,
                        protocolRotation,
                        new Tween(TimeSpan.FromSeconds(1))
                    )
                )
            );

            Advance(harness, 500);
            Assert.That(
                UQuaternion.Angle(UQuaternion.identity, target.localRotation),
                Is.EqualTo(45f).Within(0.01f)
            );
            Advance(harness, 500);
            Assert.That(UQuaternion.Angle(requested, target.localRotation), Is.Zero.Within(0.01f));
            Assert.That(
                UQuaternion.Dot(target.localRotation, target.localRotation),
                Is.EqualTo(1f).Within(0.0001f)
            );
        }

        [Test]
        public void CancellationLeavesTheCurrentlyDisplayedValue()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, Transform target) = ConnectObject(harness);
            Command tween = TweenPosition(objectId, 10, new Tween(TimeSpan.FromSeconds(1)))
                .Nonblocking();
            Submit(harness, session, tween);
            Advance(harness, 400);
            float displayed = target.localPosition.x;

            Submit(
                harness,
                session,
                new Command(
                    new CommandId(Guid.NewGuid()),
                    new CommandBody.Operation.Cancel(tween.Id)
                )
            );
            Advance(harness, 600);

            Assert.That(displayed, Is.EqualTo(3.454915f).Within(0.001f));
            Assert.That(target.localPosition.x, Is.EqualTo(displayed).Within(0.001f));
        }

        [Test]
        public void TargetDestructionCancelsBlockingTweenAndAdvancesItsBatch()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, Transform target) = ConnectObject(harness);
            var markerId = new ObjectId(Guid.NewGuid());
            Batch batch = new(
                new BatchId(Guid.NewGuid()),
                session,
                new[]
                {
                    Group(TweenPosition(objectId, 10, new Tween(TimeSpan.FromSeconds(10)))),
                    Group(Create(markerId)),
                }
            );
            Submit(harness, session, batch);

            Object.DestroyImmediate(target.gameObject);
            harness.Runner.RunFrame();

            Assert.That(FindOptional(markerId), Is.Not.Null);
        }

        [Test]
        public void InvalidRepeatFormsFailTheirBatch()
        {
            AssertInvalid(
                new Tween(
                    TimeSpan.FromSeconds(1),
                    TimeSpan.Zero,
                    Easing.Linear,
                    new TweenRepeat.Forever(RepeatMode.Restart)
                ),
                blocking: true,
                CoreErrorCode.InvalidProperty
            );
            AssertInvalid(
                new Tween(
                    TimeSpan.Zero,
                    TimeSpan.Zero,
                    Easing.Linear,
                    new TweenRepeat.Count(1, RepeatMode.Restart)
                ),
                blocking: false,
                CoreErrorCode.InvalidProperty
            );
            AssertInvalid(
                new Tween(
                    TimeSpan.FromSeconds(1),
                    TimeSpan.Zero,
                    Easing.Linear,
                    new TweenRepeat.Count(10_001, RepeatMode.Restart)
                ),
                blocking: false,
                CoreErrorCode.LimitExceeded
            );
        }

        [Test]
        public void InstantModePreservesGroupCompletionOrder()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            (SessionId session, ObjectId objectId, Transform target) = ConnectObject(harness);
            Batch batch = new(
                new BatchId(Guid.NewGuid()),
                session,
                new[]
                {
                    Group(TweenPosition(objectId, 10, new Tween(TimeSpan.FromSeconds(1)))),
                    Group(TweenPosition(objectId, 20, new Tween(TimeSpan.FromSeconds(1)))),
                }
            );

            Submit(harness, session, batch);

            Assert.That(target.localPosition.x, Is.EqualTo(20f).Within(0.001f));
        }

        private static (SessionId, ObjectId, Transform) ConnectObject(BattlementTestHarness harness)
        {
            var objectId = new ObjectId(Guid.NewGuid());
            SessionId session = ConnectObjects(harness, objectId);
            return (session, objectId, Find(objectId));
        }

        private static SessionId ConnectObjects(
            BattlementTestHarness harness,
            params ObjectId[] objectIds
        )
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(
                    session,
                    objects: objectIds.Select(GameObject).ToArray()
                )
            );
            harness.Runner.Connect();
            return session;
        }

        private static BattlementGameObject GameObject(ObjectId id) =>
            new(
                id,
                new GameObjectKind.Empty(),
                new ParentScene.Persistent(),
                null,
                true,
                LocalTransform.Identity,
                Array.Empty<PointerEvent>()
            );

        private static Command TweenPosition(ObjectId id, double x, Tween tween) =>
            new(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Transform.TweenLocalPosition(id, new Vector3(x, 0, 0), tween)
            );

        private static Tween LinearRepeat(uint additional, RepeatMode mode) =>
            new(
                TimeSpan.FromMilliseconds(100),
                TimeSpan.Zero,
                Easing.Linear,
                new TweenRepeat.Count(additional, mode)
            );

        private static Command Create(ObjectId id) =>
            new(new CommandId(Guid.NewGuid()), new CommandBody.Object.Create(GameObject(id)));

        private static ParallelCommandGroup<Command> Group(params Command[] commands) =>
            new(commands);

        private static void Submit(
            BattlementTestHarness harness,
            SessionId session,
            params Command[] commands
        ) =>
            Submit(
                harness,
                session,
                new Batch(new BatchId(Guid.NewGuid()), session, new[] { Group(commands) })
            );

        private static void Submit(BattlementTestHarness harness, SessionId session, Batch batch)
        {
            var response = new Response(
                session,
                new ResponseMessage<Command>[] { new ResponseMessage<Command>.BatchMessage(batch) }
            );
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            harness.Runner.Submit(new byte[] { 1 });
        }

        private static void Advance(BattlementTestHarness harness, double milliseconds)
        {
            harness.Clock.Advance(TimeSpan.FromMilliseconds(milliseconds));
            harness.Runner.RunFrame();
        }

        private static Transform Find(ObjectId id)
        {
            Transform? result = FindOptional(id);
            return result != null
                ? result
                : throw new AssertionException($"Object {id} was not found.");
        }

        private static Transform? FindOptional(ObjectId id)
        {
            BattlementIdentity? identity = Object
                .FindObjectsByType<BattlementIdentity>(FindObjectsInactive.Include)
                .SingleOrDefault(item => item.Id == id.Value);
            return identity != null ? identity.transform : null;
        }

        private static void AssertInvalid(Tween tween, bool blocking, CoreErrorCode expected)
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                useInstantAnimations: false
            );
            (SessionId session, ObjectId objectId, _) = ConnectObject(harness);
            Command command = TweenPosition(objectId, 1, tween) with { IsBlocking = blocking };
            var response = new Response(
                session,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.BatchMessage(
                        new Batch(new BatchId(Guid.NewGuid()), session, new[] { Group(command) })
                    ),
                }
            );
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            harness.Transport.EnqueueSubmit(
                FakeBattlementTransport.ResponseResult(
                    new Response(session, Array.Empty<ResponseMessage<Command>>())
                )
            );
            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(Failures(harness).Single().ErrorCode, Is.EqualTo(expected));
        }

        private static BatchFailed<CoreErrorCode>[] Failures(BattlementTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Select(TryDecode)
                .OfType<ClientMessage<CoreErrorCode, byte>.BatchFailedMessage>()
                .Select(message => message.Failure)
                .ToArray();

        private static ClientMessage<CoreErrorCode, byte>? TryDecode(byte[] bytes)
        {
            try
            {
                return BattlementMessagePack.DeserializeClientMessage(
                    bytes,
                    new CoreErrorFormatter(),
                    new UnusedPayloadFormatter()
                );
            }
            catch (MessagePackSerializationException)
            {
                return null;
            }
        }

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
            ) => throw new NotSupportedException();

            public byte Deserialize(
                ref MessagePackReader reader,
                MessagePackSerializerOptions options
            ) => throw new NotSupportedException();
        }
    }
}
