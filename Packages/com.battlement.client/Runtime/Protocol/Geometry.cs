#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement
{
    /// <summary>Identifies one geometry observation epoch.</summary>
    public readonly struct GeometryObservationId : IEquatable<GeometryObservationId>
    {
        public GeometryObservationId(Guid value) => Value = value;

        public Guid Value { get; }

        public bool Equals(GeometryObservationId other) => Value.Equals(other.Value);

        public override bool Equals(object? obj) =>
            obj is GeometryObservationId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();
    }

    /// <summary>Identifies one complete native sampling pass.</summary>
    public readonly struct GeometryGeneration : IEquatable<GeometryGeneration>
    {
        public GeometryGeneration(ulong value) => Value = value;

        public ulong Value { get; }

        public bool Equals(GeometryGeneration other) => Value == other.Value;

        public override bool Equals(object? obj) =>
            obj is GeometryGeneration other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();
    }

    /// <summary>Identifies one physical display.</summary>
    public readonly struct DisplayId : IEquatable<DisplayId>
    {
        public DisplayId(uint value) => Value = value;

        public uint Value { get; }

        public bool Equals(DisplayId other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is DisplayId other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();
    }

    /// <summary>Names one authored world-space geometry anchor.</summary>
    public readonly struct AnchorName : IEquatable<AnchorName>
    {
        public AnchorName(string value) => Value = value;

        public string Value { get; }

        public bool Equals(AnchorName other) => Value == other.Value;

        public override bool Equals(object? obj) => obj is AnchorName other && Equals(other);

        public override int GetHashCode() => Value.GetHashCode();
    }

    /// <summary>Selects the camera used to project a world target.</summary>
    public abstract record CameraTarget
    {
        private CameraTarget() { }

        public sealed record Input : CameraTarget;

        public sealed record Object(ObjectId ObjectId) : CameraTarget;
    }

    /// <summary>A row-major three-by-three projective transform.</summary>
    public sealed record Projective2(
        double M11,
        double M12,
        double M13,
        double M21,
        double M22,
        double M23,
        double M31,
        double M32,
        double M33
    );

    /// <summary>A point in upper-left-origin physical display coordinates.</summary>
    public sealed record ViewportPoint(double X, double Y, DisplayId DisplayId);

    /// <summary>A rectangle in upper-left-origin physical display coordinates.</summary>
    public sealed record ViewportRect(
        double X,
        double Y,
        double Width,
        double Height,
        DisplayId DisplayId
    );

    /// <summary>Geometry measured for one UI element.</summary>
    public sealed record ElementGeometry(
        Rect Layout,
        ViewportRect ViewportBound,
        Projective2 ViewportFromLocal,
        Projective2 ViewportFromParent,
        ObjectId PanelId
    );

    /// <summary>Physical display orientation.</summary>
    public enum DisplayOrientation
    {
        Landscape,
        LandscapeFlipped,
        Portrait,
        PortraitFlipped,
    }

    /// <summary>Geometry measured for one display viewport.</summary>
    public sealed record ViewportGeometry(
        ViewportRect Viewport,
        ViewportRect SafeArea,
        double Scale,
        double? Dpi,
        DisplayOrientation Orientation
    );

    /// <summary>Geometry measured for a projected world point.</summary>
    public sealed record WorldPointGeometry(
        ViewportPoint Point,
        double Depth,
        bool IsInsideViewport
    );

    /// <summary>Geometry measured for projected rendered bounds.</summary>
    public sealed record WorldBoundsGeometry(
        ViewportRect Bound,
        double NearestDepth,
        double FarthestDepth,
        bool IsInsideViewport
    );

    /// <summary>One target installed in the native observation registry.</summary>
    public abstract record GeometryObservationTarget
    {
        private GeometryObservationTarget() { }

        public sealed record UiElement(ObjectId ObjectId) : GeometryObservationTarget;

        public sealed record Viewport(DisplayId DisplayId) : GeometryObservationTarget;

        public sealed record WorldOrigin(ObjectId ObjectId, CameraTarget Camera)
            : GeometryObservationTarget;

        public sealed record WorldAnchor(ObjectId ObjectId, AnchorName Anchor, CameraTarget Camera)
            : GeometryObservationTarget;

        public sealed record WorldRenderedBounds(ObjectId ObjectId, CameraTarget Camera)
            : GeometryObservationTarget;
    }

    /// <summary>Associates one observation epoch with its target.</summary>
    public sealed record GeometryObservation(
        GeometryObservationId ObservationId,
        GeometryObservationTarget Target
    );

    /// <summary>One atomic registry update.</summary>
    public sealed record GeometryObservationUpdate(
        IReadOnlyList<GeometryObservation> Added,
        IReadOnlyList<GeometryObservationId> Removed
    );

    /// <summary>A successfully sampled geometry value.</summary>
    public abstract record GeometryValue
    {
        private GeometryValue() { }

        public sealed record Element(ElementGeometry Value) : GeometryValue;

        public sealed record Viewport(ViewportGeometry Value) : GeometryValue;

        public sealed record WorldPoint(WorldPointGeometry Value) : GeometryValue;

        public sealed record WorldBounds(WorldBoundsGeometry Value) : GeometryValue;
    }

    /// <summary>A temporary reason an observation could not be sampled.</summary>
    public enum GeometryUnavailable
    {
        Detached,
        Hidden,
        ObjectMissing,
        CameraDisabled,
        DisplayUnavailable,
        NoRenderers,
        BehindCamera,
        NoViewportMapping,
        ProjectionUnavailable,
    }

    /// <summary>The result of sampling one observation.</summary>
    public abstract record GeometryObservationResult
    {
        private GeometryObservationResult() { }

        public sealed record Current(GeometryValue Value) : GeometryObservationResult;

        public sealed record Unavailable(GeometryUnavailable Reason) : GeometryObservationResult;
    }

    /// <summary>One changed observation in a sampling pass.</summary>
    public sealed record GeometryObservationValue(
        GeometryObservationId ObservationId,
        GeometryObservationResult Result
    );

    /// <summary>Changed values from one complete native sampling pass.</summary>
    public sealed record GeometryObservationBatch(
        GeometryGeneration Generation,
        IReadOnlyList<GeometryObservationValue> Changed
    );

    /// <summary>Validates registry changes and batches atomically.</summary>
    public sealed class GeometryRegistry
    {
        private Dictionary<GeometryObservationId, GeometryObservationTarget> targets = new();
        private GeometryGeneration? generation;

        public IReadOnlyDictionary<GeometryObservationId, GeometryObservationTarget> Targets =>
            targets;

        public void Apply(GeometryObservationUpdate update)
        {
            var next = new Dictionary<GeometryObservationId, GeometryObservationTarget>(targets);
            var removed = new HashSet<GeometryObservationId>();
            foreach (GeometryObservationId id in update.Removed)
            {
                if (!removed.Add(id))
                    throw new ArgumentException("A removed geometry observation ID is duplicated.");
                if (!next.Remove(id))
                    throw new ArgumentException("A removed geometry observation is unknown.");
            }
            foreach (GeometryObservation observation in update.Added)
            {
                if (removed.Contains(observation.ObservationId))
                    throw new ArgumentException("A geometry observation epoch cannot be reused.");
                if (
                    observation.Target is GeometryObservationTarget.WorldAnchor anchor
                    && string.IsNullOrEmpty(anchor.Anchor.Value)
                )
                    throw new ArgumentException("A geometry anchor name must be nonempty.");
                if (!next.TryAdd(observation.ObservationId, observation.Target))
                    throw new ArgumentException("A geometry observation ID is duplicated.");
            }
            targets = next;
        }

        public void Accept(GeometryObservationBatch batch)
        {
            if (
                batch.Generation.Value == 0
                || generation is { } previous && batch.Generation.Value <= previous.Value
            )
                throw new ArgumentException(
                    "A geometry generation must increase from a nonzero value."
                );
            var seen = new HashSet<GeometryObservationId>();
            foreach (GeometryObservationValue changed in batch.Changed)
            {
                if (
                    !targets.TryGetValue(
                        changed.ObservationId,
                        out GeometryObservationTarget target
                    )
                )
                    throw new ArgumentException("A geometry observation ID is unknown.");
                if (!seen.Add(changed.ObservationId))
                    throw new ArgumentException("A geometry observation ID is duplicated.");
                Validate(target, changed.Result);
            }
            generation = batch.Generation;
        }

        private static void Validate(
            GeometryObservationTarget target,
            GeometryObservationResult result
        )
        {
            if (result is not GeometryObservationResult.Current current)
                return;
            bool kindMatches =
                target is GeometryObservationTarget.UiElement
                    && current.Value is GeometryValue.Element
                || target is GeometryObservationTarget.Viewport
                    && current.Value is GeometryValue.Viewport
                || target is GeometryObservationTarget.WorldOrigin
                    && current.Value is GeometryValue.WorldPoint
                || target is GeometryObservationTarget.WorldAnchor
                    && current.Value is GeometryValue.WorldPoint
                || target is GeometryObservationTarget.WorldRenderedBounds
                    && current.Value is GeometryValue.WorldBounds;
            if (!kindMatches)
                throw new ArgumentException(
                    "A geometry value does not match its registered target."
                );
            Validate(current.Value);
        }

        private static void Validate(GeometryValue value)
        {
            switch (value)
            {
                case GeometryValue.Element element:
                    Finite(
                        element.Value.Layout.X,
                        element.Value.Layout.Y,
                        element.Value.Layout.Width,
                        element.Value.Layout.Height
                    );
                    Finite(element.Value.ViewportBound);
                    Projective(element.Value.ViewportFromLocal);
                    Projective(element.Value.ViewportFromParent);
                    break;
                case GeometryValue.Viewport viewport:
                    Finite(viewport.Value.Viewport);
                    Finite(viewport.Value.SafeArea);
                    Finite(viewport.Value.Scale, viewport.Value.Dpi ?? 0);
                    break;
                case GeometryValue.WorldPoint point:
                    Finite(point.Value.Point.X, point.Value.Point.Y, point.Value.Depth);
                    break;
                case GeometryValue.WorldBounds bounds:
                    Finite(bounds.Value.Bound);
                    Finite(bounds.Value.NearestDepth, bounds.Value.FarthestDepth);
                    break;
                default:
                    throw new ArgumentOutOfRangeException(nameof(value));
            }
        }

        private static void Finite(ViewportRect value) =>
            Finite(value.X, value.Y, value.Width, value.Height);

        private static void Finite(params double[] values)
        {
            foreach (double value in values)
                if (double.IsNaN(value) || double.IsInfinity(value))
                    throw new ArgumentException("Geometry numbers must be finite.");
        }

        private static void Projective(Projective2 value)
        {
            Finite(
                value.M11,
                value.M12,
                value.M13,
                value.M21,
                value.M22,
                value.M23,
                value.M31,
                value.M32,
                value.M33
            );
            double determinant =
                value.M11 * (value.M22 * value.M33 - value.M23 * value.M32)
                - value.M12 * (value.M21 * value.M33 - value.M23 * value.M31)
                + value.M13 * (value.M21 * value.M32 - value.M22 * value.M31);
            if (double.IsNaN(determinant) || double.IsInfinity(determinant) || determinant == 0)
                throw new ArgumentException("A projective transform must be invertible.");
        }
    }

    public abstract partial record CommandBody
    {
        public sealed record GeometryObservation(GeometryObservationUpdate Value) : CommandBody;
    }
}
