#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
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
        public void MalformedBatchesStopAtTheVerifiedBoundary()
        {
            SessionId session = new(Guid.NewGuid());
            Command duplicate = ValidCommand();
            var cases = new Batch[]
            {
                new Batch(
                    new BatchId(Guid.NewGuid()),
                    session,
                    Array.Empty<ParallelCommandGroup<Command>>()
                ),
                new Batch(
                    new BatchId(Guid.NewGuid()),
                    session,
                    Enumerable.Range(0, 257).Select(_ => Group()).ToArray()
                ),
                new Batch(
                    new BatchId(Guid.NewGuid()),
                    session,
                    new[] { new ParallelCommandGroup<Command>(Array.Empty<Command>()) }
                ),
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
                new Batch(
                    new BatchId(Guid.NewGuid()),
                    session,
                    new[] { new ParallelCommandGroup<Command>(new[] { duplicate, duplicate }) }
                ),
            };

            using (BattlementTestHarness wrongSessionHarness = BattlementTestHarness.Create())
            {
                Connect(wrongSessionHarness, session);
                Batch wrongSession = ValidBatch(new SessionId(Guid.NewGuid()), BatchStart.Now);
                SubmitResponse(wrongSessionHarness, BatchResponse(session, wrongSession));
                Assert.That(wrongSessionHarness.Transport.BatchFailures, Is.Empty);
                Assert.That(wrongSessionHarness.Transport.Calls.Last(), Is.EqualTo("stop"));
            }

            foreach (Batch batch in cases)
            {
                using BattlementTestHarness harness = BattlementTestHarness.Create();
                Connect(harness, session);
                harness.Transport.EnqueueSubmit(
                    FakeBattlementTransport.ResponseResult(BatchResponse(session, batch))
                );
                harness.Runner.Submit(new byte[] { 1 });

                Assert.That(harness.Transport.BatchFailures, Is.Empty);
                Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
            }
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
            malformedHarness.Transport.EnqueueSubmit(
                new BattlementTransportResult(
                    BattlementTransportStatus.Success,
                    new byte[] { 0, 1, 2, 3 }
                )
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
    }
}
