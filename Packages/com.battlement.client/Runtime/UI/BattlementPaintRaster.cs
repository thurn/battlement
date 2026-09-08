#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
using UnityEngine.UIElements;
using UnityColor = UnityEngine.Color;
using UnityRect = UnityEngine.Rect;

namespace Battlement.UI
{
    /// <summary>Caches alpha-correct gradient artwork and its ordered silhouette filters.</summary>
    internal sealed class BattlementPaintRaster : IDisposable
    {
        private Texture2D? texture;
        private PaintFill? cachedFill;
        private Texture2D? cachedMask;
        private IReadOnlyList<UiFilterFunction>? cachedFilters;
        private Vector2[] cachedPoints = Array.Empty<Vector2>();
        private UnityRect cachedRect;
        private UnityRect bounds;

        public void Draw(
            MeshGenerationContext context,
            UnityRect rect,
            IReadOnlyList<Vector2> points,
            PaintFill fill,
            IReadOnlyList<UiFilterFunction> filters,
            Texture2D? mask = null
        )
        {
            bool sameFill = texture != null && Equals(cachedFill, fill) && cachedRect == rect;
            bool sameGeometry =
                ReferenceEquals(cachedFilters, filters) && cachedPoints.SequenceEqual(points);
            if (!sameFill || !sameGeometry || cachedMask != mask)
            {
                Dispose();
                cachedFill = fill;
                cachedMask = mask;
                cachedFilters = filters;
                cachedRect = rect;
                cachedPoints = points.ToArray();
                Rasterize(rect, points, fill, filters, mask);
            }
            MeshWriteData mesh = context.Allocate(4, 6, texture);
            mesh.SetNextVertex(
                new Vertex
                {
                    position = new UnityEngine.Vector3(bounds.xMin, bounds.yMin, Vertex.nearZ),
                    tint = UnityColor.white,
                    uv = new Vector2(0, 1),
                }
            );
            mesh.SetNextVertex(
                new Vertex
                {
                    position = new UnityEngine.Vector3(bounds.xMax, bounds.yMin, Vertex.nearZ),
                    tint = UnityColor.white,
                    uv = new Vector2(1, 1),
                }
            );
            mesh.SetNextVertex(
                new Vertex
                {
                    position = new UnityEngine.Vector3(bounds.xMax, bounds.yMax, Vertex.nearZ),
                    tint = UnityColor.white,
                    uv = new Vector2(1, 0),
                }
            );
            mesh.SetNextVertex(
                new Vertex
                {
                    position = new UnityEngine.Vector3(bounds.xMin, bounds.yMax, Vertex.nearZ),
                    tint = UnityColor.white,
                    uv = new Vector2(0, 0),
                }
            );
            foreach (ushort index in new ushort[] { 0, 1, 2, 2, 3, 0 })
                mesh.SetNextIndex(index);
        }

        public void Dispose()
        {
            if (texture != null)
                UnityEngine.Object.DestroyImmediate(texture);
            texture = null;
        }

        private void Rasterize(
            UnityRect rect,
            IReadOnlyList<Vector2> points,
            PaintFill fill,
            IReadOnlyList<UiFilterFunction> filters,
            Texture2D? mask
        )
        {
            float padding = 1;
            foreach (UiFilterFunction filter in filters)
                if (filter is UiFilterFunction.DropShadow shadow)
                    padding += (float)(
                        3 * shadow.Value.Blur
                        + Math.Abs(shadow.Value.Spread)
                        + Math.Max(Math.Abs(shadow.Value.X), Math.Abs(shadow.Value.Y))
                    );
            padding = Mathf.Ceil(padding);
            bounds = new UnityRect(
                rect.x - padding,
                rect.y - padding,
                Mathf.Ceil(rect.width + padding * 2),
                Mathf.Ceil(rect.height + padding * 2)
            );
            int width = Mathf.CeilToInt(bounds.width);
            int height = Mathf.CeilToInt(bounds.height);
            if (width > 8192 || height > 8192)
                throw new InvalidOperationException(
                    "Filtered paint exceeds the 8192-pixel surface limit."
                );
            var pixels = new UnityColor[width * height];
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            {
                Vector2 point = new(bounds.x + x + 0.5f, bounds.y + y + 0.5f);
                if (Contains(points, point))
                    pixels[y * width + x] = Sample(fill, rect, point);
            }
            if (mask != null)
                ApplyMask(pixels, width, height, rect, mask);
            foreach (UiFilterFunction filter in filters)
            {
                if (filter is UiFilterFunction.Brightness brightness)
                {
                    for (int i = 0; i < pixels.Length; i++)
                    {
                        pixels[i].r = Mathf.Min(1, pixels[i].r * (float)brightness.Value);
                        pixels[i].g = Mathf.Min(1, pixels[i].g * (float)brightness.Value);
                        pixels[i].b = Mathf.Min(1, pixels[i].b * (float)brightness.Value);
                    }
                }
                else if (filter is UiFilterFunction.DropShadow shadow)
                    Shadow(pixels, width, height, shadow.Value);
            }
            var upload = new Color32[pixels.Length];
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
                upload[(height - 1 - y) * width + x] = pixels[y * width + x];
            texture = new Texture2D(width, height, TextureFormat.RGBA32, false)
            {
                name = "Battlement owned paint",
                hideFlags = HideFlags.HideAndDontSave,
                filterMode = FilterMode.Bilinear,
                wrapMode = TextureWrapMode.Clamp,
            };
            texture.SetPixels32(upload);
            texture.Apply(false, true);
        }

