#nullable enable

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using UnityEngine;
using UnityColor = UnityEngine.Color;
using UnityRect = UnityEngine.Rect;

namespace Battlement.UI
{
    internal static class BattlementRadialGradientRaster
    {
        private const int ParallelPixelThreshold = 500_000;

        public static bool TryFill(
            int width,
            int height,
            UnityRect bounds,
            UnityRect rect,
            IReadOnlyList<Vector2> points,
            PaintFill fill,
            out Color32[] pixels
        )
        {
            if (fill is not PaintFill.Gradient { Value: Gradient.Radial radial })
            {
                pixels = Array.Empty<Color32>();
                return false;
            }

            var result = new Color32[width * height];
            var colors = new UnityColor[radial.Stops.Count];
            for (int index = 0; index < colors.Length; index++)
                colors[index] = ToUnity(radial.Stops[index].Color);

            void FillRow(int y, float[] crossings)
            {
                float sampleY = bounds.y + y + 0.5f;
                int crossingCount = 0;
                Vector2 previous = points[points.Count - 1];
                foreach (Vector2 next in points)
                {
                    if ((next.y > sampleY) != (previous.y > sampleY))
                        crossings[crossingCount++] =
                            (previous.x - next.x) * (sampleY - next.y) / (previous.y - next.y)
                            + next.x;
                    previous = next;
                }
                Array.Sort(crossings, 0, crossingCount);
                int destinationRow = (height - 1 - y) * width;
                for (int crossing = 0; crossing + 1 < crossingCount; crossing += 2)
                {
                    int start = Mathf.Clamp(
                        Mathf.CeilToInt(crossings[crossing] - bounds.x - 0.5f),
                        0,
                        width
                    );
                    int end = Mathf.Clamp(
                        Mathf.CeilToInt(crossings[crossing + 1] - bounds.x - 0.5f),
                        0,
                        width
                    );
                    for (int x = start; x < end; x++)
                    {
                        var point = new Vector2(bounds.x + x + 0.5f, sampleY);
                        float rx = (point.x - rect.x) / rect.width - (float)radial.Center[0];
                        float ry = (point.y - rect.y) / rect.height - (float)radial.Center[1];
                        rx /= Math.Max(0.000001f, (float)radial.Radius[0]);
                        ry /= Math.Max(0.000001f, (float)radial.Radius[1]);
                        result[destinationRow + x] = Sample(
                            radial.Stops,
                            colors,
                            Mathf.Sqrt(rx * rx + ry * ry)
                        );
                    }
                }
            }

            if (result.Length >= ParallelPixelThreshold)
                Parallel.For(
                    0,
                    height,
                    () => new float[points.Count],
                    (y, _, crossings) =>
                    {
                        FillRow(y, crossings);
                        return crossings;
                    },
                    _ => { }
                );
            else
            {
                var crossings = new float[points.Count];
                for (int y = 0; y < height; y++)
                    FillRow(y, crossings);
            }
            pixels = result;
            return true;
        }

        private static UnityColor Sample(
            IReadOnlyList<GradientStop> stops,
            IReadOnlyList<UnityColor> colors,
            float position
        )
        {
            if (position <= stops[0].Position)
                return colors[0];
            for (int index = 1; index < stops.Count; index++)
            {
                if (position > stops[index].Position)
                    continue;
                UnityColor a = colors[index - 1];
                UnityColor b = colors[index];
                float progress = Mathf.InverseLerp(
                    (float)stops[index - 1].Position,
                    (float)stops[index].Position,
                    position
                );
                float alpha = Mathf.Lerp(a.a, b.a, progress);
                UnityColor result =
                    UnityColor.Lerp(a * a.a, b * b.a, progress) / Math.Max(alpha, 0.000001f);
                result.a = alpha;
                return result;
            }
            return colors[^1];
        }

        private static UnityColor ToUnity(Color color) =>
            new((float)color.Red, (float)color.Green, (float)color.Blue, (float)color.Alpha);
    }
}
