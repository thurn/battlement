#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using UnityColor = UnityEngine.Color;
using UnityRect = UnityEngine.Rect;

namespace Battlement.UI
{
    internal sealed class BattlementAdvancedPaint : IDisposable
    {
        private readonly Dictionary<MotionProperty, MotionValue> values = new();
        private readonly VisualElement target;
        private IBattlementUiAssetLookup? assets;
        private IBattlementUiAssetLease? backgroundLease;
        private IBattlementUiAssetLease? maskLease;
        private IBattlementUiAssetLease? materialLease;
        private StyleColor authoredBackground;
        public bool HasStaticFill { get; private set; }

        public BattlementAdvancedPaint(VisualElement target)
        {
            this.target = target;
            target.generateVisualContent += Paint;
        }

        public MotionValue Read(MotionProperty property, MotionValue fallback) =>
            values.TryGetValue(property, out MotionValue value) ? value : fallback;

        public void Configure(IBattlementUiAssetLookup? lookup) => assets = lookup;

        public void ReplaceStatic(
            IReadOnlyList<MotionPropertyValue> previous,
            IReadOnlyList<MotionPropertyValue> next
        )
        {
            bool fill = false;
            foreach (MotionPropertyValue value in next)
                if (
                    value.Property
                    is MotionProperty.BackgroundColor
                        or MotionProperty.BackgroundGradient
                )
                    fill = true;
            if (fill && !HasStaticFill)
                authoredBackground = target.style.backgroundColor;
            if (!fill && HasStaticFill)
                target.style.backgroundColor = authoredBackground;
            HasStaticFill = fill;
            if (fill)
                target.style.backgroundColor = UnityColor.clear;
            foreach (MotionPropertyValue value in previous)
                if (BattlementStaticPaint.Owns(value.Property))
                    values.Remove(value.Property);
            foreach (MotionPropertyValue value in next)
                if (BattlementStaticPaint.Owns(value.Property))
                    Write(value.Property, value.Value);
            target.MarkDirtyRepaint();
        }

        public void CommitAuthoredStyle(UiStyle style)
        {
            if (!HasStaticFill || style.BackgroundColor.IsUnset)
                return;
            authoredBackground = target.style.backgroundColor;
            target.style.backgroundColor = UnityColor.clear;
        }

        public void Write(MotionProperty property, MotionValue value)
        {
            if (EmptyPaint(value))
                values.Remove(property);
            else
                values[property] = value;
            if (property == MotionProperty.BackgroundImage)
                WriteTexture(value, false);
            if (property == MotionProperty.Mask)
                WriteTexture(value, true);
            if (property == MotionProperty.UnityMaterial)
                WriteMaterial(value);
            if (property == MotionProperty.Filter && value is MotionValue.FilterList filters)
                target.style.filter = NativeFilters(filters.Value);
            if (property == MotionProperty.ClipInset)
                target.style.overflow = Overflow.Hidden;
            target.MarkDirtyRepaint();
        }

        public void Dispose()
        {
            target.generateVisualContent -= Paint;
            if (HasStaticFill)
                target.style.backgroundColor = authoredBackground;
            backgroundLease?.Dispose();
            maskLease?.Dispose();
            materialLease?.Dispose();
        }

        private void Paint(MeshGenerationContext context)
        {
            if (!HasPaint())
                return;
            UnityRect rect = new(0, 0, target.layout.width, target.layout.height);
            if (rect.width <= 0 || rect.height <= 0)
                return;
            IReadOnlyList<Vector2> points = Geometry(rect);
            if (points.Count < 3)
                return;
            Painter2D painter = context.painter2D;
            DrawOuterShadows(painter, points);
            if (maskLease?.Value is Texture2D mask)
                painter.fillTexture = mask;
            if (!PaintGradientSegments(painter, points, rect))
            {
                painter.BeginPath();
                Path(painter, points);
                ApplyFill(painter, rect);
                painter.Fill(FillRule.NonZero);
            }
            DrawInsetShadows(painter, points);
        }

        private bool PaintGradientSegments(
            Painter2D painter,
            IReadOnlyList<Vector2> points,
            UnityRect rect
        )
        {
            if (!values.TryGetValue(MotionProperty.BackgroundGradient, out MotionValue value))
                return false;
            return value is MotionValue.Gradient gradient
                && BattlementGradientSegments.Paint(
                    painter,
                    points,
                    rect,
                    gradient.Value,
                    FillGradient
                );
        }

