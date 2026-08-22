#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using MessagePack;
using MessagePack.Formatters;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementFailureSubmissionTests
    {
        [Test]
        public void BatchAndOperationFailuresPreserveIdsCodesAndBoundedDiagnostics()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.Parse("10000000-0000-0000-0000-000000000001"));
            BatchId batch = new(Guid.Parse("20000000-0000-0000-0000-000000000002"));
            CommandId command = new(Guid.Parse("30000000-0000-0000-0000-000000000003"));
            harness.Transport.EnqueueConnect(SnapshotResponse(session, inputDisabled: false));
            harness.Runner.Connect();

            string oversized = string.Concat(Enumerable.Repeat("😀", 20_000));
            harness.Transport.EnqueueSubmit(SnapshotResponse(session, inputDisabled: false));
            harness.Runner.ReportBatchFailure(
                new BatchFailed<CoreErrorCode>(
                    session,
                    batch,
                    CoreErrorCode.AssetNotPrepared,
                    oversized,
                    command
                )
            );

            var batchMessage = (ClientMessage<CoreErrorCode, byte>.BatchFailedMessage)Decode(
                harness.Transport.SubmitMessages[0]
            );
            Assert.That(batchMessage.Failure.SessionId, Is.EqualTo(session));
            Assert.That(batchMessage.Failure.BatchId, Is.EqualTo(batch));
            Assert.That(batchMessage.Failure.CommandId, Is.EqualTo(command));
            Assert.That(batchMessage.Failure.ErrorCode, Is.EqualTo(CoreErrorCode.AssetNotPrepared));
            Assert.That(
                Encoding.UTF8.GetByteCount(batchMessage.Failure.Message),
                Is.EqualTo(65_536)
            );

            harness.Transport.EnqueueSubmit(SnapshotResponse(session, inputDisabled: false));
            harness.Runner.ReportOperationFailure(
                new OperationFailed<CoreErrorCode>(
                    session,
                    batch,
                    command,
                    CoreErrorCode.UnityException,
                    "particle callback failed"
                )
            );

            var operationMessage = (ClientMessage<
                CoreErrorCode,
                byte
            >.OperationFailedMessage)Decode(harness.Transport.SubmitMessages[1]);
            Assert.That(operationMessage.Failure.SessionId, Is.EqualTo(session));
            Assert.That(operationMessage.Failure.BatchId, Is.EqualTo(batch));
            Assert.That(operationMessage.Failure.CommandId, Is.EqualTo(command));
            Assert.That(
                operationMessage.Failure.ErrorCode,
                Is.EqualTo(CoreErrorCode.UnityException)
            );
            Assert.That(operationMessage.Failure.Message, Is.EqualTo("particle callback failed"));
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
            Assert.That(harness.Transport.Calls, Does.Not.Contain("stop"));

            BattlementLogRecord batchLog = harness.Logger.Records.Single(record =>
                record.EventName == "battlement.batch.failed"
            );
            Assert.That(batchLog.Fields!["session_id"], Is.EqualTo(session.ToString()));
            Assert.That(batchLog.Fields["batch_id"], Is.EqualTo(batch.ToString()));
            Assert.That(batchLog.Fields["command_id"], Is.EqualTo(command.ToString()));
            Assert.That(batchLog.Fields["error_code"], Is.EqualTo("AssetNotPrepared"));
            Assert.That(
                harness
                    .Logger.Records.Single(record =>
                        record.EventName == "battlement.operation.failed"
                    )
                    .Fields!["error_code"],
                Is.EqualTo("UnityException")
            );
        }

        [Test]
        public void EveryCoreErrorCodeUsesItsExactProtocolName()
        {
            SessionId session = new(Guid.NewGuid());
            BatchId batch = new(Guid.NewGuid());
            foreach (CoreErrorCode errorCode in Enum.GetValues(typeof(CoreErrorCode)))
            {
                byte[] bytes = BattlementMessagePack.SerializeBatchFailure(
                    new BatchFailed<CoreErrorCode>(session, batch, errorCode, "diagnostic")
                );
                var message = (ClientMessage<CoreErrorCode, byte>.BatchFailedMessage)Decode(bytes);
                Assert.That(message.Failure.ErrorCode, Is.EqualTo(errorCode));
            }
        }

        [Test]
        public void ReturnedCorrectionAppliesAfterTheCurrentResponseWithoutRecursion()
        {
            var codec = new ReentrantFailureCodec();
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                protocolCodec: codec
            );
            SessionId session = new(Guid.NewGuid());
            BatchId batch = new(Guid.NewGuid());
            codec.EnqueueResponse(Response(session, inputDisabled: false));
            harness.Transport.EnqueueConnect(
                new BattlementTransportResult(BattlementTransportStatus.Success, new byte[] { 1 })
            );
            harness.Runner.Connect();

            codec.EnqueueResponse(Response(session, inputDisabled: true));
            codec.EnqueueResponse(Response(session, inputDisabled: false));
            harness.Transport.EnqueueSubmit(
                new BattlementTransportResult(BattlementTransportStatus.Success, new byte[] { 2 })
            );
            harness.Transport.EnqueueSubmit(
                new BattlementTransportResult(BattlementTransportStatus.Success, new byte[] { 3 })
            );
            codec.BeforeSecondDecode = () =>
            {
                harness.Runner.ReportBatchFailure(
                    new BatchFailed<CoreErrorCode>(
                        session,
                        batch,
                        CoreErrorCode.UnknownObject,
                        "target missing"
                    )
                );
                Assert.That(
                    harness.Runner.IsInputAvailable,
                    Is.True,
                    "The correction must not apply inside the current response."
                );
            };

            harness.Runner.Submit(new byte[] { 9 });

            Assert.That(codec.DecodedPayloads, Is.EqualTo(new byte[] { 1, 2, 3 }));
            Assert.That(
                harness.Runner.IsInputAvailable,
                Is.True,
                "The outer disabled snapshot must apply before the queued enabled correction."
            );
        }

        [Test]
        public void FailureSubmissionTransportErrorIsSessionFatal()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(SnapshotResponse(session, inputDisabled: false));
            harness.Runner.Connect();
            harness.Transport.EnqueueSubmit(
                new BattlementTransportResult(
                    BattlementTransportStatus.EngineError,
                    diagnostic: "engine rejected report"
                )
            );

            harness.Runner.ReportOperationFailure(
                new OperationFailed<CoreErrorCode>(
                    session,
                    new BatchId(Guid.NewGuid()),
                    new CommandId(Guid.NewGuid()),
                    CoreErrorCode.HandlerFailed,
                    "handler failed"
                )
            );

            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(
                harness.Transport.Calls.TakeLast(2),
                Is.EqualTo(new[] { "submit", "stop" })
            );
            Assert.That(
                harness.Logger.Records.Select(record => record.EventName).TakeLast(2),
                Is.EqualTo(new[] { "battlement.operation.failed", "battlement.session.failed" })
            );
        }

        private static ClientMessage<CoreErrorCode, byte> Decode(byte[] bytes) =>
            BattlementMessagePack.DeserializeClientMessage(
                bytes,
                new CoreErrorFormatter(),
                new UnusedPayloadFormatter()
            );

        private static BattlementTransportResult SnapshotResponse(
            SessionId session,
            bool inputDisabled
        ) => FakeBattlementTransport.ResponseResult(Response(session, inputDisabled));

        private static Response Response(SessionId session, bool inputDisabled)
        {
            Snapshot snapshot = FakeBattlementTransport.CompleteSnapshot(
                session,
                inputDisabled: inputDisabled
            );
            return new Response(
                session,
                new ResponseMessage<Command>[]
                {
                    new ResponseMessage<Command>.SnapshotMessage(snapshot),
                }
            );
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

        private sealed class ReentrantFailureCodec : IBattlementProtocolCodec
        {
            private readonly Queue<Response> responses = new();
            private int decodeCount;

            public List<byte> DecodedPayloads { get; } = new();

            public System.Action? BeforeSecondDecode { get; set; }

            public void EnqueueResponse(Response response) => responses.Enqueue(response);

            public byte[] SerializeConnect(Connect value) => Array.Empty<byte>();

            public byte[] SerializeBatchFailure(BatchFailed<CoreErrorCode> value) =>
                new byte[] { 4 };

            public byte[] SerializeOperationFailure(OperationFailed<CoreErrorCode> value) =>
                new byte[] { 5 };

            public byte[] SerializeAction(Action value) => new byte[] { 6 };

            public Response DeserializeResponse(ReadOnlyMemory<byte> bytes)
            {
                decodeCount++;
                DecodedPayloads.Add(bytes.Span[0]);
                Response response = responses.Dequeue();
                if (decodeCount == 2)
                {
                    BeforeSecondDecode?.Invoke();
                }

                return response;
            }
        }
    }
}
