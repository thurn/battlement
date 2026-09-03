#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement.UI
{
    internal readonly struct MotionTrackSample
    {
        public MotionTrackSample(
            MotionValue value,
            double velocity,
            bool done,
            uint iteration,
            ulong localMicros
        )
        {
            Value = value;
            Velocity = velocity;
            Done = done;
            Iteration = iteration;
            LocalMicros = localMicros;
        }

        public MotionValue Value { get; }
        public double Velocity { get; }
        public bool Done { get; }
        public uint Iteration { get; }
        public ulong LocalMicros { get; }
    }

    internal static class BattlementMotionValueSampler
    {
        public static MotionTrackSample Sample(
            MotionPropertyTrack track,
            MotionValue origin,
            double incomingVelocity,
            ulong elapsedMicros
        )
        {
            IReadOnlyList<MotionValue> values = Values(track, origin);
            if (track.Transition.Generator is not TransitionGenerator.Tween || values.Count == 2)
                return SampleEndpoints(track, values, incomingVelocity, elapsedMicros);

            TransitionDefinition linear = Linear(track.Transition);
            MotionScalarSample timeline = BattlementMotionScalarSampler.Sample(
                0,
                1,
                0,
                linear,
                elapsedMicros
            );
            IReadOnlyList<double> times = Times(track, values.Count);
            int segment = Segment(times, timeline.Value);
            double start = times[segment];
            double end = times[segment + 1];
            double local = end == start ? 1 : (timeline.Value - start) / (end - start);
            var tween = (TransitionGenerator.Tween)track.Transition.Generator;
            MotionEasing easing =
                segment < tween.Easings.Count ? tween.Easings[segment] : new MotionEasing.Linear();
            double progress = BattlementMotionScalarSampler.Ease(easing, local);
            return new MotionTrackSample(
                Mix(values[segment], values[segment + 1], progress),
                0,
                timeline.Done,
                timeline.Iteration,
                timeline.LocalMicros
            );
        }

        public static MotionValue Mix(MotionValue origin, MotionValue target, double progress)
        {
            progress = Math.Clamp(progress, 0, 1);
            return (origin, target) switch
            {
                (MotionValue.Scalar left, MotionValue.Scalar right) => new MotionValue.Scalar(
                    Lerp(left.Value, right.Value, progress)
                ),
                (MotionValue.Length left, MotionValue.Length right) => new MotionValue.Length(
                    Mix(left.Value, right.Value, progress)
                ),
                (MotionValue.Color left, MotionValue.Color right) => new MotionValue.Color(
                    Mix(left.Value, right.Value, progress)
                ),
                (MotionValue.Vector2 left, MotionValue.Vector2 right)
                    when left.Value.Count == 2 && right.Value.Count == 2 => new MotionValue.Vector2(
                    MixNumbers(left.Value, right.Value, progress)
                ),
                (MotionValue.Vector3 left, MotionValue.Vector3 right)
                    when left.Value.Count == 3 && right.Value.Count == 3 => new MotionValue.Vector3(
                    MixNumbers(left.Value, right.Value, progress)
                ),
                (MotionValue.Angle left, MotionValue.Angle right) => new MotionValue.Angle(
                    Lerp(left.Value, right.Value, progress)
                ),
                (MotionValue.TransformList left, MotionValue.TransformList right) => MixTransforms(
                    left,
                    right,
                    progress
                ),
                (MotionValue.FilterList left, MotionValue.FilterList right) => MixFilters(
                    left,
                    right,
                    progress
                ),
                (MotionValue.ShadowList left, MotionValue.ShadowList right) => MixShadows(
                    left,
                    right,
                    progress
                ),
                (MotionValue.Gradient left, MotionValue.Gradient right) => MixGradients(
                    left,
                    right,
                    progress
                ),
                (MotionValue.ClipInset left, MotionValue.ClipInset right)
                    when left.Value.Count == right.Value.Count => new MotionValue.ClipInset(
                    MixLengths(left.Value, right.Value, progress)
                ),
                (MotionValue.ClipPolygon left, MotionValue.ClipPolygon right)
                    when CompatiblePolygon(left.Value, right.Value) => new MotionValue.ClipPolygon(
                    MixPolygon(left.Value, right.Value, progress)
                ),
                _ => progress < 0.5 ? origin : target,
            };
        }

        private static MotionTrackSample SampleEndpoints(
            MotionPropertyTrack track,
            IReadOnlyList<MotionValue> values,
            double incomingVelocity,
            ulong elapsedMicros
        )
        {
            MotionValue origin = values[0];
            MotionValue target = values[^1];
            if (origin is MotionValue.Scalar left && target is MotionValue.Scalar right)
            {
                MotionScalarSample scalar = BattlementMotionScalarSampler.Sample(
                    left.Value,
                    right.Value,
                    incomingVelocity,
                    track.Transition,
                    elapsedMicros
                );
                return new MotionTrackSample(
                    new MotionValue.Scalar(scalar.Value),
                    scalar.Velocity,
                    scalar.Done,
                    scalar.Iteration,
                    scalar.LocalMicros
                );
            }

            MotionScalarSample progress = BattlementMotionScalarSampler.Sample(
                0,
                1,
                0,
                track.Transition,
                elapsedMicros
            );
            return new MotionTrackSample(
                Mix(origin, target, progress.Value),
                0,
                progress.Done,
                progress.Iteration,
                progress.LocalMicros
            );
        }

        private static IReadOnlyList<MotionValue> Values(
            MotionPropertyTrack track,
            MotionValue origin
        ) => track.Values.Count == 1 ? new[] { origin, track.Values[0] } : track.Values;

        private static TransitionDefinition Linear(TransitionDefinition transition)
        {
            var tween = (TransitionGenerator.Tween)transition.Generator;
            return transition with
            {
                Generator = new TransitionGenerator.Tween(
                    tween.DurationMicros,
                    new MotionEasing[] { new MotionEasing.Linear() },
                    null
                ),
            };
        }

        private static IReadOnlyList<double> Times(MotionPropertyTrack track, int count)
        {
            if (track.Times?.Count == count)
                return track.Times;
            var tween = (TransitionGenerator.Tween)track.Transition.Generator;
            if (tween.Times?.Count == count)
                return tween.Times;
            var result = new double[count];
            for (int index = 0; index < count; index++)
                result[index] = index / (double)(count - 1);
            return result;
        }

        private static int Segment(IReadOnlyList<double> times, double progress)
        {
            for (int index = 1; index < times.Count; index++)
                if (progress <= times[index])
                    return index - 1;
            return times.Count - 2;
        }

        private static UiLength Mix(UiLength left, UiLength right, double progress) =>
            UiLength.FromComponents(
                Lerp(left.Pixels, right.Pixels, progress),
                Lerp(left.Percentage, right.Percentage, progress)
            );

        private static Color Mix(Color left, Color right, double progress) =>
            new(
                RootMix(left.Red, right.Red, progress),
                RootMix(left.Green, right.Green, progress),
                RootMix(left.Blue, right.Blue, progress),
                Lerp(left.Alpha, right.Alpha, progress)
            );

        private static MotionValue MixTransforms(
            MotionValue.TransformList left,
            MotionValue.TransformList right,
            double progress
        )
        {
            if (left.Value.Count != right.Value.Count)
                return progress < 0.5 ? left : right;
            var result = new TransformOperation[left.Value.Count];
            for (int index = 0; index < result.Length; index++)
            {
                result[index] = (left.Value[index], right.Value[index]) switch
                {
                    (TransformOperation.Translate a, TransformOperation.Translate b)
                        when a.Value.Count == b.Value.Count => new TransformOperation.Translate(
                        MixLengths(a.Value, b.Value, progress)
                    ),
                    (TransformOperation.Rotate a, TransformOperation.Rotate b)
                        when a.Value.Count == b.Value.Count => new TransformOperation.Rotate(
                        MixNumbers(a.Value, b.Value, progress)
                    ),
                    (TransformOperation.Skew a, TransformOperation.Skew b)
                        when a.Value.Count == b.Value.Count => new TransformOperation.Skew(
                        MixNumbers(a.Value, b.Value, progress)
                    ),
                    (TransformOperation.Scale a, TransformOperation.Scale b)
                        when a.Value.Count == b.Value.Count => new TransformOperation.Scale(
                        MixNumbers(a.Value, b.Value, progress)
                    ),
                    _ => null!,
                };
                if (result[index] is null)
                    return progress < 0.5 ? left : right;
            }
            return new MotionValue.TransformList(result);
        }

        private static MotionValue MixFilters(
            MotionValue.FilterList left,
            MotionValue.FilterList right,
            double progress
        )
        {
            if (right.Value.Count == 0)
                return progress < 0.5 ? left : right;
            var result = new UiFilterFunction[right.Value.Count];
            int source = 0;
            for (int index = 0; index < result.Length; index++)
            {
                UiFilterFunction? origin = source < left.Value.Count ? left.Value[source++] : null;
                UiFilterFunction target = right.Value[index];
                result[index] = MixFilter(origin, target, progress);
                if (result[index] is null)
                    return progress < 0.5 ? left : right;
            }
            return new MotionValue.FilterList(result);
        }

        private static UiFilterFunction MixFilter(
            UiFilterFunction? origin,
            UiFilterFunction target,
            double progress
        )
        {
            double from = origin switch
            {
                UiFilterFunction.Blur filter => filter.Value,
                UiFilterFunction.Brightness filter => filter.Value,
                UiFilterFunction.Saturate filter => filter.Value,
                UiFilterFunction.Contrast filter => filter.Value,
                UiFilterFunction.HueRotate filter => filter.Value,
                UiFilterFunction.Opacity filter => filter.Value,
                UiFilterFunction.Invert filter => filter.Value,
                UiFilterFunction.Grayscale filter => filter.Value,
                UiFilterFunction.Sepia filter => filter.Value,
                null => 0,
                _ => double.NaN,
            };
            if (target is UiFilterFunction.Tint tint)
                return origin is UiFilterFunction.Tint source
                    ? new UiFilterFunction.Tint(Mix(source.Value, tint.Value, progress))
                    : null!;
            if (target is UiFilterFunction.DropShadow shadow)
                return origin is UiFilterFunction.DropShadow source
                    ? new UiFilterFunction.DropShadow(Mix(source.Value, shadow.Value, progress))
                    : null!;
            if (!double.IsFinite(from))
                return null!;
            double to = target switch
            {
                UiFilterFunction.Blur filter => filter.Value,
                UiFilterFunction.Brightness filter => filter.Value,
                UiFilterFunction.Saturate filter => filter.Value,
                UiFilterFunction.Contrast filter => filter.Value,
                UiFilterFunction.HueRotate filter => filter.Value,
                UiFilterFunction.Opacity filter => filter.Value,
                UiFilterFunction.Invert filter => filter.Value,
                UiFilterFunction.Grayscale filter => filter.Value,
                UiFilterFunction.Sepia filter => filter.Value,
                _ => double.NaN,
            };
            float mixed = checked((float)Lerp(from, to, progress));
            return target switch
            {
                UiFilterFunction.Blur => new UiFilterFunction.Blur(mixed),
                UiFilterFunction.Brightness => new UiFilterFunction.Brightness(mixed),
                UiFilterFunction.Saturate => new UiFilterFunction.Saturate(mixed),
                UiFilterFunction.Contrast => new UiFilterFunction.Contrast(mixed),
                UiFilterFunction.HueRotate => new UiFilterFunction.HueRotate(mixed),
                UiFilterFunction.Opacity => new UiFilterFunction.Opacity(mixed),
                UiFilterFunction.Invert => new UiFilterFunction.Invert(mixed),
                UiFilterFunction.Grayscale => new UiFilterFunction.Grayscale(mixed),
                UiFilterFunction.Sepia => new UiFilterFunction.Sepia(mixed),
                _ => null!,
            };
        }

        private static MotionValue MixShadows(
            MotionValue.ShadowList left,
            MotionValue.ShadowList right,
            double progress
        )
        {
            if (left.Value.Count != right.Value.Count)
                return progress < 0.5 ? left : right;
            var result = new Shadow[left.Value.Count];
            for (int index = 0; index < result.Length; index++)
            {
                if (left.Value[index].Inset != right.Value[index].Inset)
                    return progress < 0.5 ? left : right;
                result[index] = Mix(left.Value[index], right.Value[index], progress);
            }
            return new MotionValue.ShadowList(result);
        }

        private static Shadow Mix(Shadow left, Shadow right, double progress) =>
            new(
                Lerp(left.X, right.X, progress),
                Lerp(left.Y, right.Y, progress),
                Lerp(left.Blur, right.Blur, progress),
                Lerp(left.Spread, right.Spread, progress),
                Mix(left.Color, right.Color, progress),
                right.Inset
            );

        private static MotionValue MixGradients(
            MotionValue.Gradient left,
            MotionValue.Gradient right,
            double progress
        )
        {
            if (left.Value is Gradient.Linear a && right.Value is Gradient.Linear b)
                return a.Stops.Count == b.Stops.Count
                        ? new MotionValue.Gradient(
                            new Gradient.Linear(
                                Lerp(a.Angle, b.Angle, progress),
                                MixStops(a.Stops, b.Stops, progress)
                            )
                        )
                    : progress < 0.5 ? left
                    : right;
            if (left.Value is Gradient.Radial c && right.Value is Gradient.Radial d)
                return c.Stops.Count == d.Stops.Count
                    && c.Center.Count == d.Center.Count
                    && c.Radius.Count == d.Radius.Count
                        ? new MotionValue.Gradient(
                            new Gradient.Radial(
                                MixNumbers(c.Center, d.Center, progress),
                                MixNumbers(c.Radius, d.Radius, progress),
                                MixStops(c.Stops, d.Stops, progress)
                            )
                        )
                    : progress < 0.5 ? left
                    : right;
            return progress < 0.5 ? left : right;
        }

        private static IReadOnlyList<GradientStop> MixStops(
            IReadOnlyList<GradientStop> left,
            IReadOnlyList<GradientStop> right,
            double progress
        )
        {
            var result = new GradientStop[left.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = new GradientStop(
                    Mix(left[index].Color, right[index].Color, progress),
                    Lerp(left[index].Position, right[index].Position, progress)
                );
            return result;
        }

        private static IReadOnlyList<double> MixNumbers(
            IReadOnlyList<double> left,
            IReadOnlyList<double> right,
            double progress
        )
        {
            var result = new double[left.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = Lerp(left[index], right[index], progress);
            return result;
        }

        private static IReadOnlyList<UiLength> MixLengths(
            IReadOnlyList<UiLength> left,
            IReadOnlyList<UiLength> right,
            double progress
        )
        {
            var result = new UiLength[left.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = Mix(left[index], right[index], progress);
            return result;
        }

        private static bool CompatiblePolygon(
            IReadOnlyList<IReadOnlyList<UiLength>> left,
            IReadOnlyList<IReadOnlyList<UiLength>> right
        )
        {
            if (left.Count != right.Count)
                return false;
            for (int index = 0; index < left.Count; index++)
                if (left[index].Count != right[index].Count)
                    return false;
            return true;
        }

        private static IReadOnlyList<IReadOnlyList<UiLength>> MixPolygon(
            IReadOnlyList<IReadOnlyList<UiLength>> left,
            IReadOnlyList<IReadOnlyList<UiLength>> right,
            double progress
        )
        {
            var result = new IReadOnlyList<UiLength>[left.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = MixLengths(left[index], right[index], progress);
            return result;
        }

        private static double Lerp(double left, double right, double progress) =>
            left + (right - left) * progress;

        private static double RootMix(double left, double right, double progress) =>
            Math.Sqrt(Math.Max(0, left * left + progress * (right * right - left * left)));
    }
}
