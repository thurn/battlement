#nullable enable

using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using UnityEngine;
using UnityEngine.UIElements;
using UnityColor = UnityEngine.Color;
using UnityGradient = UnityEngine.Gradient;
using UnityRect = UnityEngine.Rect;

namespace Battlement.UI
{
    internal sealed class BattlementAdvancedPaint : IDisposable
    {
        private static readonly ConditionalWeakTable<VisualElement, BattlementAdvancedPaint> All =
            new();
        private readonly Dictionary<MotionProperty, MotionValue> motionValues = new();
        private readonly Dictionary<MotionProperty, MotionValue> staticValues = new();
        private IReadOnlyList<PaintLayer> staticLayers = Array.Empty<PaintLayer>();
        private readonly VisualElement target;
        private IBattlementUiAssetLookup? assets;
        private IBattlementUiAssetLease? backgroundLease;
        private IBattlementUiAssetLease? maskLease;
        private IBattlementUiAssetLease? materialLease;
        private StyleColor authoredBackground;
        public bool HasStaticFill { get; private set; }
        public bool HasStaticPaint => staticValues.Count > 0 || staticLayers.Count > 0;

        public static BattlementAdvancedPaint For(VisualElement target) =>
            All.GetValue(target, element => new BattlementAdvancedPaint(element));

        public static bool TryGet(VisualElement target, out BattlementAdvancedPaint paint) =>
            All.TryGetValue(target, out paint);

        public static void Release(VisualElement target)
        {
            if (All.TryGetValue(target, out BattlementAdvancedPaint paint))
                paint.Dispose();
            All.Remove(target);
        }

        public BattlementAdvancedPaint(VisualElement target)
        {
            this.target = target;
            target.generateVisualContent += Paint;
        }

        public MotionValue Read(MotionProperty property, MotionValue fallback) =>
            TryValue(property, out MotionValue value) ? value : fallback;

        public MotionValue ReadStatic(MotionProperty property, MotionValue fallback) =>
            staticValues.TryGetValue(property, out MotionValue value) ? value : fallback;

        public void Configure(IBattlementUiAssetLookup? lookup) => assets = lookup;

        public void ReplaceStatic(PaintStyle? next)
        {
            staticLayers = next?.Layers ?? Array.Empty<PaintLayer>();
            bool fill = next?.Background is not null;
            if (fill && !HasStaticFill)
                authoredBackground = target.style.backgroundColor;
            if (!fill && HasStaticFill)
                target.style.backgroundColor = authoredBackground;
            HasStaticFill = fill;
            if (fill)
                target.style.backgroundColor = UnityColor.clear;
            staticValues.Clear();
            if (next?.Background is PaintFill.Color color)
                staticValues[MotionProperty.BackgroundColor] = new MotionValue.Color(color.Value);
            if (next?.Background is PaintFill.Gradient gradient)
                staticValues[MotionProperty.BackgroundGradient] = new MotionValue.Gradient(
                    gradient.Value
                );
            if (next?.PaintFilter is not null)
                staticValues[MotionProperty.PaintFilter] = new MotionValue.FilterList(
                    next.PaintFilter
                );
            if (next?.ClipPolygon is not null)
                staticValues[MotionProperty.ClipPolygon] = new MotionValue.ClipPolygon(
                    next.ClipPolygon
                );
            if (next?.BoxShadow is not null)
                staticValues[MotionProperty.BoxShadow] = new MotionValue.ShadowList(next.BoxShadow);
            if (next?.ClipInset is not null)
                staticValues[MotionProperty.ClipInset] = new MotionValue.ClipInset(next.ClipInset);
            target.MarkDirtyRepaint();
        }

        public void ClearMotion()
        {
            motionValues.Clear();
            backgroundLease?.Dispose();
            backgroundLease = null;
            maskLease?.Dispose();
            maskLease = null;
            materialLease?.Dispose();
            materialLease = null;
            target.style.backgroundImage = StyleKeyword.None;
            target.MarkDirtyRepaint();
        }

