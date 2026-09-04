#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using UnityColor = UnityEngine.Color;

namespace Battlement.UI
{
    /// <summary>Paints bounded contour shadows and clipped insets.</summary>
    internal static class BattlementPaintShadows
    {
        /// <summary>Clips the shadow of a translated inner box to the painted surface.</summary>
        public static void Inset(
            Painter2D painter,
            IReadOnlyList<Vector2> points,
            IReadOnlyList<Vector2> box,
            Shadow shadow
        )
        {
            painter.fillGradient = default;
            foreach (var sample in Samples(shadow))
            {
                painter.fillColor = sample.Color;
                var remaining = new List<Vector2>(points);
                float orientation = Orientation(box);
                Vector2 translation = new((float)shadow.X, (float)shadow.Y);
                for (int index = 0; index < box.Count && remaining.Count >= 3; index++)
                {
                    Vector2 start = box[index] + translation;
                    Vector2 edge = box[(index + 1) % box.Count] - box[index];
                    if (edge.sqrMagnitude < 0.00000001f)
                        continue;
                    float Project(Vector2 point) => orientation * Cross(edge, point - start);
                    float boundary = sample.Spread * edge.magnitude;
                    Fill(painter, BattlementPaintContour.Clip(remaining, Project, boundary, false));
                    remaining = BattlementPaintContour.Clip(remaining, Project, boundary, true);
                }
            }
        }

        /// <summary>Blurs the contour without accumulating beyond the authored opacity.</summary>
        public static void Outer(Painter2D painter, IReadOnlyList<Vector2> points, Shadow shadow)
        {
            painter.fillGradient = default;
            foreach (var sample in Samples(shadow))
            {
                painter.fillColor = sample.Color;
                Fill(painter, Offset(points, sample.Spread, shadow));
            }
        }

        private static IEnumerable<(float Spread, UnityColor Color)> Samples(Shadow shadow)
        {
            int count =
                shadow.Blur <= 0 ? 1 : Math.Clamp((int)Math.Ceiling(shadow.Blur * 2), 8, 64);
            float total = 0;
            for (int index = 0; index < count; index++)
                total += Weight(index, count);
            float accumulated = 0;
            for (int index = 0; index < count; index++)
            {
                float amount = (float)shadow.Color.Alpha * Weight(index, count) / total;
                float alpha = amount / Mathf.Max(0.00001f, 1 - accumulated);
                accumulated += amount;
                float radius = count == 1 ? 0 : 1.5f - 3f * index / (count - 1);
                yield return (
                    (float)shadow.Spread + (float)shadow.Blur * radius,
                    new UnityColor(
                        (float)shadow.Color.Red,
                        (float)shadow.Color.Green,
                        (float)shadow.Color.Blue,
                        alpha
                    )
                );
            }
        }

        private static float Weight(int index, int count)
        {
            float radius = count == 1 ? 0 : 3f - 6f * index / (count - 1);
            return Mathf.Exp(-0.5f * radius * radius);
        }

        private static IReadOnlyList<Vector2> Offset(
            IReadOnlyList<Vector2> points,
            float distance,
            Shadow shadow
        )
        {
            if (distance < 0)
                return Contract(points, -distance, shadow);
            float orientation = Orientation(points);
            var result = new List<Vector2>(points.Count);
            for (int index = 0; index < points.Count; index++)
            {
                Vector2 point = points[index];
                Vector2 previous = points[(index + points.Count - 1) % points.Count];
                Vector2 next = points[(index + 1) % points.Count];
                Vector2 a = (point - previous).normalized;
                Vector2 b = (next - point).normalized;
                Vector2 normalA = orientation * new Vector2(a.y, -a.x);
                Vector2 normalB = orientation * new Vector2(b.y, -b.x);
                Vector2 normal = normalA + normalB;
                float denominator = Vector2.Dot(normal, normalA);
                Vector2 offset =
                    denominator > 0.00001f ? normal * (distance / denominator) : normalA * distance;
                result.Add(point + offset + new Vector2((float)shadow.X, (float)shadow.Y));
            }
            return result;
        }

        private static IReadOnlyList<Vector2> Contract(
            IReadOnlyList<Vector2> points,
            float distance,
            Shadow shadow
        )
        {
            float orientation = Orientation(points);
            var result = new List<Vector2>(points);
            for (int index = 0; index < points.Count; index++)
            {
                Vector2 start = points[index];
                Vector2 edge = points[(index + 1) % points.Count] - start;
                if (edge.sqrMagnitude < 0.00000001f)
                    continue;
                result = BattlementPaintContour.Clip(
                    result,
                    point => orientation * Cross(edge, point - start),
                    distance * edge.magnitude,
                    true
                );
            }
            Vector2 translation = new((float)shadow.X, (float)shadow.Y);
            for (int index = 0; index < result.Count; index++)
                result[index] += translation;
            return result;
        }

        private static float Orientation(IReadOnlyList<Vector2> points)
        {
            float area = 0;
            for (int index = 0; index < points.Count; index++)
                area += Cross(points[index], points[(index + 1) % points.Count]);
            return area >= 0 ? 1 : -1;
        }

        private static float Cross(Vector2 a, Vector2 b) => a.x * b.y - a.y * b.x;

        private static void Fill(Painter2D painter, IReadOnlyList<Vector2> points)
        {
            if (points.Count < 3)
                return;
            painter.BeginPath();
            painter.MoveTo(points[0]);
            for (int index = 1; index < points.Count; index++)
                painter.LineTo(points[index]);
            painter.ClosePath();
            painter.Fill(FillRule.NonZero);
        }
    }
}
