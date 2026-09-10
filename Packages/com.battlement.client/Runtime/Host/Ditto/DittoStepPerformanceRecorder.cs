#nullable enable

using System;
using System.Collections.Generic;
using System.Diagnostics;
using UnityEngine;

namespace Battlement
{
    /// Records Unity end-of-frame timing for one measured interaction.
    internal sealed class DittoStepPerformanceRecorder
    {
        public const string TimingProxy = "unity-wait-for-end-of-frame";

        private readonly uint targetFps;
        private readonly Func<long> timestamp;
        private readonly long timestampFrequency;
        private readonly Func<long>? allocatedBytes;
        private readonly List<long> endOfFrameTicks = new();
        private readonly List<long>? allocationDeltas;
        private readonly List<DittoObserverFrameTiming> observerTimings = new();
        private long previousAllocatedBytes;
        private long? inputDispatched;
        private long? activationDispatched;
        private long? response;
        private long? semanticCompletion;
        private long? settledCompletion;

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
            this.allocatedBytes = allocatedBytes;
            if (allocatedBytes is not null)
            {
                allocationDeltas = new List<long>();
                previousAllocatedBytes = allocatedBytes();
            }
        }

        public void BeginInputDispatch()
        {
            if (inputDispatched.HasValue)
                throw new InvalidOperationException("Input dispatch was already recorded.");
            inputDispatched = timestamp();
        }

        public void BeginActivationDispatch()
        {
            RequireInputDispatch();
            if (activationDispatched.HasValue)
                throw new InvalidOperationException("Activation dispatch was already recorded.");
            activationDispatched = timestamp();
        }

        public void Presented(bool changed) =>
            Presented(
                timestamp(),
                changed,
                false,
                new DittoObserverFrameTiming(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
            );

        public void Presented(
            long endOfFrameTick,
            bool changed,
            bool semanticCompleted,
            DittoObserverFrameTiming timing
        )
        {
            RequireInputDispatch();
            if (endOfFrameTick < inputDispatched)
                throw new ArgumentOutOfRangeException(nameof(endOfFrameTick));
            long bookkeepingStarted = timestamp();
            long? allocatedBeforeBookkeeping = allocatedBytes?.Invoke();
            endOfFrameTicks.Add(endOfFrameTick);
            if (allocatedBeforeBookkeeping.HasValue)
            {
                allocationDeltas!.Add(allocatedBeforeBookkeeping.Value - previousAllocatedBytes);
                previousAllocatedBytes = allocatedBytes!.Invoke();
            }
            if (changed && response is null)
                response = endOfFrameTick;
            if (semanticCompleted && semanticCompletion is null)
            {
                if (!activationDispatched.HasValue)
                    throw new InvalidOperationException(
                        "Semantic completion preceded activation dispatch."
                    );
                semanticCompletion = endOfFrameTick;
            }
            observerTimings.Add(
                timing with
                {
                    RecorderBookkeepingNs = Nanoseconds(timestamp() - bookkeepingStarted),
                }
            );
        }

        public void CompleteSettled(long endOfFrameTick)
        {
            RequireInputDispatch();
            if (settledCompletion.HasValue)
                throw new InvalidOperationException("Settled completion was already recorded.");
            settledCompletion = endOfFrameTick;
        }

        public DittoStepPerformance Finish()
        {
            RequireInputDispatch();
            if (endOfFrameTicks.Count == 0)
                Presented(
                    timestamp(),
                    false,
                    false,
                    new DittoObserverFrameTiming(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
                );
            if (
                activationDispatched.HasValue
                && !semanticCompletion.HasValue
                && !settledCompletion.HasValue
            )
                settledCompletion = endOfFrameTicks[^1];
            bool noVisualResponse = response is null;
            long responseTick = response ?? endOfFrameTicks[^1];
            ulong responseNs = Nanoseconds(responseTick - inputDispatched!.Value);
            ulong periodNs = 1_000_000_000UL / targetFps;
            ulong responseSlots = DivideRoundNearest(responseNs, periodNs);
            ulong responseMisses = responseSlots > 0 ? responseSlots - 1 : 0;
            if (noVisualResponse)
                responseMisses = Math.Max(1, responseMisses);
            var timestamps = new List<ulong>(endOfFrameTicks.Count);
            var intervals = new List<ulong>(Math.Max(0, endOfFrameTicks.Count - 1));
            for (int index = 0; index < endOfFrameTicks.Count; index++)
            {
                timestamps.Add(Nanoseconds(endOfFrameTicks[index] - inputDispatched.Value));
                if (index > 0)
                    intervals.Add(timestamps[index] - timestamps[index - 1]);
            }
            ulong pacingMisses = 0;
            ulong previousSlot = 0;
            foreach (long tick in endOfFrameTicks)
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
                TimingProxy,
                PacingConfiguration(),
                responseNs,
                responseMisses,
                pacingMisses,
                checked(responseMisses + pacingMisses),
                noVisualResponse,
                activationDispatched.HasValue,
                semanticCompletion.HasValue
                    ? Nanoseconds(semanticCompletion.Value - activationDispatched!.Value)
                    : null,
                settledCompletion.HasValue
                    ? Nanoseconds(settledCompletion.Value - activationDispatched!.Value)
                    : null,
                timestamps,
                intervals,
                allocationDeltas,
                observerTimings
            );
        }

        private DittoPacingConfiguration PacingConfiguration()
        {
            double refresh = Screen.currentResolution.refreshRateRatio.value;
            return new DittoPacingConfiguration(
                QualitySettings.vSyncCount,
                Application.targetFrameRate,
                double.IsFinite(refresh) && refresh > 0 ? refresh : null,
                checked((uint)Screen.width),
                checked((uint)Screen.height),
                UnityEngine.Debug.isDebugBuild
            );
        }

        private void RequireInputDispatch()
        {
            if (!inputDispatched.HasValue)
                throw new InvalidOperationException("Input dispatch has not been recorded.");
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