        private static bool EmptyPaint(MotionValue value) =>
            value switch
            {
                MotionValue.ClipPolygon polygon => polygon.Value.Count == 0,
                MotionValue.Gradient { Value: MotionGradient.Linear linear } => linear.Stops.Count
                    == 0,
                MotionValue.Gradient { Value: MotionGradient.Radial radial } => radial.Stops.Count
                    == 0,
                MotionValue.ShadowList shadows => shadows.Value.Count == 0,
                _ => false,
            };

        private bool HasPaint() =>
            values.ContainsKey(MotionProperty.BackgroundColor)
            || values.ContainsKey(MotionProperty.BackgroundGradient)
            || values.ContainsKey(MotionProperty.BoxShadow)
            || values.ContainsKey(MotionProperty.ClipInset)
            || values.ContainsKey(MotionProperty.ClipPolygon)
            || values.ContainsKey(MotionProperty.RotateX)
            || values.ContainsKey(MotionProperty.RotateY)
            || values.ContainsKey(MotionProperty.SkewX)
            || values.ContainsKey(MotionProperty.SkewY)
            || values.ContainsKey(MotionProperty.TransformList)
            || values.ContainsKey(MotionProperty.Mask);

        private void WriteTexture(MotionValue value, bool mask)
        {
            string? address = Address(value);
            if (address is null)
            {
                ReplaceLease(mask, null);
                if (!mask)
                    target.style.backgroundImage = StyleKeyword.None;
                return;
            }
            if (assets is null)
                throw Failure("No UI asset lookup is configured for motion paint.");
            IBattlementUiAssetLease lease = assets.Acquire(
                new PreparedAsset.Texture(new TextureAddress(address))
            );
            if (lease.Value is not Texture2D texture)
            {
                lease.Dispose();
                throw Failure($"Prepared motion texture '{address}' is not a Texture2D.");
            }
            ReplaceLease(mask, lease);
            if (!mask)
                target.style.backgroundImage = Background.FromTexture2D(texture);
        }

        private void WriteMaterial(MotionValue value)
        {
            string? address = Address(value);
            if (address is null)
            {
                materialLease?.Dispose();
                materialLease = null;
                target.style.unityMaterial = StyleKeyword.None;
                return;
            }
            if (assets is null)
                throw Failure("No UI asset lookup is configured for motion shader paint.");
            IBattlementUiAssetLease lease = assets.Acquire(
                new PreparedAsset.Material(new MaterialAddress(address))
            );
            if (lease.Value is not Material material)
            {
                lease.Dispose();
                throw Failure($"Prepared motion material '{address}' is not a Material.");
            }
            materialLease?.Dispose();
            materialLease = lease;
            target.style.unityMaterial = material;
        }

        private void ReplaceLease(bool mask, IBattlementUiAssetLease? replacement)
        {
            IBattlementUiAssetLease? previous = mask ? maskLease : backgroundLease;
            if (mask)
                maskLease = replacement;
            else
                backgroundLease = replacement;
            previous?.Dispose();
        }

        private static string? Address(MotionValue value)
        {
            if (value is not MotionValue.Discrete discrete)
                throw new InvalidOperationException("A motion asset value must be discrete.");
            return discrete.Value.Type == Newtonsoft.Json.Linq.JTokenType.Null
                ? null
                : discrete.Value.ToObject<string>();
        }

        private IReadOnlyList<Vector2> Geometry(UnityRect rect)
        {
            List<Vector2> points =
                values.TryGetValue(MotionProperty.ClipPolygon, out MotionValue polygonValue)
                && polygonValue is MotionValue.ClipPolygon polygon
                && polygon.Value.Count >= 3
                    ? Polygon(rect, polygon.Value)
                    : Inset(rect);
            Transform(points, rect.center);
            return points;
        }

        private List<Vector2> Inset(UnityRect rect)
        {
            List<Vector2> points = BattlementPaintContour.RoundedBox(rect, target.resolvedStyle);
            return
                values.TryGetValue(MotionProperty.ClipInset, out MotionValue value)
                && value is MotionValue.ClipInset inset
                && inset.Value.Count == 4
                ? BattlementPaintContour.Inset(points, rect, inset.Value)
                : points;
        }

        private static List<Vector2> Polygon(
            UnityRect rect,
            IReadOnlyList<IReadOnlyList<MotionLength>> source
        )
        {
            var points = new List<Vector2>(source.Count);
            foreach (IReadOnlyList<MotionLength> point in source)
            {
                if (point.Count != 2)
                    throw new InvalidOperationException("A motion polygon point needs two axes.");
                points.Add(
                    new Vector2(
                        rect.xMin + Resolve(point[0], rect.width),
                        rect.yMin + Resolve(point[1], rect.height)
                    )
                );
            }
            return points;
        }

