#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementBatchAdmissionTests
    {
        [Test]
        public void SerializedStartModesAreAdmittedAndOldDuplicatesAreIgnored()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            Batch now = ValidBatch(session, BatchStart.Now);
            Batch dependent = ValidBatch(session, BatchStart.AfterEarlierBlockingWork);
            var messages = new List<ResponseMessage<Command>>
            {
                new ResponseMessage<Command>.BatchMessage(now),
                new ResponseMessage<Command>.BatchMessage(dependent),
            };
            for (int index = 0; index < 64; index++)
            {
                messages.Add(
                    new ResponseMessage<Command>.BatchMessage(ValidBatch(session, BatchStart.Now))
                );
            }
            messages.Add(new ResponseMessage<Command>.BatchMessage(now));

            Connect(harness, session);
            SubmitResponse(harness, new Response(session, messages));

            BattlementLogRecord[] admitted = harness
                .Logger.Records.Where(record => record.EventName == "battlement.batch.admitted")
                .ToArray();
            Assert.That(admitted, Has.Length.EqualTo(66));
            Assert.That(admitted[0].Fields!["start"], Is.EqualTo("Now"));
            Assert.That(admitted[0].Fields, Does.Not.ContainKey("waits_through_sequence"));
            Assert.That(
                admitted[1].Fields!["waits_through_sequence"],
                Is.EqualTo("1"),
                "Dependent admission must capture all earlier blocking work."
            );
            Assert.That(
                harness.Logger.Records.Count(record =>
                    record.EventName == "battlement.batch.duplicate"
                ),
                Is.EqualTo(1)
            );
            Assert.That(harness.Transport.SubmitMessages, Has.Count.EqualTo(1));
        }

        [Test]
        public void BoundarySizedSerializedBatchesAreAccepted()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            Connect(harness, session);

            var groups = Enumerable.Range(0, 256).Select(_ => Group()).ToArray();
            var commands = Enumerable.Range(0, 4_096).Select(_ => ValidCommand()).ToArray();
            SubmitResponse(
                harness,
                BatchResponse(
                    session,
                    new Batch(new BatchId(Guid.NewGuid()), session, groups),
                    new Batch(
                        new BatchId(Guid.NewGuid()),
                        session,
                        new[] { new ParallelCommandGroup<Command>(commands) }
                    )
                )
            );

            Assert.That(
                harness.Logger.Records.Count(record =>
                    record.EventName == "battlement.batch.admitted"
                ),
                Is.EqualTo(2)
            );
            Assert.That(harness.Transport.SubmitMessages, Has.Count.EqualTo(1));
        }

        [Test]
        public void ReconnectClearsTheDuplicateBatchHistory()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId firstSession = new(Guid.NewGuid());
            SessionId secondSession = new(Guid.NewGuid());
            Batch first = ValidBatch(firstSession, BatchStart.Now);
            Connect(harness, firstSession);
            SubmitResponse(harness, BatchResponse(firstSession, first, first));

            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(secondSession)
            );
            harness.Runner.Reconnect();
            Batch reused = first with { SessionId = secondSession };
            SubmitResponse(harness, BatchResponse(secondSession, reused));

            Assert.That(
                harness.Logger.Records.Count(record =>
                    record.EventName == "battlement.batch.admitted"
                ),
                Is.EqualTo(2)
            );
            Assert.That(
                harness.Logger.Records.Count(record =>
                    record.EventName == "battlement.batch.duplicate"
                ),
                Is.EqualTo(1)
            );
        }

        [Test]
        public void BatchValidationFailuresAreSubmittedWithAvailableIdentity()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            Command duplicate = ValidCommand();
            var cases = new (Batch Batch, CoreErrorCode Code, CommandId? CommandId)[]
            {
                (
                    ValidBatch(new SessionId(Guid.NewGuid()), BatchStart.Now),
                    CoreErrorCode.WrongSession,
                    null
                ),
                (
                    new Batch(
                        new BatchId(Guid.NewGuid()),
                        session,
                        Array.Empty<ParallelCommandGroup<Command>>()
                    ),
                    CoreErrorCode.InvalidProperty,
                    null
                ),
                (
                    new Batch(
                        new BatchId(Guid.NewGuid()),
                        session,
                        Enumerable.Range(0, 257).Select(_ => Group()).ToArray()
                    ),
                    CoreErrorCode.LimitExceeded,
                    null
                ),
                (
                    new Batch(
                        new BatchId(Guid.NewGuid()),
                        session,
                        new[] { new ParallelCommandGroup<Command>(Array.Empty<Command>()) }
                    ),
                    CoreErrorCode.InvalidProperty,
                    null
                ),
                (
                    new Batch(
                        new BatchId(Guid.NewGuid()),
                        session,
                        new[]
                        {
                            new ParallelCommandGroup<Command>(
                                Enumerable.Range(0, 4_097).Select(_ => ValidCommand()).ToArray()
                            ),
                        }
                    ),
                    CoreErrorCode.LimitExceeded,
                    null
                ),
                (
                    new Batch(
                        new BatchId(Guid.NewGuid()),
                        session,
                        new[] { new ParallelCommandGroup<Command>(new[] { duplicate, duplicate }) }
                    ),
                    CoreErrorCode.DuplicateId,
                    duplicate.Id
                ),
            };
            Connect(harness, session);

            foreach ((Batch batch, CoreErrorCode code, CommandId? commandId) in cases)
            {
                harness.Transport.EnqueueSubmit(
                    FakeBattlementTransport.ResponseResult(BatchResponse(session, batch))
                );
                harness.Transport.EnqueueSubmit(
                    FakeBattlementTransport.ResponseResult(
                        new Response(session, Array.Empty<ResponseMessage<Command>>())
                    )
                );
                harness.Runner.Submit(new byte[] { 1 });

                var failure = (ClientMessage<CoreErrorCode, byte>.BatchFailedMessage)Decode(
                    harness.Transport.SubmitMessages[^1]
                );
                Assert.That(failure.Failure.BatchId, Is.EqualTo(batch.Id));
                Assert.That(failure.Failure.SessionId, Is.EqualTo(session));
                Assert.That(failure.Failure.ErrorCode, Is.EqualTo(code));
                Assert.That(failure.Failure.CommandId, Is.EqualTo(commandId));
            }

            Assert.That(harness.Transport.Calls, Does.Not.Contain("stop"));
        }

        [Test]
        public void OversizedOrUnorderableSerializedResponsesStopTheSession()
        {
            using BattlementTestHarness oversizedHarness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            Connect(oversizedHarness, session);
            Response oversized = new(
                session,
                Enumerable
                    .Range(0, 257)
                    .Select(_ =>
                        (ResponseMessage<Command>)
                            new ResponseMessage<Command>.BatchMessage(
                                ValidBatch(session, BatchStart.Now)
                            )
                    )
                    .ToArray()
            );
            SubmitResponse(oversizedHarness, oversized);

            Assert.That(oversizedHarness.Runner.IsInputAvailable, Is.False);
            Assert.That(oversizedHarness.Transport.Calls.Last(), Is.EqualTo("stop"));

            using BattlementTestHarness malformedHarness = BattlementTestHarness.Create();
            Connect(malformedHarness, session);
            byte[] malformed = BattlementMessagePack
                .SerializeResponse(BatchResponse(session, ValidBatch(session, BatchStart.Now)))
                .Take(12)
                .ToArray();
            malformedHarness.Transport.EnqueueSubmit(
                new BattlementTransportResult(BattlementTransportStatus.Success, malformed)
            );
            malformedHarness.Runner.Submit(new byte[] { 2 });

            Assert.That(malformedHarness.Runner.IsInputAvailable, Is.False);
            Assert.That(malformedHarness.Transport.Calls.Last(), Is.EqualTo("stop"));
        }

        private static void Connect(BattlementTestHarness harness, SessionId session)
        {
            harness.Transport.EnqueueConnect(FakeBattlementTransport.SnapshotResponse(session));
            harness.Runner.Connect();
        }

        private static void SubmitResponse(BattlementTestHarness harness, Response response)
        {
            harness.Transport.EnqueueSubmit(FakeBattlementTransport.ResponseResult(response));
            harness.Runner.Submit(new byte[] { 1 });
        }

        private static Response BatchResponse(SessionId session, params Batch[] batches) =>
            new(
                session,
                batches
                    .Select(batch =>
                        (ResponseMessage<Command>)new ResponseMessage<Command>.BatchMessage(batch)
                    )
                    .ToArray()
            );

        private static Batch ValidBatch(SessionId session, BatchStart start) =>
            new(new BatchId(Guid.NewGuid()), session, new[] { Group() }, Start: start);

        private static ParallelCommandGroup<Command> Group() => new(new[] { ValidCommand() });

        private static Command ValidCommand() =>
            new(new CommandId(Guid.NewGuid()), new CommandBody.Input.SetEnabled(true));

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
