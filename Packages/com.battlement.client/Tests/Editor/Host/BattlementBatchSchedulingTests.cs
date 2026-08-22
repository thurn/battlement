#nullable enable

using System;
using System.Linq;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;
using Object = UnityEngine.Object;

namespace Battlement.Tests
{
    public sealed class BattlementBatchSchedulingTests
    {
        [Test]
        public void GroupsAdvanceAtBlockingTimeWithoutWaitingForNonblockingWork()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = Connect(harness);
            var firstId = new ObjectId(Guid.NewGuid());
            var thirdGroupId = new ObjectId(Guid.NewGuid());
            var dependentId = new ObjectId(Guid.NewGuid());
            Batch timeline = BatchWithGroups(
                session,
                BatchStart.Now,
                Group(Create(firstId)),
                Group(
                    Wait(TimeSpan.FromMilliseconds(300)),
                    Wait(TimeSpan.FromMilliseconds(800)).Nonblocking()
                ),
                Group(Create(thirdGroupId), SetLocalPosition(thirdGroupId, new Vector3(3, 0, 0)))
            );
            Batch dependent = BatchWithGroups(
                session,
                BatchStart.AfterEarlierBlockingWork,
                Group(Create(dependentId))
            );

            SubmitResponse(harness, Response(session, timeline, dependent));

            Assert.That(HasIdentity(firstId), Is.True, "Group 1 must execute at 0 ms.");
            Assert.That(HasIdentity(thirdGroupId), Is.False);
            Assert.That(HasIdentity(dependentId), Is.False);

            harness.Clock.Advance(TimeSpan.FromMilliseconds(299));
            harness.Runner.RunFrame();

            Assert.That(HasIdentity(thirdGroupId), Is.False);
            Assert.That(HasIdentity(dependentId), Is.False);

            harness.Clock.Advance(TimeSpan.FromMilliseconds(1));
            harness.Runner.RunFrame();

            Assert.That(Identity(thirdGroupId).transform.localPosition.x, Is.EqualTo(3));
            Assert.That(HasIdentity(dependentId), Is.True);
            Assert.That(
                harness.Clock.Elapsed,
                Is.EqualTo(TimeSpan.FromMilliseconds(300)),
                "The 800 ms nonblocking wait must not delay the next group or dependent batch."
            );
        }

        [Test]
        public void CommandsResolveTargetsWhenTheirGroupActuallyRuns()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = Connect(harness);
            var objectId = new ObjectId(Guid.NewGuid());
            Batch delayedTarget = BatchWithGroups(
                session,
                BatchStart.Now,
                Group(Wait(TimeSpan.FromMilliseconds(300))),
                Group(SetLocalPosition(objectId, new Vector3(7, 0, 0)))
            );
            Batch independentCreate = BatchWithGroups(
                session,
                BatchStart.Now,
                Group(Create(objectId))
            );

            SubmitResponse(harness, Response(session, delayedTarget, independentCreate));

            Assert.That(
                Identity(objectId).transform.localPosition,
                Is.EqualTo(UnityEngine.Vector3.zero)
            );
            harness.Clock.Advance(TimeSpan.FromMilliseconds(300));
            harness.Runner.RunFrame();