        private void ApplyMask(
            UnityColor[] pixels,
            int width,
            int height,
            UnityRect rect,
            Texture2D mask
        )
        {
            Texture2D readable = mask;
            if (!mask.isReadable)
            {
                RenderTexture previous = RenderTexture.active;
                RenderTexture copy = RenderTexture.GetTemporary(mask.width, mask.height, 0);
                try
                {
                    Graphics.Blit(mask, copy);
                    RenderTexture.active = copy;
                    readable = new Texture2D(mask.width, mask.height, TextureFormat.RGBA32, false);
                    readable.ReadPixels(new UnityRect(0, 0, mask.width, mask.height), 0, 0);
                    readable.Apply();
                }
                finally
                {
                    RenderTexture.active = previous;
                    RenderTexture.ReleaseTemporary(copy);
                }
            }
            try
            {
                for (int y = 0; y < height; y++)
                for (int x = 0; x < width; x++)
                {
                    float u = (bounds.x + x + 0.5f - rect.x) / rect.width;
                    float v = 1 - (bounds.y + y + 0.5f - rect.y) / rect.height;
                    pixels[y * width + x] *= readable.GetPixelBilinear(u, v);
                }
            }
            finally
            {
                if (readable != mask)
                    UnityEngine.Object.DestroyImmediate(readable);
            }
        }

        private static UnityColor Sample(PaintFill fill, UnityRect rect, Vector2 point)
        {
            if (fill is PaintFill.Color color)
                return Color(color.Value);
            Gradient gradient = ((PaintFill.Gradient)fill).Value;
            float position;
            IReadOnlyList<GradientStop> stops;
            if (gradient is Gradient.Radial radial)
            {
                float x = (point.x - rect.x) / rect.width - (float)radial.Center[0];
                float y = (point.y - rect.y) / rect.height - (float)radial.Center[1];
                x /= Math.Max(0.000001f, (float)radial.Radius[0]);
                y /= Math.Max(0.000001f, (float)radial.Radius[1]);
                position = Mathf.Sqrt(x * x + y * y);
                stops = radial.Stops;
            }
            else
            {
                var linear = (Gradient.Linear)gradient;
                (Vector2 start, Vector2 end) = BattlementGradientSegments.Line(rect, linear.Angle);
                position = Vector2.Dot(point - start, end - start) / (end - start).sqrMagnitude;
                stops = linear.Stops;
            }
            if (position <= stops[0].Position)
                return Color(stops[0].Color);
            for (int i = 1; i < stops.Count; i++)
            {
                if (position > stops[i].Position)
                    continue;
                UnityColor a = Color(stops[i - 1].Color);
                UnityColor b = Color(stops[i].Color);
                float t = Mathf.InverseLerp(
                    (float)stops[i - 1].Position,
                    (float)stops[i].Position,
                    position
                );
                float alpha = Mathf.Lerp(a.a, b.a, t);
                UnityColor result =
                    UnityColor.Lerp(a * a.a, b * b.a, t) / Math.Max(alpha, 0.000001f);
                result.a = alpha;
                return result;
            }
            return Color(stops[stops.Count - 1].Color);
        }