        public void ClearMotionValue(MotionProperty property)
        {
            if (motionValues.Remove(property))
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
            if (property == MotionProperty.PaintFilter && !EmptyPaint(value))
                if (!HasStaticFill)
                    throw Failure("Motion paint filters require an owned PaintStyle background.");
            if (EmptyPaint(value))
                motionValues.Remove(property);
            else
                motionValues[property] = value;
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
            Painter2D painter = context.painter2D;
            if (HasPrimaryPaint())
            {
                IReadOnlyList<Vector2> points = Geometry(rect);
                if (points.Count >= 3)
                {
                    DrawFilterShadows(painter, points);
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
                    DrawInsetShadows(painter, points, rect);
                }
            }
            foreach (PaintLayer layer in staticLayers)
                PaintStaticLayer(painter, rect, layer);
        }

        private bool PaintGradientSegments(
            Painter2D painter,
            IReadOnlyList<Vector2> points,
            UnityRect rect
        )
        {
            if (!TryValue(MotionProperty.BackgroundGradient, out MotionValue value))
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
                MotionValue.Gradient { Value: Gradient.Linear linear } => linear.Stops.Count == 0,
                MotionValue.Gradient { Value: Gradient.Radial radial } => radial.Stops.Count == 0,
                MotionValue.FilterList filters => filters.Value.Count == 0,
                MotionValue.ShadowList shadows => shadows.Value.Count == 0,
                _ => false,
            };

        private bool HasPrimaryPaint() =>
            HasValue(MotionProperty.BackgroundColor)
            || HasValue(MotionProperty.BackgroundGradient)
            || HasValue(MotionProperty.PaintFilter)
            || HasValue(MotionProperty.BoxShadow)
            || HasValue(MotionProperty.ClipInset)
            || HasValue(MotionProperty.ClipPolygon)
            || HasValue(MotionProperty.RotateX)
            || HasValue(MotionProperty.RotateY)
            || HasValue(MotionProperty.SkewX)
            || HasValue(MotionProperty.SkewY)
            || HasValue(MotionProperty.TransformList)
            || HasValue(MotionProperty.Mask);

        private bool HasPaint() => HasPrimaryPaint() || staticLayers.Count > 0;

        private void PaintStaticLayer(Painter2D painter, UnityRect hostRect, PaintLayer layer)
        {
            UnityRect rect = LayerBounds(hostRect, layer.BoundsInset);
            if (rect.width <= 0 || rect.height <= 0)
                return;
            painter.fillTexture = null;
            List<Vector2> points =
                layer.ClipPolygon is not null && layer.ClipPolygon.Count >= 3
                    ? Polygon(rect, layer.ClipPolygon)
                    : BattlementPaintContour.RoundedBox(rect, target.resolvedStyle);
            if (layer.ClipInset is not null)
                points = BattlementPaintContour.Inset(points, rect, layer.ClipInset);
            if (points.Count < 3)
                return;
            IReadOnlyList<UiFilterFunction> filters =
                layer.PaintFilter ?? Array.Empty<UiFilterFunction>();
            for (int index = 0; index < filters.Count; index++)
                if (filters[index] is UiFilterFunction.DropShadow dropShadow)
                    DrawOuterShadow(
                        painter,
                        points,
                        Brightened(dropShadow.Value, filters, index + 1)
                    );
            foreach (Shadow shadow in layer.BoxShadow ?? Array.Empty<Shadow>())
                if (!shadow.Inset)
                    DrawOuterShadow(painter, points, shadow);
            if (layer.Background is PaintFill.Gradient gradient)
            {
                if (
                    !BattlementGradientSegments.Paint(
                        painter,
                        points,
                        rect,
                        gradient.Value,
                        (value, bounds) => FillGradient(value, bounds, filters)
                    )
                )
                {
                    painter.BeginPath();
                    Path(painter, points);
                    painter.fillGradient = FillGradient(gradient.Value, rect, filters);
                    painter.Fill(FillRule.NonZero);
                }
            }
            else
            {
                painter.BeginPath();
                Path(painter, points);
                painter.fillGradient = default;
                painter.fillColor = Brightened(
                    ToUnityColor(((PaintFill.Color)layer.Background).Value),
                    filters,
                    0
                );
                painter.Fill(FillRule.NonZero);
            }
            foreach (Shadow shadow in layer.BoxShadow ?? Array.Empty<Shadow>())
                if (shadow.Inset)
                    DrawInsetShadow(painter, points, rect, shadow);
        }

