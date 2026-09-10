#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityRect = UnityEngine.Rect;

namespace Battlement.UI
{
    internal readonly struct CachedPaint
    {
        public CachedPaint(Texture2D texture, UnityRect bounds)
        {
            Texture = texture;
            Bounds = bounds;
        }

        public Texture2D Texture { get; }

        public UnityRect Bounds { get; }
    }

    internal sealed class BattlementPaintRasterCacheKey : IEquatable<BattlementPaintRasterCacheKey>
    {
        private readonly PaintFill fill;
        private readonly UiFilterFunction[] filters;
        private readonly Texture2D? mask;
        private readonly Vector2[] points;
        private readonly UnityRect rect;
        private readonly int hash;

        public BattlementPaintRasterCacheKey(
            UnityRect rect,
            IReadOnlyList<Vector2> points,
            PaintFill fill,
            IReadOnlyList<UiFilterFunction> filters,
            Texture2D? mask
        )
        {
            this.rect = rect;
            this.points = points.ToArray();
            this.fill = fill;
            this.filters = filters.ToArray();
            this.mask = mask;
            var hash = new HashCode();
            hash.Add(rect);
            hash.Add(mask);
            foreach (Vector2 point in points)
                hash.Add(point);
            AddFill(ref hash, fill);
            foreach (UiFilterFunction filter in filters)
                hash.Add(filter);
            this.hash = hash.ToHashCode();
        }

        public bool Equals(BattlementPaintRasterCacheKey? other) =>
            other is not null
            && rect == other.rect
            && ReferenceEquals(mask, other.mask)
            && points.SequenceEqual(other.points)
            && filters.SequenceEqual(other.filters)
            && Same(fill, other.fill);

        public override bool Equals(object? obj) =>
            obj is BattlementPaintRasterCacheKey other && Equals(other);

        public override int GetHashCode() => hash;

        private static void AddFill(ref HashCode hash, PaintFill value)
        {
            switch (value)
            {
                case PaintFill.Color color:
                    hash.Add(0);
                    hash.Add(color.Value);
                    break;
                case PaintFill.Gradient { Value: Gradient.Linear linear }:
                    hash.Add(1);
                    hash.Add(linear.Angle);
                    foreach (GradientStop stop in linear.Stops)
                        hash.Add(stop);
                    break;
                case PaintFill.Gradient { Value: Gradient.Radial radial }:
                    hash.Add(2);
                    foreach (double coordinate in radial.Center)
                        hash.Add(coordinate);
                    foreach (double radius in radial.Radius)
                        hash.Add(radius);
                    foreach (GradientStop stop in radial.Stops)
                        hash.Add(stop);
                    break;
                default:
                    throw new InvalidOperationException("Unknown paint fill.");
            }
        }

        private static bool Same(PaintFill left, PaintFill right) =>
            (left, right) switch
            {
                (PaintFill.Color x, PaintFill.Color y) => x.Value == y.Value,
                (PaintFill.Gradient x, PaintFill.Gradient y) => MotionGraphDefinitionEquality.Same(
                    new MotionValue.Gradient(x.Value),
                    new MotionValue.Gradient(y.Value)
                ),
                _ => false,
            };
    }

    internal static class BattlementPaintRasterCache
    {
        private const long MaximumPixels = 64L * 1024 * 1024;
        private static readonly Dictionary<BattlementPaintRasterCacheKey, CachedPaint> Entries =
            new();
        private static long pixels;

        public static bool TryGet(BattlementPaintRasterCacheKey key, out CachedPaint paint) =>
            Entries.TryGetValue(key, out paint);

        public static bool TryStore(
            BattlementPaintRasterCacheKey key,
            Texture2D texture,
            UnityRect bounds
        )
        {
            long count = (long)texture.width * texture.height;
            if (count > MaximumPixels - pixels)
                return false;
            Entries.Add(key, new CachedPaint(texture, bounds));
            pixels += count;
            return true;
        }
    }

    internal sealed class BattlementPaintShadowCacheKey : IEquatable<BattlementPaintShadowCacheKey>
    {
        private readonly UnityRect bounds;
        private readonly PaintFill fill;
        private readonly Texture2D? mask;
        private readonly Vector2[] points;
        private readonly UnityRect rect;
        private readonly double blur;
        private readonly double spread;
        private readonly int hash;

