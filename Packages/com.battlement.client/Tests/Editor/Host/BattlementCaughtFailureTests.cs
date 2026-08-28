#nullable enable

using System;
using System.Collections.Generic;
using Battlement.Errors;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementCaughtFailureTests
    {
        [Test]
        public void EligibleFailureIsReportedAfterLocalPersistence()
        {
            var order = new List<string>();
            var reporter = new BattlementErrorReporter(
                new RecordingLogger(order),
                new RecordingSink(order),
                caughtFailureReporter: new RecordingCaughtReporter(order)
            );

            reporter.Report(
                BattlementErrorType.SessionFailed,
                BattlementErrorSource.Native,
                "battlement.session.failed",
                "panic",
                stackTrace: "rust::known_frame\ncaller",
                reportingDisposition: BattlementErrorReportingDisposition.ReportCaughtFailure
            );

            Assert.That(order, Is.EqualTo(new[] { "log", "sink", "caught" }));
        }

        [Test]
        public void EnvelopePreservesOriginalStackAndOmitsUniqueErrorId()
        {
            BattlementError error = Error("unique-error-id", "rust::known_frame\ncaller");
            var envelope = new BattlementCaughtFailureException(error);

            Assert.That(envelope.StackTrace, Is.EqualTo("rust::known_frame\ncaller"));
            StringAssert.DoesNotContain(error.Id, envelope.Message);
            StringAssert.Contains("battlement.session.failed", envelope.Message);
        }

        [Test]
        public void ReporterFailureDoesNotReplaceOriginalError()
        {
            var records = new List<BattlementLogRecord>();
            var reporter = new BattlementErrorReporter(
                new DelegateLogger(records.Add),
                new RecordingSink(new List<string>()),
                caughtFailureReporter: new ThrowingCaughtReporter()
            );

            BattlementError error = reporter.Report(
                BattlementErrorType.SessionFailed,
                BattlementErrorSource.Unity,
                "battlement.session.failed",
                "original",
                new InvalidOperationException("caught"),
                reportingDisposition: BattlementErrorReportingDisposition.ReportCaughtFailure
            );

            Assert.That(error.Message, Is.EqualTo("original"));
            Assert.That(
                records[^1].EventName,
                Is.EqualTo("battlement.error.exception_report_failed")
            );
        }

        private static BattlementError Error(string id, string stackTrace) =>
            new(
                id,
                DateTimeOffset.UtcNow,
                BattlementErrorType.SessionFailed,
                BattlementErrorSource.Native,
                "battlement.session.failed",
                "panic",
                null,
                stackTrace,
                new Dictionary<string, string>(),
                Array.Empty<BattlementLogRecord>()
            );

        private sealed class RecordingLogger : IBattlementLogger
        {
            private readonly List<string> order;

            public RecordingLogger(List<string> order) => this.order = order;

            public void Log(BattlementLogRecord record) => order.Add("log");
        }

        private sealed class DelegateLogger : IBattlementLogger
        {
            private readonly Action<BattlementLogRecord> log;

            public DelegateLogger(Action<BattlementLogRecord> log) => this.log = log;

            public void Log(BattlementLogRecord record) => log(record);
        }

        private sealed class RecordingSink : IBattlementErrorSink
        {
            private readonly List<string> order;

            public RecordingSink(List<string> order) => this.order = order;

            public void Report(BattlementError error) => order.Add("sink");
        }

        private sealed class RecordingCaughtReporter : IBattlementCaughtFailureReporter
        {
            private readonly List<string> order;

            public RecordingCaughtReporter(List<string> order) => this.order = order;

            public void Report(BattlementError error) => order.Add("caught");
        }

        private sealed class ThrowingCaughtReporter : IBattlementCaughtFailureReporter
        {
            public void Report(BattlementError error) => throw new InvalidOperationException();
        }
    }
}
