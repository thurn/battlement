#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    /// <summary>A closed discrete Motion value.</summary>
    public abstract record MotionDiscreteValue
    {
        public sealed record Null : MotionDiscreteValue;

        public sealed record String(string Value) : MotionDiscreteValue;

        public static implicit operator MotionDiscreteValue(string value) => new String(value);
    }

    /// <summary>Every normalized value shape accepted by a motion property.</summary>
    public abstract record MotionValue
    {
        public sealed record Scalar(double Value) : MotionValue;

        public sealed record Length(UiLength Value) : MotionValue;

        public sealed record Color(Battlement.Color Value) : MotionValue;

        public sealed record Vector2(IReadOnlyList<double> Value) : MotionValue;

        public sealed record Vector3(IReadOnlyList<double> Value) : MotionValue;

        public sealed record Angle(double Value) : MotionValue;

        public sealed record TransformList(IReadOnlyList<TransformOperation> Value) : MotionValue;

        public sealed record FilterList(IReadOnlyList<UiFilterFunction> Value) : MotionValue;

        public sealed record ShadowList(IReadOnlyList<Shadow> Value) : MotionValue;

        public sealed record Gradient(Battlement.Gradient Value) : MotionValue;

        public sealed record ClipInset(IReadOnlyList<UiLength> Value) : MotionValue;

        public sealed record ClipPolygon(IReadOnlyList<IReadOnlyList<UiLength>> Value)
            : MotionValue;

        public sealed record Discrete(MotionDiscreteValue Value) : MotionValue;
    }
}