        private static void Shadow(UnityColor[] pixels, int width, int height, Shadow shadow)
        {
            var alpha = new float[pixels.Length];
            for (int i = 0; i < pixels.Length; i++)
                alpha[i] = pixels[i].a;
            if (shadow.Spread != 0)
                alpha = Spread(
                    alpha,
                    width,
                    height,
                    (int)Math.Ceiling(Math.Abs(shadow.Spread)),
                    shadow.Spread > 0
                );
            if (shadow.Blur > 0)
            {
                float[] kernel = Kernel((float)shadow.Blur);
                alpha = Blur(
                    Blur(alpha, width, height, kernel, true),
                    width,
                    height,
                    kernel,
                    false
                );
            }
            UnityColor tint = Color(shadow.Color);
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            {
                int i = y * width + x;
                float silhouette = SampleAlpha(
                    alpha,
                    width,
                    height,
                    x - (float)shadow.X,
                    y - (float)shadow.Y
                );
                float a = silhouette * tint.a * (1 - pixels[i].a);
                float total = pixels[i].a + a;
                UnityColor result =
                    (pixels[i] * pixels[i].a + tint * a) / Math.Max(total, 0.000001f);
                result.a = total;
                pixels[i] = result;
            }
        }

        private static float[] Kernel(float sigma)
        {
            int radius = Mathf.CeilToInt(3 * sigma);
            var kernel = new float[radius * 2 + 1];
            float sum = 0;
            for (int i = -radius; i <= radius; i++)
                sum += kernel[i + radius] = Mathf.Exp(-i * i / (2 * sigma * sigma));
            for (int i = 0; i < kernel.Length; i++)
                kernel[i] /= sum;
            return kernel;
        }

        private static float[] Blur(
            float[] source,
            int width,
            int height,
            float[] kernel,
            bool horizontal
        )
        {
            var result = new float[source.Length];
            int radius = kernel.Length / 2;
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            for (int k = -radius; k <= radius; k++)
            {
                int sx = horizontal ? x + k : x;
                int sy = horizontal ? y : y + k;
                if (InBounds(sx, sy, width, height))
                    result[y * width + x] += source[sy * width + sx] * kernel[k + radius];
            }
            return result;
        }

        private static float[] Spread(float[] source, int width, int height, int radius, bool grow)
        {
            var result = new float[source.Length];
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            {
                float value = grow ? 0 : 1;
                for (int dy = -radius; dy <= radius; dy++)
                for (int dx = -radius; dx <= radius; dx++)
                {
                    if (dx * dx + dy * dy > radius * radius)
                        continue;
                    int sx = x + dx;
                    int sy = y + dy;
                    float sample = !InBounds(sx, sy, width, height) ? 0 : source[sy * width + sx];
                    value = grow ? Math.Max(value, sample) : Math.Min(value, sample);
                }
                result[y * width + x] = value;
            }
            return result;
        }

        private static bool Contains(IReadOnlyList<Vector2> points, Vector2 point)
        {
            bool inside = false;
            Vector2 previous = points[points.Count - 1];
            foreach (Vector2 next in points)
            {
                if ((next.y > point.y) != (previous.y > point.y))
                    if (
                        point.x
                        < (previous.x - next.x) * (point.y - next.y) / (previous.y - next.y)
                            + next.x
                    )
                        inside = !inside;
                previous = next;
            }
            return inside;
        }

        private static bool InBounds(int x, int y, int width, int height) =>
            (uint)x < (uint)width && (uint)y < (uint)height;

        private static float SampleAlpha(float[] pixels, int width, int height, float x, float y)
        {
            int left = Mathf.FloorToInt(x);
            int top = Mathf.FloorToInt(y);
            float At(int sx, int sy) =>
                InBounds(sx, sy, width, height) ? pixels[sy * width + sx] : 0;
            return Mathf.Lerp(
                Mathf.Lerp(At(left, top), At(left + 1, top), x - left),
                Mathf.Lerp(At(left, top + 1), At(left + 1, top + 1), x - left),
                y - top
            );
        }

        private static UnityColor Color(Color color) =>
            new((float)color.Red, (float)color.Green, (float)color.Blue, (float)color.Alpha);
    }
}
