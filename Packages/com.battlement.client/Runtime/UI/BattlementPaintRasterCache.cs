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
}
