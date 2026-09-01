#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

namespace Battlement.UI
{
    internal readonly struct BattlementGridContribution
    {
        public BattlementGridContribution(int start, int span, float preferred)
        {
            Start = start;
            Span = span;
            Preferred = preferred;
        }

        public int Start { get; }

        public int Span { get; }

        public float Preferred { get; }
    }

    internal readonly struct BattlementGridAxis
    {
        public BattlementGridAxis(
            IReadOnlyList<float> sizes,
            IReadOnlyList<float> positions,
            float total,
            float overflow
        )
        {
            Sizes = sizes;
            Positions = positions;
            Total = total;
            Overflow = overflow;
        }

        public IReadOnlyList<float> Sizes { get; }

        public IReadOnlyList<float> Positions { get; }

        public float Total { get; }

        public float Overflow { get; }
    }

    internal static class BattlementGridTrackSizing
    {
        public static BattlementGridAxis Resolve(
            IReadOnlyList<GridTrack> explicitTracks,
            GridTrack implicitTrack,
            int requiredTracks,
            float gap,
            float? available,
            IReadOnlyList<BattlementGridContribution> contributions
        )
        {
            if (!float.IsFinite(gap) || gap < 0)
                throw new InvalidOperationException("Grid gaps must be finite and nonnegative.");
            if (
                available is float availableBound
                && (!float.IsFinite(availableBound) || availableBound < 0)
            )
                throw new InvalidOperationException(
                    "Available grid space must be finite and nonnegative."
                );
            int count = Math.Max(explicitTracks.Count, Math.Max(requiredTracks, 1));
            GridTrack[] tracks = Enumerable
                .Range(0, count)
                .Select(index =>
                    index < explicitTracks.Count ? explicitTracks[index] : implicitTrack
                )
                .ToArray();
            var sizes = new float[count];
            var weights = new float[count];
            float unit = 0;
            for (int index = 0; index < count; index++)
            {
                switch (tracks[index])
                {
                    case GridTrack.Px pixels:
                        if (!float.IsFinite(pixels.Value) || pixels.Value < 0)
                            throw new InvalidOperationException(
                                "Grid pixel tracks must be finite and nonnegative."
                            );
                        sizes[index] = pixels.Value;
                        break;
                    case GridTrack.Fraction fraction:
                        if (!float.IsFinite(fraction.Value) || fraction.Value <= 0)
                            throw new InvalidOperationException(
                                "Grid fraction tracks must be finite and positive."
                            );
                        weights[index] = fraction.Value;
                        break;
                    case GridTrack.Auto:
                        break;
                    default:
                        throw new InvalidOperationException("Unknown grid track type.");
                }
            }

            foreach (BattlementGridContribution contribution in contributions)
            {
                if (
                    contribution.Start < 0
                    || contribution.Span <= 0
                    || contribution.Start + contribution.Span > count
                    || !float.IsFinite(contribution.Preferred)
                    || contribution.Preferred < 0
                )
                    throw new InvalidOperationException("Grid contributions must be valid.");
                if (contribution.Span != 1)
                    continue;
                int index = contribution.Start;
                if (tracks[index] is GridTrack.Auto)
                    sizes[index] = Math.Max(sizes[index], contribution.Preferred);
                else if (weights[index] > 0)
                    unit = Math.Max(unit, contribution.Preferred / weights[index]);
            }

            foreach (BattlementGridContribution contribution in contributions)
            {
                if (contribution.Span == 1)
                    continue;
                int end = contribution.Start + contribution.Span;
                float current = gap * (contribution.Span - 1);
                for (int index = contribution.Start; index < end; index++)
                    current += weights[index] > 0 ? unit * weights[index] : sizes[index];
                float deficit = Math.Max(0, contribution.Preferred - current);
                int automatic = Enumerable
                    .Range(contribution.Start, contribution.Span)
                    .Count(index => tracks[index] is GridTrack.Auto);
                if (automatic > 0)
                {
                    float share = deficit / automatic;
                    for (int index = contribution.Start; index < end; index++)
                        if (tracks[index] is GridTrack.Auto)
                            sizes[index] += share;
                    continue;
                }
                float spanWeight = Enumerable
                    .Range(contribution.Start, contribution.Span)
                    .Sum(index => weights[index]);
                if (spanWeight > 0)
                    unit += deficit / spanWeight;
            }

            float totalWeight = weights.Sum();
            float gaps = gap * Math.Max(0, count - 1);
            if (available is float extent && totalWeight > 0)
            {
                float fixedSize = sizes.Sum();
                unit = Math.Max(unit, Math.Max(0, extent - fixedSize - gaps) / totalWeight);
            }
            for (int index = 0; index < count; index++)
                if (weights[index] > 0)
                    sizes[index] = unit * weights[index];

            var positions = new float[count];
            float cursor = 0;
            for (int index = 0; index < count; index++)
            {
                positions[index] = cursor;
                cursor += sizes[index] + (index + 1 < count ? gap : 0);
            }
            if (!float.IsFinite(cursor) || sizes.Any(size => !float.IsFinite(size) || size < 0))
                throw new InvalidOperationException(
                    "Grid track sizing did not produce finite output."
                );
            return new BattlementGridAxis(
                sizes,
                positions,
                cursor,
                available is float bound ? Math.Max(0, cursor - bound) : 0
            );
        }
    }
}
