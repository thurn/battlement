#nullable enable

using System.Collections.Generic;

namespace Battlement.UI
{
    internal sealed class TrackState
    {
        private MotionValue origin;
        private double incomingVelocity;
        private ulong anchorElapsed;

        public TrackState(
            MotionPropertyTrack definition,
            MotionValue origin,
            double incomingVelocity
        )
        {
            Definition = definition;
            this.origin = origin;
            this.incomingVelocity = incomingVelocity;
            Velocity = incomingVelocity;
        }

        public MotionPropertyTrack Definition { get; private set; }

        public double Velocity { get; private set; }

        public bool Done { get; private set; }

        public bool IsInfinite => Definition.Transition.Repeat is MotionRepeat.Forever;

        public uint Iteration { get; private set; }

        public bool Suppressed { get; private set; }

        public MotionValue Origin => origin;

        public MotionValue End => EndValue();

        public void Adopt(TrackState previous)
        {
            origin = previous.origin;
            incomingVelocity = previous.incomingVelocity;
            Velocity = previous.Velocity;
            Suppressed = previous.Suppressed;
            Done = previous.Done;
            Iteration = previous.Iteration;
            anchorElapsed = previous.anchorElapsed;
        }

        public void Reset()
        {
            Velocity = incomingVelocity;
            Done = false;
            Iteration = 0;
            Suppressed = false;
            anchorElapsed = 0;
        }

        public void Retarget(IBattlementMotionTarget target)
        {
            origin = target.Read(Definition.Property);
            incomingVelocity = Velocity;
            Reset();
        }

        public void RetargetDestination(
            IBattlementMotionTarget target,
            MotionValue value,
            ulong elapsedMicros
        )
        {
            if (Equals(Definition.Values[^1], value))
                return;
            origin = target.Read(Definition.Property);
            incomingVelocity = Velocity;
            Definition = Definition with { Values = new[] { value } };
            Velocity = incomingVelocity;
            Done = false;
            Iteration = 0;
            Suppressed = false;
            anchorElapsed = elapsedMicros;
        }

        public void Freeze()
        {
            Velocity = 0;
            Done = true;
        }

        public void ApplyOrigin(IBattlementMotionTarget target) =>
            target.Write(Definition.Property, origin);

        public void ApplyTerminal(IBattlementMotionTarget target) =>
            target.Write(Definition.Property, EndValue());

        public void Sample(
            IBattlementMotionTarget target,
            ulong elapsedMicros,
            MotionPlaybackDirection direction,
            bool suppressed,
            bool write = true
        )
        {
            elapsedMicros = elapsedMicros >= anchorElapsed ? elapsedMicros - anchorElapsed : 0;
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
                if (write)
                    target.WriteScalar(Definition.Property, scalar.Value);
                Velocity = scalar.Velocity;
                Done = scalar.Done;
                Iteration = scalar.Iteration;
                if (suppressed)
                    Suppress(target, write);
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
            if (write)
                target.Write(Definition.Property, sample.Value);
            Velocity = sample.Velocity;
            Done = sample.Done;
            Iteration = sample.Iteration;
            if (suppressed)
                Suppress(target, write);
        }

        private void Suppress(IBattlementMotionTarget target, bool write)
        {
            if (write)
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
