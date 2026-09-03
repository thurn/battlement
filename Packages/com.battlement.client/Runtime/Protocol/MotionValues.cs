#nullable enable

using System.Collections.Generic;

namespace Battlement
{
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

        public sealed record Discrete(Newtonsoft.Json.Linq.JToken Value) : MotionValue;
    }
}
