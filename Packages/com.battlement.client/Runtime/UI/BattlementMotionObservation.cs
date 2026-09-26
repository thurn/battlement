#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    internal enum MotionObservationKind
    {
        Slot,
        ValuePlayback,
        GraphTime,
    }

    internal enum MotionObservationClock
    {
        Unscaled,
        Scaled,
        Controlled,
        Audio,
    }

    // Values retained from renderer sampling; observation never advances or resamples a clock.
    internal sealed record MotionTimelineObservation(
        MotionObservationKind Kind,
        Guid OwnerId,
        ulong? Slot,
        MotionObservationClock Clock,
        Guid? ClockId,
        ulong? SampledClockMicros,
        ulong ElapsedMicros,
        ulong? AnchorMicros,
        bool Held,
        bool Infinite
    )
    {
        public static MotionTimelineObservation Create(
            MotionObservationKind kind,
            Guid owner,
            ulong? slot,
            MotionClockSource clock,
            ulong? sampledClockMicros,
            ulong elapsedMicros,
            ulong? anchorMicros,
            bool held,
            bool infinite
        ) =>
            new(
                kind,
                owner,
                slot,
                clock switch
                {
                    MotionClockSource.Unscaled => MotionObservationClock.Unscaled,
                    MotionClockSource.Scaled => MotionObservationClock.Scaled,
                    MotionClockSource.Controlled => MotionObservationClock.Controlled,
                    MotionClockSource.Audio => MotionObservationClock.Audio,
                    _ => throw new InvalidOperationException("Unknown Motion clock."),
                },
                clock switch
                {
                    MotionClockSource.Controlled value => value.Value.Value,
                    MotionClockSource.Audio value => value.Value.Value,
                    _ => null,
                },
                sampledClockMicros,
                elapsedMicros,
                anchorMicros,
                held,
                infinite
            );
    }

    internal sealed record MotionPresentationObservation(
        int SampleCount,
        IReadOnlyList<MotionTimelineObservation> Samples
    )
    {
        internal const int MaximumSamples = 16;

        public static MotionPresentationObservation Capture(
            IEnumerable<MotionTimelineObservation> observations
        )
        {
            int count = 0;
            var groups =
                new Dictionary<
                    (MotionObservationKind, MotionObservationClock),
                    List<MotionTimelineObservation>
                >();
            foreach (MotionTimelineObservation observation in observations)
            {
                count++;
                var key = (observation.Kind, observation.Clock);
                if (!groups.TryGetValue(key, out List<MotionTimelineObservation> group))
                    groups.Add(key, group = new List<MotionTimelineObservation>());
                if (group.Count < MaximumSamples)
                    group.Add(observation);
            }
            // Round-robin clock/kind groups so many decorative slots cannot hide an audio clock.
            List<MotionTimelineObservation>[] ordered = groups
                .OrderBy(pair => pair.Key.Item1)
                .ThenBy(pair => pair.Key.Item2)
                .Select(pair => pair.Value)
                .ToArray();
            var samples = new List<MotionTimelineObservation>();
            for (int index = 0; index < MaximumSamples; index++)
                foreach (List<MotionTimelineObservation> group in ordered)
                    if (index < group.Count && samples.Count < MaximumSamples)
                        samples.Add(group[index]);
            return new MotionPresentationObservation(count, samples.ToArray());
        }
    }
}
