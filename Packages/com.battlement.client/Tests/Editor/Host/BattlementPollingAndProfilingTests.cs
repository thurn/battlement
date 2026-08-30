#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using Unity.Profiling;

namespace Battlement.Tests
{
    public sealed class BattlementPollingAndProfilingTests
    {
        [Test]
        public void ActiveRunnerPollsOncePerFrameAndAppliesResponsesInOrder()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session, session, false)
            );
            harness.Transport.EnqueuePoll(SnapshotResponse(session, true));
            harness.Transport.EnqueuePoll(SnapshotResponse(session, false));

            harness.Runner.Connect();
            harness.Runner.RunFrame();
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            harness.Runner.RunFrame();

            Assert.That(harness.Transport.Calls, Is.EqualTo(new[] { "connect", "poll", "poll" }));
            Assert.That(harness.Runner.IsInputAvailable, Is.True);
        }

        [Test]
        public void EmptyPollsDoNotEmitRoutineSuccessLogs()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Runner.Connect();
            int recordsBeforePoll = harness.Logger.Records.Count;

            harness.Runner.RunFrame();

            Assert.That(harness.Logger.Records, Has.Count.EqualTo(recordsBeforePoll));
        }

        [Test]
        public void SlowFrameRecordIncludesDurationPayloadAndSession()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session, session, false)
            );
            harness.Runner.Connect();
            harness.Clock.Advance(TimeSpan.FromMilliseconds(20));

            harness.Runner.RunFrame();

            BattlementLogRecord record = harness.Logger.Records.Single(value =>
                value.EventName == "battlement.frame.slow"
            );
            Assert.That(record.Severity, Is.EqualTo(BattlementLogSeverity.Warning));
            Assert.That(record.Fields!["duration_ms"], Is.EqualTo("20.000"));
            Assert.That(record.Fields["payload_bytes"], Is.EqualTo("0"));
            Assert.That(record.Fields["session_id"], Is.EqualTo(session.Value.ToString()));
        }

        [Test]
        public void ControlledVirtualFramesDoNotEmitSlowFrameRecords()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Runner.Connect();
            harness.Runner.BeginDittoMotion(DittoMotion.Controlled);

            for (var frame = 0; frame < 256; frame++)
            {
                harness.Runner.PrepareDittoFrame();
                harness.Runner.RunFrame();
            }

            Assert.That(
                harness.Logger.Records,
                Has.None.Matches<BattlementLogRecord>(value =>
                    value.EventName == "battlement.frame.slow"
                )
            );
        }

        [Test]
        public void PollFailureIncludesStableDiagnosticFieldsAndStopsTheSession()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeBattlementTransport.SnapshotResponse(session, session, false)
            );
            harness.Transport.EnqueuePoll(
                new BattlementTransportResult(
                    BattlementTransportStatus.EngineError,
                    new byte[] { 1, 2 },
                    "rules failed"
                )
            );
            harness.Runner.Connect();

            harness.Runner.RunFrame();

            BattlementLogRecord record = harness.Logger.Records.Last();
            Assert.That(record.EventName, Is.EqualTo("battlement.session.failed"));
            Assert.That(record.Fields!["status"], Is.EqualTo("EngineError"));
            Assert.That(record.Fields["payload_bytes"], Is.EqualTo("2"));
            Assert.That(record.Fields["session_id"], Is.EqualTo(session.Value.ToString()));
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
        }

        [Test]
        public void PublicProfilerRecordersObserveTheCoarseHostMarkers()
        {
            using BattlementTestHarness harness = BattlementTestHarness.Create();
            harness.Runner.Connect();

            using ProfilerRecorder frame = Recorder("Battlement.Frame");
            using ProfilerRecorder poll = Recorder("Battlement.Poll");
            using ProfilerRecorder serialization = Recorder("Battlement.Serialization");
            using ProfilerRecorder transport = Recorder("Battlement.Transport");
            using ProfilerRecorder parsing = Recorder("Battlement.Response.Parse");
            using ProfilerRecorder application = Recorder("Battlement.Response.Apply");
            using ProfilerRecorder customHandler = Recorder("Battlement.CustomHandler");

            harness.Runner.Reconnect();
            harness.Transport.EnqueuePoll(FakeBattlementTransport.SnapshotResponse());
            harness.Runner.RunFrame();

            AssertRecorder(frame, "Battlement.Frame");
            AssertRecorder(poll, "Battlement.Poll");
            AssertRecorder(serialization, "Battlement.Serialization");
            AssertRecorder(transport, "Battlement.Transport");
            AssertRecorder(parsing, "Battlement.Response.Parse");
            AssertRecorder(application, "Battlement.Response.Apply");
            AssertRecorder(customHandler, "Battlement.CustomHandler");
        }

        private static ProfilerRecorder Recorder(string marker) =>
            ProfilerRecorder.StartNew(ProfilerCategory.Scripts, marker, 8);

        private static void AssertRecorder(ProfilerRecorder recorder, string marker)
        {
            Assert.That(recorder.Valid, Is.True, $"{marker} was not registered.");
            Assert.That(recorder.IsRunning, Is.True, $"{marker} could not be recorded.");
        }

        private static BattlementTransportResult SnapshotResponse(
            SessionId session,
            bool inputDisabled
        ) => FakeBattlementTransport.SnapshotResponse(session, session, inputDisabled);
    }
}
