#nullable enable

using System;
using Battlement.UI;
using Newtonsoft.Json;

namespace Battlement
{
    // Ticks are exact 100 ns units; timeline clocks and phases use microseconds.
    internal sealed record DittoMotionEvidence(
        DittoMotion Mode,
        ulong ElapsedTicks,
        ulong ScenarioElapsedTicks,
        int FiniteTimelineCount,
        int InfiniteTimelineCount,
        int HeldTimelineCount,
        bool HasPendingWork,
        bool HasDeferredUiWork,
        MotionPresentationObservation Timelines
    )
    {
        public void Validate()
        {
            Require(Enum.IsDefined(typeof(DittoMotion), Mode), "Unknown Motion evidence mode.");
            Require(ScenarioElapsedTicks <= ElapsedTicks, "Invalid scenario Motion elapsed time.");
            Require(FiniteTimelineCount >= 0, "Negative finite timeline count.");
            Require(InfiniteTimelineCount >= 0, "Negative infinite timeline count.");
            Require(HeldTimelineCount >= 0, "Negative held timeline count.");
            if (Timelines is null)
                throw new JsonSerializationException("Missing sampled timeline evidence.");
            Require(Timelines.SampleCount >= 0, "Negative sampled timeline count.");
            Require(
                Timelines.Samples.Count
                    == Math.Min(
                        Timelines.SampleCount,
                        MotionPresentationObservation.MaximumSamples
                    ),
                "Sampled timeline evidence has an invalid bound."
            );
            foreach (MotionTimelineObservation sample in Timelines.Samples)
            {
                Require(
                    Enum.IsDefined(typeof(MotionObservationKind), sample.Kind),
                    "Unknown sample."
                );
                Require(
                    Enum.IsDefined(typeof(MotionObservationClock), sample.Clock),
                    "Unknown clock."
                );
                Require(sample.OwnerId != Guid.Empty, "Missing Motion sample owner.");
                bool identified =
                    sample.Clock
                    is MotionObservationClock.Audio
                        or MotionObservationClock.Controlled;
                Require(identified == sample.ClockId.HasValue, "Motion clock identity mismatch.");
                Require(sample.ClockId != Guid.Empty, "Empty Motion clock identity.");
                Require(
                    (sample.Kind == MotionObservationKind.Slot) == sample.Slot.HasValue,
                    "Motion slot identity mismatch."
                );
            }
        }

        private static void Require(bool condition, string message)
        {
            if (!condition)
                throw new JsonSerializationException(message);
        }
    }
}