        public BattlementPaintShadowCacheKey(
            UnityRect bounds,
            UnityRect rect,
            IReadOnlyList<Vector2> points,
            PaintFill fill,
            Texture2D? mask,
            Shadow shadow
        )
        {
            this.bounds = bounds;
            this.rect = rect;
            this.points = points.ToArray();
            this.fill = fill;
            this.mask = mask;
            blur = shadow.Blur;
            spread = shadow.Spread;
            var hash = new HashCode();
            hash.Add(bounds);
            hash.Add(rect);
            hash.Add(mask);
            hash.Add(blur);
            hash.Add(spread);
            foreach (Vector2 point in points)
                hash.Add(point);
            AddAlpha(ref hash, fill);
            this.hash = hash.ToHashCode();
        }

        public bool Equals(BattlementPaintShadowCacheKey? other) =>
            other is not null
            && bounds == other.bounds
            && rect == other.rect
            && ReferenceEquals(mask, other.mask)
            && blur == other.blur
            && spread == other.spread
            && points.SequenceEqual(other.points)
            && SameAlpha(fill, other.fill);

        public override bool Equals(object? obj) =>
            obj is BattlementPaintShadowCacheKey other && Equals(other);

        public override int GetHashCode() => hash;

        private static void AddAlpha(ref HashCode hash, PaintFill value)
        {
            switch (value)
            {
                case PaintFill.Color color:
                    hash.Add(0);
                    hash.Add(color.Value.Alpha);
                    break;
                case PaintFill.Gradient { Value: Gradient.Linear linear }:
                    hash.Add(1);
                    hash.Add(linear.Angle);
                    AddStops(ref hash, linear.Stops);
                    break;
                case PaintFill.Gradient { Value: Gradient.Radial radial }:
                    hash.Add(2);
                    foreach (double coordinate in radial.Center)
                        hash.Add(coordinate);
                    foreach (double radius in radial.Radius)
                        hash.Add(radius);
                    AddStops(ref hash, radial.Stops);
                    break;
                default:
                    throw new InvalidOperationException("Unknown paint fill.");
            }
        }

        private static void AddStops(ref HashCode hash, IReadOnlyList<GradientStop> stops)
        {
            foreach (GradientStop stop in stops)
            {
                hash.Add(stop.Position);
                hash.Add(stop.Color.Alpha);
            }
        }

        private static bool SameAlpha(PaintFill left, PaintFill right) =>
            (left, right) switch
            {
                (PaintFill.Color x, PaintFill.Color y) => x.Value.Alpha == y.Value.Alpha,
                (
                    PaintFill.Gradient { Value: Gradient.Linear x },
                    PaintFill.Gradient { Value: Gradient.Linear y }
                ) => x.Angle == y.Angle && SameStops(x.Stops, y.Stops),
                (
                    PaintFill.Gradient { Value: Gradient.Radial x },
                    PaintFill.Gradient { Value: Gradient.Radial y }
                ) => x.Center.SequenceEqual(y.Center)
                    && x.Radius.SequenceEqual(y.Radius)
                    && SameStops(x.Stops, y.Stops),
                _ => false,
            };

        private static bool SameStops(
            IReadOnlyList<GradientStop> left,
            IReadOnlyList<GradientStop> right
        ) =>
            left.Count == right.Count
            && left.Zip(right, (x, y) => x.Position == y.Position && x.Color.Alpha == y.Color.Alpha)
                .All(equal => equal);
    }

    internal static class BattlementPaintShadowCache
    {
        private const long MaximumSamples = 32L * 1024 * 1024;
        private static readonly Dictionary<BattlementPaintShadowCacheKey, float[]> Entries = new();
        private static long samples;

        public static bool TryGet(BattlementPaintShadowCacheKey key, out float[] alpha) =>
            Entries.TryGetValue(key, out alpha);

        public static void TryStore(BattlementPaintShadowCacheKey key, float[] alpha)
        {
            if (alpha.LongLength > MaximumSamples - samples)
                return;
            Entries.Add(key, alpha);
            samples += alpha.LongLength;
        }
    }
}
