#nullable enable

using System;
using System.Linq;
using NUnit.Framework;
using Unity.Profiling;

namespace Masonry.Tests
{
    public sealed class MasonryPollingAndProfilingTests
    {
        [Test]
        public void ActiveRunnerPollsOncePerFrameAndAppliesResponsesInOrder()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(session, session, false)
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
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            harness.Runner.Connect();
            int recordsBeforePoll = harness.Logger.Records.Count;

            harness.Runner.RunFrame();

            Assert.That(harness.Logger.Records, Has.Count.EqualTo(recordsBeforePoll));
        }

        [Test]
        public void SlowFrameRecordIncludesDurationPayloadAndSession()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(session, session, false)
            );
            harness.Runner.Connect();
            harness.Clock.Advance(TimeSpan.FromMilliseconds(20));

            harness.Runner.RunFrame();

            MasonryLogRecord record = harness.Logger.Records.Single(value =>
                value.EventName == "masonry.frame.slow"
            );
            Assert.That(record.Severity, Is.EqualTo(MasonryLogSeverity.Warning));
            Assert.That(record.Fields!["duration_ms"], Is.EqualTo("20.000"));
            Assert.That(record.Fields["payload_bytes"], Is.EqualTo("0"));
            Assert.That(record.Fields["session_id"], Is.EqualTo(session.Value.ToString()));
        }

        [Test]
        public void PollFailureIncludesStableDiagnosticFieldsAndStopsTheSession()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            SessionId session = new(Guid.NewGuid());
            harness.Transport.EnqueueConnect(
                FakeMasonryTransport.SnapshotResponse(session, session, false)
            );
            harness.Transport.EnqueuePoll(
                new MasonryTransportResult(
                    MasonryTransportStatus.EngineError,
                    new byte[] { 1, 2 },
                    "rules failed"
                )
            );
            harness.Runner.Connect();

            harness.Runner.RunFrame();

            MasonryLogRecord record = harness.Logger.Records.Last();
            Assert.That(record.EventName, Is.EqualTo("masonry.session.failed"));
            Assert.That(record.Fields!["status"], Is.EqualTo("EngineError"));
            Assert.That(record.Fields["payload_bytes"], Is.EqualTo("2"));
            Assert.That(record.Fields["session_id"], Is.EqualTo(session.Value.ToString()));
            Assert.That(harness.Runner.IsInputAvailable, Is.False);
            Assert.That(harness.Transport.Calls.Last(), Is.EqualTo("stop"));
        }

        [Test]
        public void PublicProfilerRecordersObserveTheCoarseHostMarkers()
        {
            using MasonryTestHarness harness = MasonryTestHarness.Create();
            harness.Runner.Connect();

            using ProfilerRecorder frame = Recorder("Masonry.Frame");
            using ProfilerRecorder poll = Recorder("Masonry.Poll");
            using ProfilerRecorder serialization = Recorder("Masonry.Serialization");
            using ProfilerRecorder transport = Recorder("Masonry.Transport");
            using ProfilerRecorder parsing = Recorder("Masonry.Response.Parse");
            using ProfilerRecorder application = Recorder("Masonry.Response.Apply");
            using ProfilerRecorder customHandler = Recorder("Masonry.CustomHandler");

            harness.Runner.Reconnect();
            harness.Transport.EnqueuePoll(FakeMasonryTransport.SnapshotResponse());
            harness.Runner.RunFrame();

            AssertRecorder(frame, "Masonry.Frame");
            AssertRecorder(poll, "Masonry.Poll");
            AssertRecorder(serialization, "Masonry.Serialization");
            AssertRecorder(transport, "Masonry.Transport");
            AssertRecorder(parsing, "Masonry.Response.Parse");
            AssertRecorder(application, "Masonry.Response.Apply");
            AssertRecorder(customHandler, "Masonry.CustomHandler");
        }

        private static ProfilerRecorder Recorder(string marker) =>
            ProfilerRecorder.StartNew(ProfilerCategory.Scripts, marker, 8);

        private static void AssertRecorder(ProfilerRecorder recorder, string marker)
        {
            Assert.That(recorder.Valid, Is.True, $"{marker} was not registered.");
            Assert.That(recorder.IsRunning, Is.True, $"{marker} could not be recorded.");
        }

        private static MasonryTransportResult SnapshotResponse(
            SessionId session,
            bool inputDisabled
        ) => FakeMasonryTransport.SnapshotResponse(session, session, inputDisabled);
    }
}