        private static UnityRect LayerBounds(UnityRect rect, IReadOnlyList<UiLength>? insets)
        {
            if (insets is null)
                return rect;
            float top = Resolve(insets[0], rect.height);
            float right = Resolve(insets[1], rect.width);
            float bottom = Resolve(insets[2], rect.height);
            float left = Resolve(insets[3], rect.width);
            return new UnityRect(
                rect.xMin + left,
                rect.yMin + top,
                rect.width - left - right,
                rect.height - top - bottom
            );
        }

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
            if (lease.Value is not Texture2D)
            {
                lease.Dispose();
                throw Failure($"Prepared motion texture '{address}' is not a Texture2D.");
            }
            ReplaceLease(mask, lease);
            if (!mask)
                target.style.backgroundImage = Background.FromTexture2D((Texture2D)lease.Value);
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
                TryValue(MotionProperty.ClipPolygon, out MotionValue polygonValue)
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
                TryValue(MotionProperty.ClipInset, out MotionValue value)
                && value is MotionValue.ClipInset inset
                && inset.Value.Count == 4
                ? BattlementPaintContour.Inset(points, rect, inset.Value)
                : points;
        }

        private static List<Vector2> Polygon(
            UnityRect rect,
            IReadOnlyList<IReadOnlyList<UiLength>> source
        )
        {
            var points = new List<Vector2>(source.Count);
            foreach (IReadOnlyList<UiLength> point in source)
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
                !TryValue(MotionProperty.TransformList, out MotionValue value)
                || value is not MotionValue.TransformList transforms
            )
                return;
            foreach (TransformOperation transform in transforms.Value)
                ApplyTransform(points, origin, transform);
        }

        private static void ApplyTransform(
            List<Vector2> points,
            Vector2 origin,
            TransformOperation transform
        )
        {
            for (int index = 0; index < points.Count; index++)
            {
                Vector2 offset = points[index] - origin;
                points[index] = transform switch
                {
                    TransformOperation.Translate translate => points[index]
                        + Translation(translate.Value),
                    TransformOperation.Rotate rotate => origin
                        + Rotate(offset, checked((float)rotate.Value[2])),
                    TransformOperation.Skew skew => origin
                        + new Vector2(
                            offset.x
                                + Mathf.Tan(checked((float)skew.Value[0]) * Mathf.Deg2Rad)
                                    * offset.y,
                            offset.y
                                + Mathf.Tan(checked((float)skew.Value[1]) * Mathf.Deg2Rad)
                                    * offset.x
                        ),
                    TransformOperation.Scale scale => origin
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
                TryValue(MotionProperty.BackgroundGradient, out MotionValue value)
                && value is MotionValue.Gradient gradient
            )
            {
                painter.fillGradient = FillGradient(gradient.Value, rect);
                return;
            }
            painter.fillColor = Brightened(
                TryValue(MotionProperty.BackgroundColor, out MotionValue color)
                && color is MotionValue.Color solid
                    ? ToUnityColor(solid.Value)
                    : target.resolvedStyle.backgroundColor
            );
        }

        private void DrawFilterShadows(Painter2D painter, IReadOnlyList<Vector2> points)
        {
            IReadOnlyList<UiFilterFunction> filters = PaintFilters();
            for (int index = 0; index < filters.Count; index++)
                if (filters[index] is UiFilterFunction.DropShadow dropShadow)
                    DrawOuterShadow(painter, points, Brightened(dropShadow.Value, index + 1));
        }

        private void DrawOuterShadows(Painter2D painter, IReadOnlyList<Vector2> points)
        {
            foreach (Shadow shadow in Shadows(false))
                DrawOuterShadow(painter, points, shadow);
        }

        private void DrawInsetShadows(
            Painter2D painter,
            IReadOnlyList<Vector2> points,
            UnityRect rect
        )
        {
            foreach (Shadow shadow in Shadows(true))
                DrawInsetShadow(painter, points, rect, shadow);
        }

        private void DrawInsetShadow(
            Painter2D painter,
            IReadOnlyList<Vector2> points,
            UnityRect rect,
            Shadow shadow
        ) =>
            BattlementPaintShadows.Inset(
                painter,
                points,
                BattlementPaintContour.RoundedBox(rect, target.resolvedStyle),
                shadow
            );

        private static void DrawOuterShadow(
            Painter2D painter,
            IReadOnlyList<Vector2> points,
            Shadow shadow
        ) => BattlementPaintShadows.Outer(painter, points, shadow);

        private IEnumerable<Shadow> Shadows(bool inset)
        {
            if (
                TryValue(MotionProperty.BoxShadow, out MotionValue value)
                && value is MotionValue.ShadowList shadows
            )
                foreach (Shadow shadow in shadows.Value)
                    if (shadow.Inset == inset)
                        yield return shadow;
        }

        private IReadOnlyList<UiFilterFunction> PaintFilters() =>
            HasStaticFill
            && TryValue(MotionProperty.PaintFilter, out MotionValue value)
            && value is MotionValue.FilterList filters
                ? filters.Value
                : Array.Empty<UiFilterFunction>();

        private UnityColor Brightened(UnityColor color)
        {
            return Brightened(color, 0);
        }

        private UnityColor Brightened(UnityColor color, int startIndex)
        {
            float brightness = 1;
            IReadOnlyList<UiFilterFunction> filters = PaintFilters();
            for (int index = startIndex; index < filters.Count; index++)
                if (filters[index] is UiFilterFunction.Brightness value)
                    brightness *= checked((float)value.Value);
            color.r *= brightness;
            color.g *= brightness;
            color.b *= brightness;
            return color;
        }

        private Shadow Brightened(Shadow shadow, int startIndex)
        {
            UnityColor color = Brightened(ToUnityColor(shadow.Color), startIndex);
            return shadow with { Color = new Color(color.r, color.g, color.b, color.a) };
        }

        private static Shadow Brightened(
            Shadow shadow,
            IReadOnlyList<UiFilterFunction> filters,
            int startIndex
        )
        {
            UnityColor color = Brightened(ToUnityColor(shadow.Color), filters, startIndex);
            return shadow with { Color = new Color(color.r, color.g, color.b, color.a) };
        }

        private static UnityColor Brightened(
            UnityColor color,
            IReadOnlyList<UiFilterFunction> filters,
            int startIndex
        )
        {
            float brightness = 1;
            for (int index = startIndex; index < filters.Count; index++)
                if (filters[index] is UiFilterFunction.Brightness value)
                    brightness *= checked((float)value.Value);
            color.r *= brightness;
            color.g *= brightness;
            color.b *= brightness;
            return color;
        }

        private static void Path(Painter2D painter, IReadOnlyList<Vector2> points)
        {
            painter.MoveTo(points[0]);
            for (int index = 1; index < points.Count; index++)
                painter.LineTo(points[index]);
            painter.ClosePath();
        }

        private float Angle(MotionProperty property) =>
            TryValue(property, out MotionValue value) && value is MotionValue.Angle angle
                ? checked((float)angle.Value)
                : 0;

        private static StyleList<FilterFunction> NativeFilters(
            IReadOnlyList<UiFilterFunction> filters
        )
        {
            var result = new List<FilterFunction>();
            foreach (UiFilterFunction filter in filters)
            {
                FilterFunction converted = filter switch
                {
                    UiFilterFunction.Tint tint => Tint(tint.Value),
                    UiFilterFunction.Blur blur => Function(FilterFunctionType.Blur, blur.Value),
                    UiFilterFunction.Contrast contrast => Function(
                        FilterFunctionType.Contrast,
                        contrast.Value
                    ),
                    UiFilterFunction.HueRotate hue => Function(
                        FilterFunctionType.HueRotate,
                        hue.Value
                    ),
                    UiFilterFunction.Opacity opacity => Function(
                        FilterFunctionType.Opacity,
                        opacity.Value
                    ),
                    UiFilterFunction.Invert invert => Function(
                        FilterFunctionType.Invert,
                        invert.Value
                    ),
                    UiFilterFunction.Grayscale grayscale => Function(
                        FilterFunctionType.Grayscale,
                        grayscale.Value
                    ),
                    UiFilterFunction.Sepia sepia => Function(FilterFunctionType.Sepia, sepia.Value),
                    _ => throw new BattlementUiException(
                        CoreErrorCode.InvalidProperty,
                        $"Motion filter {filter.GetType().Name} is unsupported "
                            + "by the Unity adapter."
                    ),
                };
                result.Add(converted);
            }
            return new StyleList<FilterFunction>(result);
        }

        private static FilterFunction Function(FilterFunctionType type, double value)
        {
            var result = new FilterFunction(type);
            result.AddParameter(new FilterParameter(checked((float)value)));
            return result;
        }

        private static FilterFunction Tint(Color value)
        {
            var result = new FilterFunction(FilterFunctionType.Tint);
            result.AddParameter(new FilterParameter(ToUnityColor(value)));
            return result;
        }

        private FillGradient FillGradient(Gradient value, UnityRect rect) =>
            FillGradient(value, rect, PaintFilters());

        private static FillGradient FillGradient(
            Gradient value,
            UnityRect rect,
            IReadOnlyList<UiFilterFunction> filters
        )
        {
            UnityGradient gradient = ToUnityGradient(value, filters);
            if (value is Gradient.Linear linear)
            {
                (Vector2 start, Vector2 end) = BattlementGradientSegments.Line(rect, linear.Angle);
                return UnityEngine.UIElements.FillGradient.MakeLinearGradient(
                    gradient,
                    start,
                    end,
                    AddressMode.Clamp
                );
            }
            if (value is Gradient.Radial radial)
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

        private static UnityGradient ToUnityGradient(
            Gradient value,
            IReadOnlyList<UiFilterFunction> filters
        )
        {
            IReadOnlyList<GradientStop> stops = value switch
            {
                Gradient.Linear linear => linear.Stops,
                Gradient.Radial radial => radial.Stops,
                _ => throw new InvalidOperationException("Unknown motion gradient."),
            };
            var colors = new GradientColorKey[stops.Count];
            var alpha = new GradientAlphaKey[stops.Count];
            for (int index = 0; index < stops.Count; index++)
            {
                GradientStop stop = stops[index];
                UnityColor color = Brightened(ToUnityColor(stop.Color), filters, 0);
                float time = checked((float)stop.Position);
                colors[index] = new GradientColorKey(color, time);
                alpha[index] = new GradientAlphaKey(color.a, time);
            }
            var gradient = new UnityGradient();
            gradient.SetKeys(colors, alpha);
            return gradient;
        }

        private static Vector2 Translation(IReadOnlyList<UiLength> value) =>
            new(checked((float)value[0].Pixels), checked((float)value[1].Pixels));

        private bool HasValue(MotionProperty property) =>
            motionValues.ContainsKey(property) || staticValues.ContainsKey(property);

        private bool TryValue(MotionProperty property, out MotionValue value)
        {
            if (motionValues.TryGetValue(property, out value))
                return true;
            return staticValues.TryGetValue(property, out value);
        }

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

        private static float Resolve(UiLength value, float reference) =>
            checked((float)(value.Pixels + value.Percentage * reference / 100));

        private static UnityColor ToUnityColor(Color value) =>
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
