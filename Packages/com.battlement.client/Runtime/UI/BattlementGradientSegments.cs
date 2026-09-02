#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using UnityRect = UnityEngine.Rect;

namespace Battlement.UI
{
    internal static class BattlementGradientSegments
    {
        public static bool Paint(
            Painter2D painter,
            IReadOnlyList<Vector2> points,
            UnityRect rect,
            MotionGradient value,
            Func<MotionGradient, UnityRect, FillGradient> makeGradient
        )
        {
            if (value is not MotionGradient.Linear linear || linear.Stops.Count <= 8)
                return false;
            (Vector2 start, Vector2 end) = Line(rect, linear.Angle);
            Vector2 direction = end - start;
            float lengthSquared = direction.sqrMagnitude;
            float Project(Vector2 point) => Vector2.Dot(point - start, direction) / lengthSquared;
            for (int first = 0; first < linear.Stops.Count - 1; first += 7)
            {
                int last = Math.Min(first + 7, linear.Stops.Count - 1);
                var clipped = new List<Vector2>(points);
                if (first > 0)
                    clipped = Clip(clipped, Project, (float)linear.Stops[first].Position, true);
                if (last < linear.Stops.Count - 1)
                    clipped = Clip(clipped, Project, (float)linear.Stops[last].Position, false);
                if (clipped.Count < 3)
                    continue;
                var stops = new List<MotionGradientStop>();
                for (int index = first; index <= last; index++)
                    stops.Add(linear.Stops[index]);
                painter.fillGradient = makeGradient(
                    new MotionGradient.Linear(linear.Angle, stops),
                    rect
                );
                painter.BeginPath();
                painter.MoveTo(clipped[0]);
                for (int index = 1; index < clipped.Count; index++)
                    painter.LineTo(clipped[index]);
                painter.ClosePath();
                painter.Fill(FillRule.NonZero);
            }
            return true;
        }

        public static (Vector2 Start, Vector2 End) Line(UnityRect rect, double angle)
        {
            float radians = checked((float)angle) * Mathf.Deg2Rad;
            Vector2 axis = new(Mathf.Cos(radians), Mathf.Sin(radians));
            float radius = Mathf.Sqrt(rect.width * rect.width + rect.height * rect.height) / 2;
            return (rect.center - axis * radius, rect.center + axis * radius);
        }

        private static List<Vector2> Clip(
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
    }
}