        private void Transform(List<Vector2> points, Vector2 origin)
        {
            float rotateX = Angle(MotionProperty.RotateX);
            float rotateY = Angle(MotionProperty.RotateY);
            float cosineX = Mathf.Cos(rotateX * Mathf.Deg2Rad);
            float cosineY = Mathf.Cos(rotateY * Mathf.Deg2Rad);
            float skewX = Mathf.Tan(Angle(MotionProperty.SkewX) * Mathf.Deg2Rad);
            float skewY = Mathf.Tan(Angle(MotionProperty.SkewY) * Mathf.Deg2Rad);
            for (int index = 0; index < points.Count; index++)
            {
                Vector2 offset = points[index] - origin;
                points[index] =
                    origin
                    + new Vector2(
                        offset.x * cosineY + skewX * offset.y,
                        offset.y * cosineX + skewY * offset.x
                    );
            }
            if (
                !values.TryGetValue(MotionProperty.TransformList, out MotionValue value)
                || value is not MotionValue.TransformList transforms
            )
                return;
            foreach (MotionTransform transform in transforms.Value)
                ApplyTransform(points, origin, transform);
        }

        private static void ApplyTransform(
            List<Vector2> points,
            Vector2 origin,
            MotionTransform transform
        )
        {
            for (int index = 0; index < points.Count; index++)
            {
                Vector2 offset = points[index] - origin;
                points[index] = transform switch
                {
                    MotionTransform.Translate translate => points[index]
                        + Translation(translate.Value),
                    MotionTransform.Rotate rotate => origin
                        + Rotate(offset, checked((float)rotate.Value[2])),
                    MotionTransform.Skew skew => origin
                        + new Vector2(
                            offset.x
                                + Mathf.Tan(checked((float)skew.Value[0]) * Mathf.Deg2Rad)
                                    * offset.y,
                            offset.y
                                + Mathf.Tan(checked((float)skew.Value[1]) * Mathf.Deg2Rad)
                                    * offset.x
                        ),
                    MotionTransform.Scale scale => origin
                        + new Vector2(
                            offset.x * checked((float)scale.Value[0]),
                            offset.y * checked((float)scale.Value[1])
                        ),
                    _ => throw new InvalidOperationException("Unknown motion transform."),
                };
            }
        }

        private void ApplyFill(Painter2D painter, UnityRect rect)
        {
            if (
                values.TryGetValue(MotionProperty.BackgroundGradient, out MotionValue value)
                && value is MotionValue.Gradient gradient
            )
            {
                painter.fillGradient = FillGradient(gradient.Value, rect);
                return;
            }
            painter.fillColor =
                values.TryGetValue(MotionProperty.BackgroundColor, out MotionValue color)
                && color is MotionValue.Color solid
                    ? ToUnityColor(solid.Value)
                    : target.resolvedStyle.backgroundColor;
        }

        private void DrawOuterShadows(Painter2D painter, IReadOnlyList<Vector2> points)
        {
            foreach (MotionShadow shadow in Shadows(false))
            {
                painter.fillGradient = default;
                painter.fillColor = ToUnityColor(shadow.Color);
                painter.BeginPath();
                Path(painter, Offset(points, shadow));
                painter.Fill(FillRule.NonZero);
            }
        }

        private void DrawInsetShadows(Painter2D painter, IReadOnlyList<Vector2> points)
        {
            foreach (MotionShadow shadow in Shadows(true))
            {
                painter.strokeColor = ToUnityColor(shadow.Color);
                painter.lineWidth = checked((float)Math.Max(1, shadow.Blur + shadow.Spread));
                painter.BeginPath();
                Path(painter, Offset(points, shadow));
                painter.Stroke();
            }
        }

        private IEnumerable<MotionShadow> Shadows(bool inset)
        {
            if (
                values.TryGetValue(MotionProperty.BoxShadow, out MotionValue value)
                && value is MotionValue.ShadowList shadows
            )
                foreach (MotionShadow shadow in shadows.Value)
                    if (shadow.Inset == inset)
                        yield return shadow;
        }

        private static IReadOnlyList<Vector2> Offset(
            IReadOnlyList<Vector2> points,
            MotionShadow shadow
        )
        {
            float spread = checked((float)shadow.Spread);
            Vector2 center = Vector2.zero;
            foreach (Vector2 point in points)
                center += point;
            center /= points.Count;
            var result = new List<Vector2>(points.Count);
            foreach (Vector2 point in points)
            {
                Vector2 radial = (point - center).normalized * spread;
                result.Add(
                    point + radial + new Vector2(checked((float)shadow.X), checked((float)shadow.Y))
                );
            }
            return result;
        }

