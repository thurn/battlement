#nullable enable

using System;

namespace Battlement.UI
{
    internal readonly struct MotionScalarSample
    {
        public MotionScalarSample(
            double value,
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

        public double Value { get; }
        public double Velocity { get; }
        public bool Done { get; }
        public uint Iteration { get; }
        public ulong LocalMicros { get; }
    }

    internal static class BattlementMotionScalarSampler
    {
        private const double SafeMinimum = 0.001;

        public static MotionScalarSample Sample(
            double origin,
            double target,
            double incomingVelocity,
            TransitionDefinition transition,
            ulong elapsedMicros
        )
        {
            double activeMicros = elapsedMicros - (double)transition.DelayMicros;
            if (activeMicros < 0)
                return new MotionScalarSample(origin, 0, false, 0, 0);

            return transition.Generator switch
            {
                TransitionGenerator.Immediate => new MotionScalarSample(target, 0, true, 0, 0),
                TransitionGenerator.Tween tween => SampleTween(
                    origin,
                    target,
                    tween,
                    transition,
                    activeMicros
                ),
                TransitionGenerator.Spring spring => SampleSpringTimeline(
                    origin,
                    target,
                    incomingVelocity,
                    spring.Value,
                    transition,
                    activeMicros
                ),
                TransitionGenerator.Inertia inertia => SampleInertia(origin, inertia, activeMicros),
                _ => throw Invalid("Unknown motion transition generator."),
            };
        }

        public static double Ease(MotionEasing easing, double progress)
        {
            progress = Math.Clamp(progress, 0, 1);
            return easing switch
            {
                MotionEasing.Linear => progress,
                MotionEasing.EaseIn => CubicBezier(0.42, 0, 1, 1, progress),
                MotionEasing.EaseOut => CubicBezier(0, 0, 0.58, 1, progress),
                MotionEasing.EaseInOut => CubicBezier(0.42, 0, 0.58, 1, progress),
                MotionEasing.CubicBezier value => CubicBezier(
                    value.Value[0],
                    value.Value[1],
                    value.Value[2],
                    value.Value[3],
                    progress
                ),
                MotionEasing.Steps value => value.Position == MotionStepPosition.Start
                    ? Math.Min(1, Math.Ceiling(progress * value.Count) / value.Count)
                    : Math.Floor(progress * value.Count) / value.Count,
                _ => throw Invalid("Unknown motion easing."),
            };
        }

        public static (double Stiffness, double Damping) ResolveSpring(
            SpringConfiguration configuration
        ) =>
            configuration switch
            {
                SpringConfiguration.Physical value => (value.Stiffness, value.Damping),
                SpringConfiguration.Duration value => ResolveDurationSpring(
                    value.DurationMicros,
                    value.Bounce,
                    value.Mass
                ),
                SpringConfiguration.VisualDuration value => ResolveVisualSpring(
                    value.DurationMicros,
                    value.Bounce,
                    value.Mass
                ),
                _ => throw Invalid("Unknown spring configuration."),
            };

        private static MotionScalarSample SampleTween(
            double origin,
            double target,
            TransitionGenerator.Tween tween,
            TransitionDefinition transition,
            double activeMicros
        )
        {
            ulong duration = tween.DurationMicros;
            if (duration == 0)
                return new MotionScalarSample(target, 0, true, 0, 0);
            TimelinePosition position = Position(
                activeMicros,
                duration,
                transition.RepeatDelayMicros,
                transition.Repeat
            );
            bool reverse = Reversed(position.Iteration, transition.RepeatType);
            double progress = position.InDelay ? 1 : position.LocalMicros / duration;
            if (reverse)
                progress = 1 - progress;
            MotionEasing easing =
                tween.Easings.Count == 0 ? new MotionEasing.Linear() : tween.Easings[0];
            double eased = Ease(easing, progress);
            double velocity =
                position.InDelay || position.Done ? 0 : (target - origin) / (duration / 1_000_000d);
            if (reverse)
                velocity = -velocity;
            return new MotionScalarSample(
                origin + (target - origin) * eased,
                velocity,
                position.Done,
                position.Iteration,
                (ulong)position.LocalMicros
            );
        }

        private static MotionScalarSample SampleSpringTimeline(
            double origin,
            double target,
            double incomingVelocity,
            SpringConfiguration configuration,
            TransitionDefinition transition,
            double activeMicros
        )
        {
            ulong duration = configuration switch
            {
                SpringConfiguration.Duration value => value.DurationMicros,
                SpringConfiguration.VisualDuration value => value.DurationMicros,
                _ => FindPhysicalDuration(origin, target, incomingVelocity, configuration),
            };
            TimelinePosition position = Position(
                activeMicros,
                duration,
                transition.RepeatDelayMicros,
                transition.Repeat
            );
            bool reverse = Reversed(position.Iteration, transition.RepeatType);
            double from = reverse ? target : origin;
            double to = reverse ? origin : target;
            double velocity =
                transition.RepeatType == MotionRepeatType.Mirror && reverse
                    ? -incomingVelocity
                    : incomingVelocity;
            MotionScalarSample sample = SampleSpring(
                from,
                to,
                velocity,
                configuration,
                (ulong)position.LocalMicros
            );
            return new MotionScalarSample(
                sample.Value,
                position.InDelay ? 0 : sample.Velocity,
                position.Done,
                position.Iteration,
                sample.LocalMicros
            );
        }

        private static MotionScalarSample SampleSpring(
            double origin,
            double target,
            double incomingVelocity,
            SpringConfiguration configuration,
            ulong elapsedMicros
        )
        {
            (double stiffness, double damping) = ResolveSpring(configuration);
            double mass = configuration switch
            {
                SpringConfiguration.Physical physicalValue => physicalValue.Mass,
                SpringConfiguration.Duration durationValue => durationValue.Mass,
                SpringConfiguration.VisualDuration visualValue => visualValue.Mass,
                _ => 1,
            };
            double initialVelocity = configuration is SpringConfiguration.Physical physical
                ? physical.InitialVelocity ?? incomingVelocity
                : 0;
            double restSpeed = configuration is SpringConfiguration.Physical rest
                ? rest.RestSpeed ?? (Math.Abs(target - origin) < 5 ? 0.01 : 2)
                : 0;
            double restDelta = configuration is SpringConfiguration.Physical delta
                ? delta.RestDelta ?? (Math.Abs(target - origin) < 5 ? 0.005 : 0.5)
                : 0;
            double t = elapsedMicros / 1_000_000d;
            (double sampledValue, double velocity) = ClosedSpring(
                origin,
                target,
                initialVelocity,
                stiffness,
                damping,
                mass,
                t
            );
            bool durationDone = configuration switch
            {
                SpringConfiguration.Duration durationValue => elapsedMicros
                    >= durationValue.DurationMicros,
                SpringConfiguration.VisualDuration visualValue => elapsedMicros
                    >= visualValue.DurationMicros,
                _ => false,
            };
            bool physicalDone =
                configuration is SpringConfiguration.Physical
                && Math.Abs(velocity) <= restSpeed
                && Math.Abs(target - sampledValue) <= restDelta;
            bool done = durationDone || physicalDone;
            return new MotionScalarSample(
                done ? target : sampledValue,
                done ? 0 : velocity,
                done,
                0,
                elapsedMicros
            );
        }

        private static (double Value, double Velocity) ClosedSpring(
            double origin,
            double target,
            double velocity,
            double stiffness,
            double damping,
            double mass,
            double t
        )
        {
            double delta = target - origin;
            double omega = Math.Sqrt(stiffness / mass);
            double ratio = damping / (2 * Math.Sqrt(stiffness * mass));
            if (ratio < 1)
            {
                double frequency = omega * Math.Sqrt(1 - ratio * ratio);
                double a = (ratio * omega * delta - velocity) / frequency;
                double envelope = Math.Exp(-ratio * omega * t);
                double sin = Math.Sin(frequency * t);
                double cos = Math.Cos(frequency * t);
                double value = target - envelope * (a * sin + delta * cos);
                double sinCoefficient = ratio * omega * a + delta * frequency;
                double cosCoefficient = ratio * omega * delta - a * frequency;
                return (value, envelope * (sinCoefficient * sin + cosCoefficient * cos));
            }
            if (ratio == 1)
            {
                double c = omega * delta - velocity;
                double envelope = Math.Exp(-omega * t);
                return (target - envelope * (delta + c * t), envelope * (omega * c * t - velocity));
            }
            double damped = omega * Math.Sqrt(ratio * ratio - 1);
            double p = (ratio * omega * delta - velocity) / damped;
            double bounded = Math.Min(damped * t, 300);
            double decay = Math.Exp(-ratio * omega * t);
            double valueOver =
                target - decay * (p * Math.Sinh(bounded) + delta * Math.Cosh(bounded));
            double sinh = ratio * omega * p - delta * damped;
            double cosh = ratio * omega * delta - p * damped;
            return (valueOver, decay * (sinh * Math.Sinh(bounded) + cosh * Math.Cosh(bounded)));
        }

        private static MotionScalarSample SampleInertia(
            double origin,
            TransitionGenerator.Inertia inertia,
            double activeMicros
        )
        {
            double target = inertia.Target switch
            {
                InertiaTarget.Identity => origin + inertia.Power * inertia.InitialVelocity,
                InertiaTarget.NearestMultiple multiple => Math.Round(
                    (origin + inertia.Power * inertia.InitialVelocity) / multiple.Value
                ) * multiple.Value,
                InertiaTarget.FloorMultiple multiple => Math.Floor(
                    (origin + inertia.Power * inertia.InitialVelocity) / multiple.Value
                ) * multiple.Value,
                InertiaTarget.CeilingMultiple multiple => Math.Ceiling(
                    (origin + inertia.Power * inertia.InitialVelocity) / multiple.Value
                ) * multiple.Value,
                InertiaTarget.Clamp clamp => Math.Clamp(
                    origin + inertia.Power * inertia.InitialVelocity,
                    clamp.Min,
                    clamp.Max
                ),
                _ => throw Invalid("Unknown inertia target modifier."),
            };
            double amplitude = target - origin;
            if (
                TryBoundary(origin, target, inertia, out double boundary, out double crossingMicros)
            )
            {
                double crossingVelocity =
                    (target - boundary) * 1_000_000d / inertia.TimeConstantMicros;
                if (activeMicros >= crossingMicros)
                {
                    var bounce = new SpringConfiguration.Physical(
                        inertia.BounceStiffness,
                        inertia.BounceDamping,
                        1,
                        crossingVelocity,
                        null,
                        inertia.RestDelta
                    );
                    MotionScalarSample sample = SampleSpring(
                        boundary,
                        boundary,
                        crossingVelocity,
                        bounce,
                        (ulong)(activeMicros - crossingMicros)
                    );
                    return new MotionScalarSample(
                        sample.Value,
                        sample.Velocity,
                        sample.Done,
                        sample.Iteration,
                        (ulong)activeMicros
                    );
                }
            }
            double decay = Math.Exp(-activeMicros / inertia.TimeConstantMicros);
            double value = target - amplitude * decay;
            double velocity = amplitude * decay * 1_000_000d / inertia.TimeConstantMicros;
            bool done = Math.Abs(target - value) <= inertia.RestDelta;
            return new MotionScalarSample(
                done ? target : value,
                done ? 0 : velocity,
                done,
                0,
                (ulong)activeMicros
            );
        }

        private static bool TryBoundary(
            double origin,
            double target,
            TransitionGenerator.Inertia inertia,
            out double boundary,
            out double crossingMicros
        )
        {
            if (inertia.Maximum is double maximum && target > maximum && origin <= maximum)
                boundary = maximum;
            else if (inertia.Minimum is double minimum && target < minimum && origin >= minimum)
                boundary = minimum;
            else if (
                inertia.Minimum is double lower
                && inertia.Maximum is double upper
                && (origin < lower || origin > upper)
            )
                boundary = Math.Abs(origin - lower) <= Math.Abs(origin - upper) ? lower : upper;
            else if (inertia.Minimum is double minimumOnly && origin < minimumOnly)
                boundary = minimumOnly;
            else if (inertia.Maximum is double maximumOnly && origin > maximumOnly)
                boundary = maximumOnly;
            else
            {
                boundary = 0;
                crossingMicros = 0;
                return false;
            }

            if (origin == boundary)
            {
                crossingMicros = 0;
                return true;
            }
            double ratio = (target - boundary) / (target - origin);
            crossingMicros = ratio <= 0 ? 0 : -(double)inertia.TimeConstantMicros * Math.Log(ratio);
            return true;
        }

        private static (double Stiffness, double Damping) ResolveDurationSpring(
            ulong durationMicros,
            double bounce,
            double mass
        )
        {
            double duration = Math.Clamp(durationMicros / 1_000_000d, 0.01, 10);
            double ratio = Math.Clamp(1 - bounce, 0.05, 1);
            Func<double, double> envelope;
            Func<double, double> derivative;
            if (ratio < 1)
            {
                envelope = frequency =>
                {
                    double decay = frequency * ratio;
                    return SafeMinimum
                        - (decay / (frequency * Math.Sqrt(1 - ratio * ratio)))
                            * Math.Exp(-decay * duration);
                };
                derivative = frequency =>
                {
                    double decay = frequency * ratio;
                    double e = ratio * ratio * frequency * frequency * duration;
                    double factor = -envelope(frequency) + SafeMinimum > 0 ? -1 : 1;
                    return factor
                        * (-e * Math.Exp(-decay * duration))
                        / (frequency * frequency * Math.Sqrt(1 - ratio * ratio));
                };
            }
            else
            {
                envelope = frequency =>
                    -SafeMinimum + Math.Exp(-frequency * duration) * (frequency * duration + 1);
                derivative = frequency =>
                    -Math.Exp(-frequency * duration) * frequency * duration * duration;
            }
            double root = 5 / duration;
            for (int i = 1; i < 12; i++)
                root -= envelope(root) / derivative(root);
            if (!double.IsFinite(root))
                return (100, 10);
            double stiffness = root * root * mass;
            return (stiffness, ratio * 2 * Math.Sqrt(mass * stiffness));
        }

        private static (double Stiffness, double Damping) ResolveVisualSpring(
            ulong durationMicros,
            double bounce,
            double mass
        )
        {
            double duration = Math.Clamp(durationMicros / 1_000_000d, 0.01, 10);
            double root = 2 * Math.PI / (1.2 * duration);
            double stiffness = root * root * mass;
            double ratio = Math.Clamp(1 - bounce, 0.05, 1);
            return (stiffness, 2 * ratio * Math.Sqrt(mass * stiffness));
        }

        private static ulong FindPhysicalDuration(
            double origin,
            double target,
            double velocity,
            SpringConfiguration configuration
        )
        {
            for (ulong elapsed = 0; elapsed < 20_000_000; elapsed += 50_000)
                if (SampleSpring(origin, target, velocity, configuration, elapsed).Done)
                    return elapsed;
            return 20_000_000;
        }

        private static TimelinePosition Position(
            double activeMicros,
            ulong durationMicros,
            ulong repeatDelayMicros,
            MotionRepeat repeat
        )
        {
            double cycle = durationMicros + (double)repeatDelayMicros;
            bool forever = repeat is MotionRepeat.Forever;
            ulong totalIterations = repeat is MotionRepeat.Count count ? count.Value + 1UL : 1;
            double total = forever
                ? double.PositiveInfinity
                : cycle * totalIterations - repeatDelayMicros;
            bool done = activeMicros >= total;
            double bounded = done ? total : activeMicros;
            double rawIteration = Math.Floor(bounded / cycle);
            uint iteration = forever
                ? (uint)Math.Min(rawIteration, uint.MaxValue)
                : (uint)Math.Min(rawIteration, totalIterations - 1);
            double local = done ? durationMicros : bounded - iteration * cycle;
            bool inDelay = local > durationMicros;
            return new TimelinePosition(iteration, Math.Min(local, durationMicros), inDelay, done);
        }

        private static bool Reversed(uint iteration, MotionRepeatType type) =>
            (type is MotionRepeatType.Reverse or MotionRepeatType.Mirror) && iteration % 2 == 1;

        private static double CubicBezier(double x1, double y1, double x2, double y2, double x)
        {
            double t = x;
            for (int i = 0; i < 8; i++)
            {
                double error = Bezier(t, x1, x2) - x;
                double slope = BezierDerivative(t, x1, x2);
                if (Math.Abs(slope) < 1e-7)
                    break;
                t = Math.Clamp(t - error / slope, 0, 1);
            }
            double low = 0;
            double high = 1;
            for (int i = 0; i < 12; i++)
            {
                if (Bezier(t, x1, x2) < x)
                    low = t;
                else
                    high = t;
                t = (low + high) * 0.5;
            }
            return Bezier(t, y1, y2);
        }

        private static double Bezier(double t, double a, double b)
        {
            double inverse = 1 - t;
            return 3 * inverse * inverse * t * a + 3 * inverse * t * t * b + t * t * t;
        }

        private static double BezierDerivative(double t, double a, double b) =>
            3 * (1 - t) * (1 - t) * a + 6 * (1 - t) * t * (b - a) + 3 * t * t * (1 - b);

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);

        private readonly struct TimelinePosition
        {
            public TimelinePosition(uint iteration, double localMicros, bool inDelay, bool done)
            {
                Iteration = iteration;
                LocalMicros = localMicros;
                InDelay = inDelay;
                Done = done;
            }

            public uint Iteration { get; }
            public double LocalMicros { get; }
            public bool InDelay { get; }
            public bool Done { get; }
        }
    }
}
