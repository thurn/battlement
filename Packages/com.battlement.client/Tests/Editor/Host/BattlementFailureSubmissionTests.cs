#nullable enable

using System;
using System.Linq;
using System.Text;
using Battlement.CustomFixtures;
using Google.FlatBuffers;
using NUnit.Framework;
using Wire = Battlement.FlatBuffers.Generated;

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

            Wire.BatchFailed batchMessage = Decode(harness.Transport.SubmitMessages[0])
                .BodyAsBatchFailed();
            Assert.That(ReadId(batchMessage.SessionId, "session_id"), Is.EqualTo(session.Value));
            Assert.That(ReadId(batchMessage.BatchId, "batch_id"), Is.EqualTo(batch.Value));
            Assert.That(ReadId(batchMessage.CommandId, "command_id"), Is.EqualTo(command.Value));
            Assert.That(batchMessage.ErrorCode, Is.EqualTo(Wire.CoreErrorCode.AssetNotPrepared));
            Assert.That(Encoding.UTF8.GetByteCount(batchMessage.Message), Is.EqualTo(65_536));

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

            Wire.OperationFailed operationMessage = Decode(harness.Transport.SubmitMessages[1])
                .BodyAsOperationFailed();
            Assert.That(
                ReadId(operationMessage.SessionId, "session_id"),
                Is.EqualTo(session.Value)
            );
            Assert.That(ReadId(operationMessage.BatchId, "batch_id"), Is.EqualTo(batch.Value));
            Assert.That(
                ReadId(operationMessage.CommandId, "command_id"),
                Is.EqualTo(command.Value)
            );
            Assert.That(operationMessage.ErrorCode, Is.EqualTo(Wire.CoreErrorCode.UnityException));
            Assert.That(operationMessage.Message, Is.EqualTo("particle callback failed"));
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
        public void EveryCoreErrorCodeUsesItsExactWireOrdinal()
        {
            SessionId session = new(Guid.NewGuid());
            BatchId batch = new(Guid.NewGuid());
            var writer = new BattlementCoreClientMessageWriter();
            foreach (CoreErrorCode errorCode in Enum.GetValues(typeof(CoreErrorCode)))
            {
                byte[] bytes = writer
                    .WriteBatchFailure(
                        new BatchFailed<CoreErrorCode>(session, batch, errorCode, "diagnostic")
                    )
                    .ToArray();
                Wire.BatchFailed message = Decode(bytes).BodyAsBatchFailed();
                Assert.That((int)message.ErrorCode, Is.EqualTo((int)errorCode));
            }
        }

        [Test]
        public void ReturnedCorrectionAppliesAfterTheCurrentResponseWithoutRecursion()
        {
            SessionId session = new(Guid.NewGuid());
            BatchId batch = new(Guid.NewGuid());
            var responseSchema = new FixtureFlatBufferResponseSchema(_ => 0, _ => default);
            using BattlementTestHarness harness = BattlementTestHarness.Create(
                flatBufferResponseSchema: responseSchema
            );
            harness.Transport.EnqueueConnect(SnapshotResponse(session, inputDisabled: false));
            harness.Runner.Connect();
            responseSchema.InvokeBeforeNextRead(() =>
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
            });

            harness.Transport.EnqueueSubmit(
                new BattlementTransportResult(
                    BattlementTransportStatus.Success,
                    BattlementFlatBufferResponseFixtures.Write(
                        Response(session, inputDisabled: true)
                    )
                )
            );
            harness.Transport.EnqueueSubmit(SnapshotResponse(session, inputDisabled: false));

            harness.Runner.Submit(new byte[] { 9 });

            Assert.That(
                harness.Transport.Calls.TakeLast(2),
                Is.EqualTo(new[] { "submit", "submit" })
            );
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

        private static Wire.CoreClientMessage Decode(byte[] bytes)
        {
            var buffer = new ByteBuffer(bytes);
            var verifier = new Verifier(buffer, new Options(64, 1_000_000, true, true));
            Assert.That(
                verifier.VerifyBuffer("BTCM", true, Wire.CoreClientMessageVerify.Verify),
                Is.True
            );
            buffer.Position = 4;
            return Wire.CoreClientMessage.GetRootAsCoreClientMessage(buffer);
        }

        private static Guid ReadId(Wire.Uuid? value, string field) =>
            BattlementFlatBufferCore.ReadUuid(value, field);

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
    }
}
