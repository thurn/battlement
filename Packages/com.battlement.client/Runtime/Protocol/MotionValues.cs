#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    /// <summary>A length preserving pixel and percentage components.</summary>
    public sealed record MotionLength(double Px, double Percent);

    /// <summary>A Motion-compatible linear RGBA color.</summary>
    public sealed record MotionColor(double Red, double Green, double Blue, double Alpha);

    /// <summary>One typed operation in an authored transform list.</summary>
    public abstract record MotionTransform
    {
        public sealed record Translate(IReadOnlyList<MotionLength> Value) : MotionTransform;

        public sealed record Rotate(IReadOnlyList<double> Value) : MotionTransform;

        public sealed record Skew(IReadOnlyList<double> Value) : MotionTransform;

        public sealed record Scale(IReadOnlyList<double> Value) : MotionTransform;
    }

    /// <summary>One outer or inset shadow.</summary>
    public sealed record MotionShadow(
        double X,
        double Y,
        double Blur,
        double Spread,
        MotionColor Color,
        bool Inset
    );

    /// <summary>One supported filter operation.</summary>
    public abstract record MotionFilter
    {
        public sealed record Blur(double Value) : MotionFilter;

        public sealed record Brightness(double Value) : MotionFilter;

        public sealed record Saturate(double Value) : MotionFilter;

        public sealed record Contrast(double Value) : MotionFilter;

        public sealed record HueRotate(double Value) : MotionFilter;

        public sealed record Opacity(double Value) : MotionFilter;

        public sealed record DropShadow(MotionShadow Value) : MotionFilter;
    }

    /// <summary>One gradient color stop.</summary>
    public sealed record MotionGradientStop(MotionColor Color, double Position);

    /// <summary>A compatible linear or radial gradient.</summary>
    public abstract record MotionGradient
    {
        public sealed record Linear(double Angle, IReadOnlyList<MotionGradientStop> Stops)
            : MotionGradient;

        public sealed record Radial(
            IReadOnlyList<double> Center,
            IReadOnlyList<double> Radius,
            IReadOnlyList<MotionGradientStop> Stops
        ) : MotionGradient;
    }

    /// <summary>Every normalized value shape accepted by a motion property.</summary>
    public abstract record MotionValue
    {
        public sealed record Scalar(double Value) : MotionValue;

        public sealed record Length(MotionLength Value) : MotionValue;

        public sealed record Color(MotionColor Value) : MotionValue;

        public sealed record Vector2(IReadOnlyList<double> Value) : MotionValue;

        public sealed record Vector3(IReadOnlyList<double> Value) : MotionValue;

        public sealed record Angle(double Value) : MotionValue;

        public sealed record TransformList(IReadOnlyList<MotionTransform> Value) : MotionValue;

        public sealed record FilterList(IReadOnlyList<MotionFilter> Value) : MotionValue;

        public sealed record ShadowList(IReadOnlyList<MotionShadow> Value) : MotionValue;

        public sealed record Gradient(MotionGradient Value) : MotionValue;

        public sealed record ClipInset(IReadOnlyList<MotionLength> Value) : MotionValue;

        public sealed record ClipPolygon(IReadOnlyList<IReadOnlyList<MotionLength>> Value)
            : MotionValue;

        public sealed record Discrete(Newtonsoft.Json.Linq.JToken Value) : MotionValue;
    }
}
