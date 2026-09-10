#nullable enable

using System;
using System.Collections.Generic;
using System.Threading.Tasks;
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
        private BattlementPaintRasterCacheKey? cachedKey;
        private bool ownsTexture;
        private UnityRect bounds;
        private UnityColor[] pixels = Array.Empty<UnityColor>();
        private float[] alpha = Array.Empty<float>();
        private float[] workA = Array.Empty<float>();
        private float[] workB = Array.Empty<float>();
        private float[] workC = Array.Empty<float>();

        public void Draw(
            MeshGenerationContext context,
            UnityRect rect,
            IReadOnlyList<Vector2> points,
            PaintFill fill,
            IReadOnlyList<UiFilterFunction> filters,
            Texture2D? mask = null
        )
        {
            var key = new BattlementPaintRasterCacheKey(rect, points, fill, filters, mask);
            if (!key.Equals(cachedKey))
            {
                ReleaseOwnedTexture();
                if (BattlementPaintRasterCache.TryGet(key, out CachedPaint paint))
                {
                    texture = paint.Texture;
                    bounds = paint.Bounds;
                }
                else
                {
                    Rasterize(rect, points, fill, filters, mask);
                    ownsTexture = !BattlementPaintRasterCache.TryStore(key, texture!, bounds);
                }
                cachedKey = key;
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
            ReleaseOwnedTexture();
            texture = null;
            pixels = Array.Empty<UnityColor>();
            alpha = Array.Empty<float>();
            workA = Array.Empty<float>();
            workB = Array.Empty<float>();
            workC = Array.Empty<float>();
        }

        private void ReleaseOwnedTexture()
        {
            if (ownsTexture && texture != null)
                UnityEngine.Object.DestroyImmediate(texture);
            ownsTexture = false;
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
            int count = width * height;
            EnsureCapacity(ref pixels, count);
            Array.Clear(pixels, 0, count);
            FillPolygon(pixels, width, height, rect, points, fill);
            if (mask != null)
                ApplyMask(pixels, width, height, rect, mask);
            bool pristineAlpha = true;
            foreach (UiFilterFunction filter in filters)
            {
                if (filter is UiFilterFunction.Brightness brightness)
                {
                    for (int i = 0; i < count; i++)
                    {
                        pixels[i].r = Mathf.Min(1, pixels[i].r * (float)brightness.Value);
                        pixels[i].g = Mathf.Min(1, pixels[i].g * (float)brightness.Value);
                        pixels[i].b = Mathf.Min(1, pixels[i].b * (float)brightness.Value);
                    }
                }
                else if (filter is UiFilterFunction.DropShadow shadow)
                {
                    BattlementPaintShadowCacheKey? shadowKey = pristineAlpha
                        ? new BattlementPaintShadowCacheKey(
                            bounds,
                            rect,
                            points,
                            fill,
                            mask,
                            shadow.Value
                        )
                        : null;
                    Shadow(pixels, width, height, shadow.Value, shadowKey);
                    pristineAlpha = false;
                }
            }
            var upload = new Color32[count];
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
                upload[(height - 1 - y) * width + x] = pixels[y * width + x];
            texture = new Texture2D(width, height, TextureFormat.RGBA32, false)
            {
                name = "Battlement cached paint",
                hideFlags = HideFlags.HideAndDontSave,
                filterMode = FilterMode.Bilinear,
                wrapMode = TextureWrapMode.Clamp,
            };
            texture.SetPixels32(upload);
            texture.Apply(false, false);
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

        private void FillPolygon(
            UnityColor[] destination,
            int width,
            int height,
            UnityRect rect,
            IReadOnlyList<Vector2> points,
            PaintFill fill
        )
        {
            UnityColor solid = default;
            IReadOnlyList<GradientStop>? stops = null;
            Vector2 linearStart = default;
            Vector2 linearDelta = default;
            float linearSquared = 0;
            Gradient.Radial? radial = null;
            if (fill is PaintFill.Color color)
                solid = Color(color.Value);
            else if (((PaintFill.Gradient)fill).Value is Gradient.Radial radialFill)
            {
                radial = radialFill;
                stops = radialFill.Stops;
            }
            else
            {
                var linear = (Gradient.Linear)((PaintFill.Gradient)fill).Value;
                (Vector2 start, Vector2 end) = BattlementGradientSegments.Line(rect, linear.Angle);
                linearStart = start;
                linearDelta = end - linearStart;
                linearSquared = linearDelta.sqrMagnitude;
                stops = linear.Stops;
            }
            var crossings = new float[points.Count];
            for (int y = 0; y < height; y++)
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
                        if (stops is null)
                        {
                            destination[y * width + x] = solid;
                            continue;
                        }
                        Vector2 point = new(bounds.x + x + 0.5f, sampleY);
                        float position;
                        if (radial is not null)
                        {
                            float rx = (point.x - rect.x) / rect.width - (float)radial.Center[0];
                            float ry = (point.y - rect.y) / rect.height - (float)radial.Center[1];
                            rx /= Math.Max(0.000001f, (float)radial.Radius[0]);
                            ry /= Math.Max(0.000001f, (float)radial.Radius[1]);
                            position = Mathf.Sqrt(rx * rx + ry * ry);
                        }
                        else
                        {
                            position =
                                Vector2.Dot(point - linearStart, linearDelta) / linearSquared;
                        }
                        destination[y * width + x] = Sample(stops, position);
                    }
                }
            }
        }

        private static UnityColor Sample(IReadOnlyList<GradientStop> stops, float position)
        {
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

        private void Shadow(
            UnityColor[] pixels,
            int width,
            int height,
            Shadow shadow,
            BattlementPaintShadowCacheKey? cacheKey
        )
        {
            int count = width * height;
            EnsureCapacity(ref alpha, count);
            EnsureCapacity(ref workA, count);
            EnsureCapacity(ref workB, count);
            EnsureCapacity(ref workC, count);
            for (int i = 0; i < count; i++)
                alpha[i] = pixels[i].a;
            float[] silhouette;
            if (
                cacheKey is not null
                && BattlementPaintShadowCache.TryGet(cacheKey, out float[] cached)
            )
            {
                silhouette = cached;
            }
            else
            {
                silhouette = alpha;
                if (shadow.Spread != 0)
                {
                    Spread(
                        silhouette,
                        workA,
                        width,
                        height,
                        (int)Math.Ceiling(Math.Abs(shadow.Spread)),
                        shadow.Spread > 0
                    );
                    silhouette = workA;
                }
                if (shadow.Blur > 0)
                {
                    float[] destination = ReferenceEquals(silhouette, alpha) ? workA : alpha;
                    Gaussian(
                        silhouette,
                        destination,
                        workB,
                        workC,
                        width,
                        height,
                        (float)shadow.Blur
                    );
                    silhouette = destination;
                }
                if (cacheKey is not null)
                {
                    var retained = new float[count];
                    Array.Copy(silhouette, retained, count);
                    BattlementPaintShadowCache.TryStore(cacheKey, retained);
                    silhouette = retained;
                }
            }
            UnityColor tint = Color(shadow.Color);
            for (int y = 0; y < height; y++)
            for (int x = 0; x < width; x++)
            {
                int i = y * width + x;
                float shadowAlpha = SampleAlpha(
                    silhouette,
                    width,
                    height,
                    x - (float)shadow.X,
                    y - (float)shadow.Y
                );
                float a = shadowAlpha * tint.a * (1 - pixels[i].a);
                float total = pixels[i].a + a;
                UnityColor result =
                    (pixels[i] * pixels[i].a + tint * a) / Math.Max(total, 0.000001f);
                result.a = total;
                pixels[i] = result;
            }
        }

        private static void Gaussian(
            float[] source,
            float[] result,
            float[] first,
            float[] second,
            int width,
            int height,
            float sigma
        )
        {
            int[] widths = BoxWidths(sigma);
            BoxBlur(source, first, width, height, widths[0] / 2, true);
            BoxBlur(first, second, width, height, widths[0] / 2, false);
            BoxBlur(second, first, width, height, widths[1] / 2, true);
            BoxBlur(first, second, width, height, widths[1] / 2, false);
            BoxBlur(second, first, width, height, widths[2] / 2, true);
            BoxBlur(first, result, width, height, widths[2] / 2, false);
        }

        private static int[] BoxWidths(float sigma)
        {
            const int count = 3;
            float ideal = Mathf.Sqrt(12 * sigma * sigma / count + 1);
            int lower = Mathf.FloorToInt(ideal);
            if (lower % 2 == 0)
                lower--;
            int upper = lower + 2;
            int lowerCount = Mathf.RoundToInt(
                (12 * sigma * sigma - count * lower * lower - 4 * count * lower - 3 * count)
                    / (-4f * lower - 4)
            );
            return new[]
            {
                lowerCount > 0 ? lower : upper,
                lowerCount > 1 ? lower : upper,
                lowerCount > 2 ? lower : upper,
            };
        }

        private static void BoxBlur(
            float[] source,
            float[] result,
            int width,
            int height,
            int radius,
            bool horizontal
        )
        {
            int lines = horizontal ? height : width;
            int length = horizontal ? width : height;
            float scale = 1f / (radius * 2 + 1);
            Parallel.For(
                0,
                lines,
                line =>
                {
                    int Index(int position) =>
                        horizontal ? line * width + position : position * width + line;
                    float sum = 0;
                    for (int position = 0; position <= radius && position < length; position++)
                        sum += source[Index(position)];
                    for (int position = 0; position < length; position++)
                    {
                        result[Index(position)] = sum * scale;
                        int leaving = position - radius;
                        if (leaving >= 0)
                            sum -= source[Index(leaving)];
                        int entering = position + radius + 1;
                        if (entering < length)
                            sum += source[Index(entering)];
                    }
                }
            );
        }

        private static void Spread(
            float[] source,
            float[] result,
            int width,
            int height,
            int radius,
            bool grow
        )
        {
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
        }

        private static void EnsureCapacity<T>(ref T[] buffer, int count)
        {
            if (buffer.Length < count)
                buffer = new T[count];
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
