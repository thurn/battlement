#nullable enable

using System.Collections.Generic;
using Newtonsoft.Json;

namespace Battlement
{
    /// <summary>Boundary placement for a stepped easing function.</summary>
    public enum MotionStepPosition
    {
        Start,
        End,
    }

    /// <summary>Typed easing accepted by tween timelines.</summary>
    public abstract record MotionEasing
    {
        public sealed record Linear : MotionEasing;

        public sealed record EaseIn : MotionEasing;

        public sealed record EaseOut : MotionEasing;

        public sealed record EaseInOut : MotionEasing;

        public sealed record CubicBezier(IReadOnlyList<double> Value) : MotionEasing;

        public sealed record Steps(uint Count, MotionStepPosition Position) : MotionEasing;
    }

    /// <summary>Number of additional Motion iterations after the first.</summary>
    public abstract record MotionRepeat
    {
        public sealed record None : MotionRepeat;

        public sealed record Count(uint Value) : MotionRepeat;

        public sealed record Forever : MotionRepeat;
    }

    /// <summary>How later Motion iterations derive direction and endpoints.</summary>
    public enum MotionRepeatType
    {
        Loop,
        Reverse,
        Mirror,
    }

    /// <summary>Serializable inertia target modifier.</summary>
    public abstract record InertiaTarget
    {
        public sealed record Identity : InertiaTarget;

        public sealed record NearestMultiple(double Value) : InertiaTarget;

        public sealed record FloorMultiple(double Value) : InertiaTarget;

        public sealed record CeilingMultiple(double Value) : InertiaTarget;

        public sealed record Clamp(double Min, double Max) : InertiaTarget;
    }

    /// <summary>Physical-parameter or duration-derived spring configuration.</summary>
    public abstract record SpringConfiguration
    {
        public sealed record Physical(
            double Stiffness,
            double Damping,
            double Mass,
            [property: JsonProperty(NullValueHandling = NullValueHandling.Ignore)]
                double? InitialVelocity,
            [property: JsonProperty(NullValueHandling = NullValueHandling.Ignore)]
                double? RestSpeed,
            [property: JsonProperty(NullValueHandling = NullValueHandling.Ignore)] double? RestDelta
        ) : SpringConfiguration;

        public sealed record Duration(ulong DurationMicros, double Bounce, double Mass)
            : SpringConfiguration;

        public sealed record VisualDuration(ulong DurationMicros, double Bounce, double Mass)
            : SpringConfiguration;
    }

    /// <summary>Fully normalized timing generator for one property track.</summary>
    public abstract record TransitionGenerator
    {
        public sealed record Immediate : TransitionGenerator;

        public sealed record Tween(
            ulong DurationMicros,
            IReadOnlyList<MotionEasing> Easings,
            IReadOnlyList<double>? Times = null
        ) : TransitionGenerator;

        public sealed record Spring(SpringConfiguration Value) : TransitionGenerator;

        public sealed record Inertia(
            double InitialVelocity,
            double Power,
            ulong TimeConstantMicros,
            [property: JsonProperty(NullValueHandling = NullValueHandling.Ignore)] double? Minimum,
            [property: JsonProperty(NullValueHandling = NullValueHandling.Ignore)] double? Maximum,
            double RestDelta,
            double BounceStiffness,
            double BounceDamping,
            InertiaTarget Target
        ) : TransitionGenerator;
    }

    /// <summary>A generator plus delay and repetition semantics.</summary>
    public sealed record TransitionDefinition(
        TransitionGenerator Generator,
        long DelayMicros,
        MotionRepeat Repeat,
        ulong RepeatDelayMicros,
        MotionRepeatType RepeatType
    );

    /// <summary>One property-specific timing override.</summary>
    public sealed record PropertyTransition(
        MotionProperty Property,
        TransitionDefinition Transition
    );

    /// <summary>Default transition plus property-specific replacements.</summary>
    public sealed record MotionTransition(
        TransitionDefinition Default,
        IReadOnlyList<PropertyTransition> Properties
    );
}
