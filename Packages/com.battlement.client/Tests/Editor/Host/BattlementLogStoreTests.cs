#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class BattlementLogStoreTests
    {
        [SetUp]
        public void SetUp() => BattlementLogStore.Clear();

        [TearDown]
        public void TearDown() => BattlementLogStore.Clear();

        [Test]
        public void OrdinaryAndTypedContextRecordsShareOneImmutableUtcTranscript()
        {
            var fields = new Dictionary<string, string> { ["phase"] = "before" };
            DateTimeOffset localTime = new(2026, 8, 28, 12, 0, 0, TimeSpan.FromHours(2));
            BattlementLogStore.Add(
                "battlement",
                new BattlementLogRecord(
                    BattlementLogSeverity.Warning,
                    "managed.warning",
                    "managed",
                    fields,
                    new InvalidOperationException("fixture exception"),
                    "managed stack"
                ),
                localTime
            );
            fields["phase"] = "after";
            BattlementLogStore.Add(
                "rust",
                new BattlementLogRecord(BattlementLogSeverity.Debug, "rust.trace", "native")
            );
            BattlementLogStore.Add(
                "unity",
                new BattlementLogRecord(BattlementLogSeverity.Information, "unity.log", "unity")
            );
            var context = new FixtureContext("scenario-started", 7);
            BattlementLogStore.AddContext(
                "ditto-player",
                new BattlementLogRecord(
                    BattlementLogSeverity.Information,
                    "ditto.context",
                    "scenario started"
                ),
                context
            );

            BattlementLogEntry[] transcript = BattlementLogStore.Snapshot(out _);
            Assert.That(transcript, Has.Length.EqualTo(4));
            Assert.That(
                transcript.Select(entry => entry.Source),
                Is.EqualTo(new[] { "battlement", "rust", "unity", "ditto-player" })
            );
            AssertContiguous(transcript);
            Assert.That(transcript.All(entry => entry.OccurredAt.Offset == TimeSpan.Zero), Is.True);
            Assert.That(transcript[0].OccurredAt.Hour, Is.EqualTo(10));
            Assert.That(transcript[0].Record.Fields!["phase"], Is.EqualTo("before"));
            Assert.That(transcript[0].Record.Exception!.Message, Is.EqualTo("fixture exception"));
            Assert.That(transcript[0].Record.StackTrace, Is.EqualTo("managed stack"));
            Assert.That(
                transcript.Take(3).All(entry => entry.Payload is BattlementStoredPayload.Ordinary),
                Is.True
            );
            Assert.That(
                transcript[3].Payload,
                Is.EqualTo(new BattlementStoredPayload.Context(transcript[3].Record, context))
            );
            Assert.That(
                BattlementLogStore.RecentRecords(1).Single().EventName,
                Is.EqualTo("ditto.context")
            );
        }

        [Test]
        public void RegistrationDuringPublicationLosesAndDuplicatesNothing()
        {
            using var start = new Barrier(2);
            Task writer = Task.Run(() =>
            {
                start.SignalAndWait();
                for (int index = 0; index < 1_000; index++)
                {
                    Add($"event-{index:D4}");
                }
            });
            start.SignalAndWait();
            using BattlementLogObserver observer = BattlementLogStore.Observe();
            writer.Wait();

            BattlementLogEntry[] transcript = observer.Drain();
            Assert.That(transcript, Has.Length.EqualTo(1_000));
            AssertContiguous(transcript);
            Assert.That(
                transcript.Select(entry => entry.Record.EventName).Distinct().Count(),
                Is.EqualTo(1_000)
            );
            Assert.That(observer.Drain(), Is.Empty);
        }

        [Test]
        public void ViewerEvictionCannotCreateAnObserverDeliveryGap()
        {
            using BattlementLogObserver observer = BattlementLogStore.Observe();
            for (int index = 0; index < 1_024; index++)
            {
                Add($"first-{index:D4}");
            }
            BattlementLogEntry[] first = observer.Drain();
            for (int index = 0; index < 2_048; index++)
            {
                Add($"second-{index:D4}");
            }

            BattlementLogEntry[] second = observer.Drain();
            BattlementLogEntry[] transcript = first.Concat(second).ToArray();
            BattlementLogEntry[] retained = BattlementLogStore.Snapshot(out _);
            Assert.That(first, Has.Length.EqualTo(1_024));
            Assert.That(second, Has.Length.EqualTo(2_048));
            Assert.That(observer.Overflowed, Is.False);
            AssertContiguous(transcript);
            Assert.That(
                retained.Select(entry => entry.Sequence),
                Is.EqualTo(transcript.Skip(1_024).Select(entry => entry.Sequence))
            );
        }

        [Test]
        public void DeliveryOverflowRetainsOneContiguousPrefixAndStopsAdmission()
        {
            using BattlementLogObserver observer = BattlementLogStore.Observe();
            for (int index = 0; index < 2_049; index++)
            {
                Add($"overflow-{index:D4}");
            }

            Assert.That(observer.Overflowed, Is.True);
            Assert.That(observer.Count, Is.EqualTo(2_048));
            BattlementLogEntry[] prefix = observer.Drain();
            AssertContiguous(prefix);
            Add("after-overflow");
            Assert.That(observer.Drain(), Is.Empty);
        }

        [Test]
        public void ClearingViewerRetentionDoesNotRestartProcessSequence()
        {
            Add("before-clear");
            ulong before = BattlementLogStore.Snapshot(out _).Single().Sequence;
            BattlementLogStore.Clear();
            Add("after-clear");
            Assert.That(
                BattlementLogStore.Snapshot(out _).Single().Sequence,
                Is.EqualTo(before + 1)
            );
        }

        private static void Add(string eventName) =>
            BattlementLogStore.Add(
                "battlement",
                new BattlementLogRecord(BattlementLogSeverity.Information, eventName, eventName)
            );

        private static void AssertContiguous(IReadOnlyList<BattlementLogEntry> entries)
        {
            for (int index = 1; index < entries.Count; index++)
            {
                Assert.That(entries[index].Sequence, Is.EqualTo(entries[index - 1].Sequence + 1));
            }
        }

        private sealed record FixtureContext(string Boundary, int Index) : DittoContext;
    }
}
