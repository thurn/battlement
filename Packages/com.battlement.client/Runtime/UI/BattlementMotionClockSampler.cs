#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement.UI
{
    internal readonly struct MotionClockSample
    {
        public MotionClockSample(ulong elapsedMicros, bool discontinuity) =>
            (ElapsedMicros, Discontinuity) = (elapsedMicros, discontinuity);

        public ulong ElapsedMicros { get; }

        public bool Discontinuity { get; }
    }

    internal static class BattlementMotionClockSampler
    {
        public static MotionClockSample Sample(
            MotionClockSource source,
            Func<double> unscaledTime,
            Func<double> scaledTime,
            IReadOnlyDictionary<Guid, ulong> controlledClocks,
            Func<ObjectId, MotionClockSample>? audioTime
        ) =>
            source switch
            {
                MotionClockSource.Unscaled => new MotionClockSample(
                    BattlementMotionClock.ToMicros(unscaledTime()),
                    false
                ),
                MotionClockSource.Scaled => new MotionClockSample(
                    BattlementMotionClock.ToMicros(scaledTime()),
                    false
                ),
                MotionClockSource.Controlled value => controlledClocks.TryGetValue(
                    value.Value.Value,
                    out ulong elapsed
                )
                    ? new MotionClockSample(elapsed, false)
                    : new MotionClockSample(0, false),
                MotionClockSource.Audio value => audioTime?.Invoke(value.Value)
                    ?? new MotionClockSample(0, false),
                _ => throw new BattlementUiException(
                    CoreErrorCode.InvalidProperty,
                    "Unknown motion clock source."
                ),
            };
    }
}
