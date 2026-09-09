#nullable enable

using System.Collections.Generic;
using NUnit.Framework;

namespace Battlement.Tests
{
    public sealed class DittoStepPerformanceRecorderTests
    {
        [Test]
        public void RecordsAlignedIntervalsAndExcludesRecorderBookkeepingAllocations()
        {
            var ticks = new Queue<long>(new long[] { 0, 1, 3, 6 });
            var allocations = new Queue<long>(
                new long[] { 100, 120, 1_000, 1_030, 2_000, 2_050, 3_000 }
            );
            var recorder = new DittoStepPerformanceRecorder(
                60,
                ticks.Dequeue,
                60,
                allocations.Dequeue
            );

            recorder.Presented(false);
            recorder.Presented(false);
            recorder.Presented(false);
            DittoStepPerformance result = recorder.Finish();

            Assert.That(
                result.PresentationTimestampsNs,
                Is.EqualTo(new ulong[] { 16_666_666, 50_000_000, 100_000_000 })
            );
            Assert.That(
                result.PresentationIntervalsNs,
                Is.EqualTo(new ulong[] { 33_333_334, 50_000_000 })
            );
            Assert.That(result.ManagedAllocationDeltas, Is.EqualTo(new long[] { 20, 30, 50 }));
            Assert.That(result.NoVisualResponse, Is.True);
            Assert.That(result.ResponseMissedDeadlines, Is.GreaterThanOrEqualTo(1));
            Assert.That(
                result.MissedInteractionDeadlines,
                Is.EqualTo(result.ResponseMissedDeadlines + result.PacingMissedDeadlines)
            );
        }

        [Test]
        public void EarlyPresentationBurstsDoNotFillFuturePacingSlots()
        {
            var ticks = new Queue<long>(new long[] { 0, 1, 2, 3, 4, 5, 8 });
            var recorder = new DittoStepPerformanceRecorder(60, ticks.Dequeue, 60, () => 0);

            recorder.Presented(true);
            for (var index = 0; index < 5; index++)
                recorder.Presented(false);
            DittoStepPerformance result = recorder.Finish();

            Assert.That(result.NoVisualResponse, Is.False);
            Assert.That(result.ResponseMissedDeadlines, Is.Zero);
            Assert.That(result.PacingMissedDeadlines, Is.EqualTo(2));
        }
    }
}
