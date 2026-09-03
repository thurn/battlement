#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using UnityRect = UnityEngine.Rect;

namespace Battlement.UI
{
    internal static class BattlementPaintContour
    {
        public static List<Vector2> RoundedBox(UnityRect rect, IResolvedStyle style)
        {
            float topLeft = Mathf.Max(0, style.borderTopLeftRadius);
            float topRight = Mathf.Max(0, style.borderTopRightRadius);
            float bottomRight = Mathf.Max(0, style.borderBottomRightRadius);
            float bottomLeft = Mathf.Max(0, style.borderBottomLeftRadius);
            float scale = Mathf.Min(
                1,
                Fit(rect.width, topLeft + topRight),
                Fit(rect.width, bottomLeft + bottomRight),
                Fit(rect.height, topLeft + bottomLeft),
                Fit(rect.height, topRight + bottomRight)
            );
            topLeft *= scale;
            topRight *= scale;
            bottomRight *= scale;
            bottomLeft *= scale;
            var points = new List<Vector2>();
            Arc(points, new Vector2(rect.xMin + topLeft, rect.yMin + topLeft), topLeft, 180);
            Arc(points, new Vector2(rect.xMax - topRight, rect.yMin + topRight), topRight, 270);
            Arc(
                points,
                new Vector2(rect.xMax - bottomRight, rect.yMax - bottomRight),
                bottomRight,
                0
            );
            Arc(
                points,
                new Vector2(rect.xMin + bottomLeft, rect.yMax - bottomLeft),
                bottomLeft,
                90
            );
            return points;
        }

        public static List<Vector2> Inset(
            List<Vector2> points,
            UnityRect rect,
            IReadOnlyList<UiLength> inset
        )
        {
            points = Clip(
                points,
                point => point.y,
                rect.yMin + Resolve(inset[0], rect.height),
                true
            );
            points = Clip(
                points,
                point => point.x,
                rect.xMax - Resolve(inset[1], rect.width),
                false
            );
            points = Clip(
                points,
                point => point.y,
                rect.yMax - Resolve(inset[2], rect.height),
                false
            );
            return Clip(points, point => point.x, rect.xMin + Resolve(inset[3], rect.width), true);
        }

        public static List<Vector2> Clip(
            IReadOnlyList<Vector2> points,
            Func<Vector2, float> project,
            float boundary,
            bool greater
        )
        {
            var result = new List<Vector2>();
            if (points.Count == 0)
                return result;
            Vector2 previous = points[points.Count - 1];
            float previousDistance = project(previous) - boundary;
            bool previousInside = greater ? previousDistance >= 0 : previousDistance <= 0;
            foreach (Vector2 current in points)
            {
                float distance = project(current) - boundary;
                bool inside = greater ? distance >= 0 : distance <= 0;
                if (inside != previousInside)
                    result.Add(
                        Vector2.LerpUnclamped(
                            previous,
                            current,
                            previousDistance / (previousDistance - distance)
                        )
                    );
                if (inside)
                    result.Add(current);
                previous = current;
                previousDistance = distance;
                previousInside = inside;
            }
            return result;
        }

        private static float Fit(float length, float radii) => radii > 0 ? length / radii : 1;

        private static float Resolve(UiLength value, float reference) =>
            checked((float)(value.Pixels + value.Percentage * reference / 100));

        private static void Arc(List<Vector2> points, Vector2 center, float radius, float degrees)
        {
            if (radius == 0)
            {
                points.Add(center);
                return;
            }
            float step = 2 * Mathf.Acos(Mathf.Clamp(1 - 0.15f / radius, -1, 1));
            int segments = Mathf.Max(1, Mathf.CeilToInt(Mathf.PI / 2 / step));
            for (int index = 0; index <= segments; index++)
            {
                float angle = degrees * Mathf.Deg2Rad + Mathf.PI / 2 * index / segments;
                points.Add(center + radius * new Vector2(Mathf.Cos(angle), Mathf.Sin(angle)));
            }
        }
    }
}
