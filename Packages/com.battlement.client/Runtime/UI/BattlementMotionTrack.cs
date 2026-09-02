#nullable enable

using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal sealed class TrackState
    {
        private MotionValue origin;
        private double incomingVelocity;

        public TrackState(
            MotionPropertyTrack definition,
            MotionValue origin,
            double incomingVelocity
        )
        {
            Definition = definition;
            this.origin = origin;
            this.incomingVelocity = incomingVelocity;
        }

        public MotionPropertyTrack Definition { get; }

        public double Velocity { get; private set; }

        public bool Done { get; private set; }

        public uint Iteration { get; private set; }

        public bool Suppressed { get; private set; }

        public MotionValue Origin => origin;

        public MotionValue End => EndValue();

        public void Adopt(TrackState previous)
        {
            Velocity = previous.Velocity;
            Done = previous.Done;
            Iteration = previous.Iteration;
        }

        public void Reset()
        {
            Velocity = incomingVelocity;
            Done = false;
            Iteration = 0;
            Suppressed = false;
        }

        public void Retarget(VisualElement target)
        {
            origin = BattlementMotionPropertyWriter.Read(target, Definition.Property);
            incomingVelocity = Velocity;
            Reset();
        }

        public void Freeze()
        {
            Velocity = 0;
            Done = true;
        }

        public void ApplyOrigin(VisualElement target) =>
            BattlementMotionPropertyWriter.Write(target, Definition.Property, origin);

        public void ApplyTerminal(VisualElement target) =>
            BattlementMotionPropertyWriter.Write(target, Definition.Property, EndValue());

        public void Sample(
            VisualElement target,
            ulong elapsedMicros,
            MotionPlaybackDirection direction,
            bool suppressed
        )
        {
            Suppressed = suppressed;
            bool reverse =
                direction
                is MotionPlaybackDirection.Reverse
                    or MotionPlaybackDirection.AlternateReverse;
            MotionValue from = reverse ? EndValue() : origin;
            MotionValue to = reverse ? origin : EndValue();
            double velocity = reverse ? -incomingVelocity : incomingVelocity;
            TransitionDefinition transition = DirectedTransition(direction);
            if (
                from is MotionValue.Scalar left
                && to is MotionValue.Scalar right
                && Definition.Values.Count <= 1
            )
            {
                MotionScalarSample scalar = BattlementMotionScalarSampler.Sample(
                    left.Value,
                    right.Value,
                    velocity,
                    transition,
                    elapsedMicros
                );
                BattlementMotionPropertyWriter.WriteScalar(
                    target,
                    Definition.Property,
                    scalar.Value
                );
                Velocity = scalar.Velocity;
                Done = scalar.Done;
                Iteration = scalar.Iteration;
                if (suppressed)
                    Suppress(target);
                return;
            }
            bool reverseSequence =
                direction
                is MotionPlaybackDirection.Reverse
                    or MotionPlaybackDirection.AlternateReverse;
            MotionPropertyTrack definition = new(
                Definition.Property,
                reverseSequence ? Reverse(Definition.Values) : Definition.Values,
                transition,
                reverseSequence ? ReverseTimes(Definition.Times) : Definition.Times
            );
            MotionTrackSample sample = BattlementMotionValueSampler.Sample(
                definition,
                from,
                velocity,
                elapsedMicros
            );
            BattlementMotionPropertyWriter.Write(target, Definition.Property, sample.Value);
            Velocity = sample.Velocity;
            Done = sample.Done;
            Iteration = sample.Iteration;
            if (suppressed)
                Suppress(target);
        }

        private void Suppress(VisualElement target)
        {
            ApplyTerminal(target);
            Velocity = 0;
            if (Definition.Transition.Repeat is not MotionRepeat.Forever)
                Done = true;
        }

        private MotionValue EndValue() =>
            Definition.Values.Count == 0 ? origin : Definition.Values[^1];

        private TransitionDefinition DirectedTransition(MotionPlaybackDirection direction)
        {
            if (direction is MotionPlaybackDirection.Forward or MotionPlaybackDirection.Reverse)
                return Definition.Transition;
            return new TransitionDefinition(
                Definition.Transition.Generator,
                Definition.Transition.DelayMicros,
                Definition.Transition.Repeat,
                Definition.Transition.RepeatDelayMicros,
                MotionRepeatType.Reverse
            );
        }

        private static IReadOnlyList<MotionValue> Reverse(IReadOnlyList<MotionValue> values)
        {
            var result = new MotionValue[values.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = values[values.Count - index - 1];
            return result;
        }

        private static IReadOnlyList<double>? ReverseTimes(IReadOnlyList<double>? times)
        {
            if (times is null)
                return null;
            var result = new double[times.Count];
            for (int index = 0; index < result.Length; index++)
                result[index] = 1 - times[times.Count - index - 1];
            return result;
        }
    }
}
