#nullable enable

using System.Collections.Generic;

namespace Battlement
{
    /// <summary>One typed operation in an authored transform list.</summary>
    public abstract record TransformOperation
    {
        public sealed record Translate(IReadOnlyList<UiLength> Value) : TransformOperation;

        public sealed record Rotate(IReadOnlyList<double> Value) : TransformOperation;

        public sealed record Skew(IReadOnlyList<double> Value) : TransformOperation;

        public sealed record Scale(IReadOnlyList<double> Value) : TransformOperation;
    }

    /// <summary>One outer or inset shadow.</summary>
    public sealed record Shadow(
        double X,
        double Y,
        double Blur,
        double Spread,
        Color Color,
        bool Inset
    );

    /// <summary>One gradient color stop.</summary>
    public sealed record GradientStop(Color Color, double Position);

    /// <summary>A compatible linear or radial gradient.</summary>
    public abstract record Gradient
    {
        public sealed record Linear(double Angle, IReadOnlyList<GradientStop> Stops) : Gradient;

        public sealed record Radial(
            IReadOnlyList<double> Center,
            IReadOnlyList<double> Radius,
            IReadOnlyList<GradientStop> Stops
        ) : Gradient;
    }

    /// <summary>A solid or gradient background fill.</summary>
    public abstract record PaintFill
    {
        public sealed record Color(Battlement.Color Value) : PaintFill;

        public sealed record Gradient(Battlement.Gradient Value) : PaintFill;
    }

    /// <summary>One additional static paint layer drawn over primary paint.</summary>
    public sealed record PaintLayer(
        PaintFill Background,
        IReadOnlyList<UiFilterFunction>? PaintFilter = null,
        IReadOnlyList<IReadOnlyList<UiLength>>? ClipPolygon = null,
        IReadOnlyList<Shadow>? BoxShadow = null,
        IReadOnlyList<UiLength>? ClipInset = null,
        IReadOnlyList<UiLength>? BoundsInset = null
    );

    /// <summary>Static decorative paint in element border-box coordinates.</summary>
    public sealed record PaintStyle(
        PaintFill? Background = null,
        IReadOnlyList<UiFilterFunction>? PaintFilter = null,
        IReadOnlyList<IReadOnlyList<UiLength>>? ClipPolygon = null,
        IReadOnlyList<Shadow>? BoxShadow = null,
        IReadOnlyList<UiLength>? ClipInset = null,
        IReadOnlyList<PaintLayer>? Layers = null
    );
}