            Assert.That(Identity(objectId).transform.localPosition.x, Is.EqualTo(7));
            Assert.That(Failures(harness), Is.Empty);
        }

        [Test]
        public void FirstFailureKeepsPartialEffectsAndPropagatesUntilANowBatch()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = Connect(harness);
            var retainedId = new ObjectId(Guid.NewGuid());
            var missingId = new ObjectId(Guid.NewGuid());
            var skippedId = new ObjectId(Guid.NewGuid());
            var dependentOneId = new ObjectId(Guid.NewGuid());
            var dependentTwoId = new ObjectId(Guid.NewGuid());
            var independentId = new ObjectId(Guid.NewGuid());
            Command failing = SetLocalPosition(missingId, new Vector3(1, 0, 0));
            Batch failed = BatchWithGroups(
                session,
                BatchStart.Now,
                Group(Create(retainedId), failing, Create(skippedId))
            );
            Batch dependentOne = BatchWithGroups(
                session,
                BatchStart.AfterEarlierBlockingWork,
                Group(Create(dependentOneId))
            );
            Batch dependentTwo = BatchWithGroups(
                session,
                BatchStart.AfterEarlierBlockingWork,
                Group(Create(dependentTwoId))
            );
            Batch independent = BatchWithGroups(
                session,
                BatchStart.Now,
                Group(Create(independentId))
            );
            harness.Transport.EnqueueSubmit(
                FakeBattlementTransport.ResponseResult(
                    Response(session, failed, dependentOne, dependentTwo, independent)
                )
            );
            for (int index = 0; index < 3; index++)
            {
                harness.Transport.EnqueueSubmit(
                    FakeBattlementTransport.ResponseResult(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    )
                );
            }

            harness.Runner.Submit(new byte[] { 1 });

            Assert.That(HasIdentity(retainedId), Is.True);
            Assert.That(HasIdentity(skippedId), Is.False);
            Assert.That(HasIdentity(dependentOneId), Is.False);
            Assert.That(HasIdentity(dependentTwoId), Is.False);
            Assert.That(HasIdentity(independentId), Is.True);
            BatchFailed<CoreErrorCode>[] failures = Failures(harness);
            Assert.That(
                failures.Select(failure => failure.BatchId),
                Is.EqualTo(new[] { failed.Id, dependentOne.Id, dependentTwo.Id })
            );
            Assert.That(failures[0].ErrorCode, Is.EqualTo(CoreErrorCode.UnknownObject));
            Assert.That(failures[0].CommandId, Is.EqualTo(failing.Id));
            Assert.That(
                failures.Skip(1).Select(failure => failure.ErrorCode),
                Is.All.EqualTo(CoreErrorCode.EarlierBatchFailed)
            );
        }

        private static SessionId Connect(BattlementTestHarness harness)
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.Connect();
            return session;
        }

        private static void SubmitResponse(BattlementTestHarness harness, Response response)
        {
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            harness.Runner.Submit(new byte[] { 1 });
        }

        private static Response Response(SessionId session, params Batch[] batches) =>
            new(
                session,
                batches
                    .Select(batch =>
                        (ResponseMessage<Command>)new ResponseMessage<Command>.BatchMessage(batch)
                    )
                    .ToArray()
            );

        private static Batch BatchWithGroups(
            SessionId session,
            BatchStart start,
            params ParallelCommandGroup<Command>[] groups
        ) => new(new BatchId(Guid.NewGuid()), session, groups, Start: start);

        private static ParallelCommandGroup<Command> Group(params Command[] commands) =>
            new(commands);

        private static Command Create(ObjectId id) =>
            new(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Object.Create(
                    new BattlementGameObject(
                        id,
                        new GameObjectKind.Empty(),
                        new ParentScene.Persistent(),
                        null,
                        true,
                        LocalTransform.Identity,
                        Array.Empty<PointerEvent>()
                    )
                )
            );

        private static Command SetLocalPosition(ObjectId id, Vector3 position) =>
            new(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Transform.SetLocalPosition(id, position)
            );

        private static Command Wait(TimeSpan duration) =>
            new(new CommandId(Guid.NewGuid()), new CommandBody.Time.Wait(duration));

        private static bool HasIdentity(ObjectId id) =>
            Object.FindObjectsByType<BattlementIdentity>().Any(identity => identity.Id == id.Value);

        private static BattlementIdentity Identity(ObjectId id) =>
            Object
                .FindObjectsByType<BattlementIdentity>()
                .Single(identity => identity.Id == id.Value);

        private static BatchFailed<CoreErrorCode>[] Failures(BattlementTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Skip(1)
                .Select(Decode)
                .OfType<ClientMessage<CoreErrorCode, byte>.BatchFailedMessage>()
                .Select(message => message.Failure)
                .ToArray();

        private static ClientMessage<CoreErrorCode, byte> Decode(byte[] bytes) =>
            BattlementMessagePack.DeserializeClientMessage(
                bytes,
                new CoreErrorFormatter(),
                new UnusedPayloadFormatter()
            );

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
