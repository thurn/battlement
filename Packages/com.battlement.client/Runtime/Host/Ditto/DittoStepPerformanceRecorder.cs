#nullable enable

using System;
using System.Collections.Generic;
using System.Diagnostics;

namespace Battlement
{
    /// Records wall-clock presentation cadence for one measured interaction.
    internal sealed class DittoStepPerformanceRecorder
    {
        private readonly uint targetFps;
        private readonly Func<long> timestamp;
        private readonly long timestampFrequency;
        private readonly Func<long> allocatedBytes;
        private readonly long started;
        private readonly List<long> presentationTicks = new();
        private readonly List<long> allocationDeltas = new();
        private long previousAllocatedBytes;
        private long? response;

        public DittoStepPerformanceRecorder(
            uint targetFps,
            Func<long>? timestamp = null,
            long? timestampFrequency = null,
            Func<long>? allocatedBytes = null
        )
        {
            if (targetFps == 0)
                throw new ArgumentOutOfRangeException(nameof(targetFps));
            this.targetFps = targetFps;
            this.timestamp = timestamp ?? Stopwatch.GetTimestamp;
            this.timestampFrequency = timestampFrequency ?? Stopwatch.Frequency;
            if (this.timestampFrequency <= 0)
                throw new ArgumentOutOfRangeException(nameof(timestampFrequency));
            this.allocatedBytes = allocatedBytes ?? GC.GetAllocatedBytesForCurrentThread;
            started = this.timestamp();
            previousAllocatedBytes = this.allocatedBytes();
        }

        public void Presented(bool changed)
        {
            long allocatedBeforeBookkeeping = allocatedBytes();
            long presentedAt = timestamp();
            presentationTicks.Add(presentedAt);
            allocationDeltas.Add(allocatedBeforeBookkeeping - previousAllocatedBytes);
            previousAllocatedBytes = allocatedBytes();
            if (changed && response is null)
                response = presentedAt;
        }

        public DittoStepPerformance Finish()
        {
            if (presentationTicks.Count == 0)
                Presented(false);
            bool noVisualResponse = response is null;
            long responseTick = response ?? presentationTicks[^1];
            ulong responseNs = Nanoseconds(responseTick - started);
            ulong periodNs = 1_000_000_000UL / targetFps;
            ulong responseSlots = DivideRoundNearest(responseNs, periodNs);
            ulong responseMisses = responseSlots > 0 ? responseSlots - 1 : 0;
            if (noVisualResponse)
                responseMisses = Math.Max(1, responseMisses);
            var timestamps = new List<ulong>(presentationTicks.Count);
            var intervals = new List<ulong>(Math.Max(0, presentationTicks.Count - 1));
            for (int index = 0; index < presentationTicks.Count; index++)
            {
                timestamps.Add(Nanoseconds(presentationTicks[index] - started));
                if (index > 0)
                    intervals.Add(timestamps[index] - timestamps[index - 1]);
            }
            ulong pacingMisses = 0;
            ulong previousSlot = 0;
            foreach (long tick in presentationTicks)
            {
                if (tick <= responseTick)
                    continue;
                ulong slot = DivideRoundNearest(Nanoseconds(tick - responseTick), periodNs);
                if (slot <= previousSlot)
                    continue;
                if (slot > previousSlot + 1)
                    pacingMisses += slot - previousSlot - 1;
                previousSlot = slot;
            }
            return new DittoStepPerformance(
                targetFps,
                responseNs,
                responseMisses,
                pacingMisses,
                checked(responseMisses + pacingMisses),
                noVisualResponse,
                timestamps,
                intervals,
                allocationDeltas
            );
        }

        private ulong Nanoseconds(long ticks)
        {
            ulong value = checked((ulong)ticks);
            ulong frequency = checked((ulong)timestampFrequency);
            ulong wholeSeconds = value / frequency;
            ulong remainder = value % frequency;
            return checked(
                wholeSeconds * 1_000_000_000UL + remainder * 1_000_000_000UL / frequency
            );
        }

        private static ulong DivideRoundNearest(ulong value, ulong divisor) =>
            checked(value + divisor / 2) / divisor;
    }
}
