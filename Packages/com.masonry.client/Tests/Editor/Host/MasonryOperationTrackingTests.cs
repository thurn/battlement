#nullable enable

using System;
using System.Linq;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;
using Object = UnityEngine.Object;

namespace Masonry.Tests
{
    public sealed class MasonryOperationTrackingTests
    {
        [Test]
        public void CancelStopsRunningWaitAndKnownCompletedCommandIsANoOp()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = Connect(harness);
            var objectId = new ObjectId(Guid.NewGuid());
            Command wait = Wait(TimeSpan.FromHours(1));
            Command create = Create(objectId);
            Batch batch = BatchWithGroups(
                session,
                Group(wait, Cancel(wait.Id)),
                Group(create),
                Group(Cancel(create.Id))
            );

            Submit(harness, Response(session, batch));

            Assert.That(HasIdentity(objectId), Is.True);
            Assert.That(Failures(harness), Is.Empty);
        }

        [Test]
        public void CancelNeverExecutedCommandFailsTheWaitingBatch()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = Connect(harness);
            var unknown = new CommandId(Guid.NewGuid());
            Command cancel = Cancel(unknown);
            Batch batch = BatchWithGroups(session, Group(cancel));
            SubmitExpectingFailure(harness, Response(session, batch));

            BatchFailed<CoreErrorCode> failure = Failures(harness).Single();
            Assert.That(failure.BatchId, Is.EqualTo(batch.Id));
            Assert.That(failure.CommandId, Is.EqualTo(cancel.Id));
            Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.UnknownCommand));
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
        }

        [Test]
        public void WaitCannotExceedOneDay()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = Connect(harness);
            Command wait = Wait(TimeSpan.FromDays(1) + TimeSpan.FromMilliseconds(1));
            Batch batch = BatchWithGroups(session, Group(wait));

            SubmitExpectingFailure(harness, Response(session, batch));

            BatchFailed<CoreErrorCode> failure = Failures(harness).Single();
            Assert.That(failure.CommandId, Is.EqualTo(wait.Id));
            Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.LimitExceeded));
        }

        [Test]
        public void CommandIdentityCannotBeExecutedTwiceAcrossBatches()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = Connect(harness);
            Command wait = Wait(TimeSpan.FromMilliseconds(1));
            Submit(harness, Response(session, BatchWithGroups(session, Group(wait))));
            harness.Clock.Advance(TimeSpan.FromMilliseconds(1));
            harness.Runner.RunFrame();
            Batch repeated = BatchWithGroups(session, Group(wait));
            SubmitExpectingFailure(harness, Response(session, repeated));

            BatchFailed<CoreErrorCode> failure = Failures(harness).Single();
            Assert.That(failure.BatchId, Is.EqualTo(repeated.Id));
            Assert.That(failure.CommandId, Is.EqualTo(wait.Id));
            Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.DuplicateId));
        }

        [Test]
        public void SnapshotCancelsOldWorkButRetainsExecutedCommandHistory()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = Connect(harness);
            var staleObject = new ObjectId(Guid.NewGuid());
            Command wait = Wait(TimeSpan.FromHours(1));
            Batch stale = BatchWithGroups(session, Group(wait), Group(Create(staleObject)));
            Submit(harness, Response(session, stale));

            Snapshot snapshot = FakeMasonryTransport.CompleteSnapshot(session);
            Submit(
                harness,
                new Response(
                    session,
                    new ResponseMessage<Command>[]
                    {
                        new ResponseMessage<Command>.SnapshotMessage(snapshot),
                    }
                )
            );
            Batch proveKnown = BatchWithGroups(session, Group(Cancel(wait.Id)));
            Submit(harness, Response(session, proveKnown));
            harness.Clock.Advance(TimeSpan.FromHours(2));
            harness.Runner.RunFrame();

            Assert.That(HasIdentity(staleObject), Is.False);
            Assert.That(Failures(harness), Is.Empty);
        }

        [Test]
        public void ReconnectCancelsWorkAndStartsFreshCommandHistory()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId firstSession = Connect(harness);
            Command wait = Wait(TimeSpan.FromHours(1));
            Submit(harness, Response(firstSession, BatchWithGroups(firstSession, Group(wait))));

            var secondSession = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeMasonryTransport.SnapshotResponse(secondSession));
            harness.Runner.Reconnect();
            Batch cancelOld = BatchWithGroups(secondSession, Group(Cancel(wait.Id)));
            SubmitExpectingFailure(harness, Response(secondSession, cancelOld));

            BatchFailed<CoreErrorCode> failure = Failures(harness).Single();
            Assert.That(failure.BatchId, Is.EqualTo(cancelOld.Id));
            Assert.That(failure.ErrorCode, Is.EqualTo(CoreErrorCode.UnknownCommand));
        }

        private static SessionId Connect(MasonryTestHarness harness)
        {
            var session = new SessionId(Guid.NewGuid());
            harness.Transport.EnqueueConnect(FakeMasonryTransport.SnapshotResponse(session));
            harness.Runner.Connect();
            return session;
        }

        private static void Submit(MasonryTestHarness harness, Response response)
        {
            harness.Transport.EnqueueSubmit(FakeMasonryTransport.ResponseResult(response));
            harness.Runner.Submit(new byte[] { 1 });
        }

        private static void SubmitExpectingFailure(MasonryTestHarness harness, Response response)
        {
            harness.Transport.EnqueueSubmit(FakeMasonryTransport.ResponseResult(response));
            harness.Transport.EnqueueSubmit(
                FakeMasonryTransport.ResponseResult(
                    new Response(response.SessionId, Array.Empty<ResponseMessage<Command>>())
                )
            );
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
            params ParallelCommandGroup<Command>[] groups
        ) => new(new BatchId(Guid.NewGuid()), session, groups);

        private static ParallelCommandGroup<Command> Group(params Command[] commands) =>
            new(commands);

        private static Command Wait(TimeSpan duration) =>
            new(new CommandId(Guid.NewGuid()), new CommandBody.Time.Wait(duration));

        private static Command Cancel(CommandId target) =>
            new(new CommandId(Guid.NewGuid()), new CommandBody.Operation.Cancel(target));

        private static Command Create(ObjectId id) =>
            new(
                new CommandId(Guid.NewGuid()),
                new CommandBody.Object.Create(
                    new MasonryGameObject(
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

        private static bool HasIdentity(ObjectId id) =>
            Object.FindObjectsByType<MasonryIdentity>().Any(identity => identity.Id == id.Value);

        private static BatchFailed<CoreErrorCode>[] Failures(MasonryTestHarness harness) =>
            harness
                .Transport.SubmitMessages.Select(TryDecode)
                .OfType<ClientMessage<CoreErrorCode, byte>.BatchFailedMessage>()
                .Select(message => message.Failure)
                .ToArray();

        private static ClientMessage<CoreErrorCode, byte>? TryDecode(byte[] bytes)
        {
            try
            {
                return Decode(bytes);
            }
            catch (MessagePackSerializationException)
            {
                return null;
            }
        }

        private static ClientMessage<CoreErrorCode, byte> Decode(byte[] bytes) =>
            MasonryMessagePack.DeserializeClientMessage(
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