        private static void Path(Painter2D painter, IReadOnlyList<Vector2> points)
        {
            painter.MoveTo(points[0]);
            for (int index = 1; index < points.Count; index++)
                painter.LineTo(points[index]);
            painter.ClosePath();
        }

        private float Angle(MotionProperty property) =>
            values.TryGetValue(property, out MotionValue value) && value is MotionValue.Angle angle
                ? checked((float)angle.Value)
                : 0;

        private static StyleList<FilterFunction> NativeFilters(IReadOnlyList<MotionFilter> filters)
        {
            var result = new List<FilterFunction>();
            foreach (MotionFilter filter in filters)
            {
                FilterFunction? converted = filter switch
                {
                    MotionFilter.Blur blur => Function(FilterFunctionType.Blur, blur.Value),
                    MotionFilter.Contrast contrast => Function(
                        FilterFunctionType.Contrast,
                        contrast.Value
                    ),
                    MotionFilter.HueRotate hue => Function(FilterFunctionType.HueRotate, hue.Value),
                    MotionFilter.Opacity opacity => Function(
                        FilterFunctionType.Opacity,
                        opacity.Value
                    ),
                    _ => null,
                };
                if (converted is not null)
                    result.Add(converted.Value);
            }
            return new StyleList<FilterFunction>(result);
        }

        private static FilterFunction Function(FilterFunctionType type, double value)
        {
            var result = new FilterFunction(type);
            result.AddParameter(new FilterParameter(checked((float)value)));
            return result;
        }

        private static FillGradient FillGradient(MotionGradient value, UnityRect rect)
        {
            Gradient gradient = Gradient(value);
            if (value is MotionGradient.Linear linear)
            {
                (Vector2 start, Vector2 end) = BattlementGradientSegments.Line(rect, linear.Angle);
                return UnityEngine.UIElements.FillGradient.MakeLinearGradient(
                    gradient,
                    start,
                    end,
                    AddressMode.Clamp
                );
            }
            if (value is MotionGradient.Radial radial)
            {
                Vector2 center = new(
                    rect.xMin + checked((float)radial.Center[0]) * rect.width,
                    rect.yMin + checked((float)radial.Center[1]) * rect.height
                );
                float radius = checked((float)radial.Radius[0]) * rect.width;
                return UnityEngine.UIElements.FillGradient.MakeRadialGradient(
                    gradient,
                    center,
                    radius,
                    center,
                    AddressMode.Clamp
                );
            }
            throw new InvalidOperationException("Unknown motion gradient.");
        }

        private static Gradient Gradient(MotionGradient value)
        {
            IReadOnlyList<MotionGradientStop> stops = value switch
            {
                MotionGradient.Linear linear => linear.Stops,
                MotionGradient.Radial radial => radial.Stops,
                _ => throw new InvalidOperationException("Unknown motion gradient."),
            };
            var colors = new GradientColorKey[stops.Count];
            var alpha = new GradientAlphaKey[stops.Count];
            for (int index = 0; index < stops.Count; index++)
            {
                MotionGradientStop stop = stops[index];
                UnityColor color = ToUnityColor(stop.Color);
                float time = checked((float)stop.Position);
                colors[index] = new GradientColorKey(color, time);
                alpha[index] = new GradientAlphaKey(color.a, time);
            }
            var gradient = new Gradient();
            gradient.SetKeys(colors, alpha);
            return gradient;
        }

        private static Vector2 Translation(IReadOnlyList<MotionLength> value) =>
            new(checked((float)value[0].Px), checked((float)value[1].Px));

        private static Vector2 Rotate(Vector2 value, float degrees)
        {
            float radians = degrees * Mathf.Deg2Rad;
            float sine = Mathf.Sin(radians);
            float cosine = Mathf.Cos(radians);
            return new Vector2(
                value.x * cosine - value.y * sine,
                value.x * sine + value.y * cosine
            );
        }

        private static float Resolve(MotionLength value, float reference) =>
            checked((float)(value.Px + value.Percent * reference / 100));

        private static UnityColor ToUnityColor(MotionColor value) =>
            new(
                checked((float)value.Red),
                checked((float)value.Green),
                checked((float)value.Blue),
                checked((float)value.Alpha)
            );

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.AssetNotPrepared, message);
    }
}
