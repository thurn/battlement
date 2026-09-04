#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementPaintProperties
    {
        public static void Apply(VisualElement target, Prop<PaintStyle> value)
        {
            if (value.IsUnset)
                return;
            BattlementAdvancedPaint.For(target).ReplaceStatic(value.IsSet ? value.Value : null);
        }

        public static void Release(VisualElement target) => BattlementAdvancedPaint.Release(target);

        public static bool Owns(MotionProperty property) =>
            property
                is MotionProperty.BackgroundColor
                    or MotionProperty.PaintFilter
                    or MotionProperty.BackgroundGradient
                    or MotionProperty.BoxShadow
                    or MotionProperty.ClipInset
                    or MotionProperty.ClipPolygon;

        public static MotionValue Read(VisualElement target, MotionProperty property)
        {
            MotionValue fallback = property switch
            {
                MotionProperty.BackgroundColor =>
                    BattlementMotionPropertyWriter.ReadNativeBackgroundColor(target),
                MotionProperty.PaintFilter => new MotionValue.FilterList(
                    Array.Empty<UiFilterFunction>()
                ),
                MotionProperty.BackgroundGradient => new MotionValue.Gradient(
                    new Gradient.Linear(0, Array.Empty<GradientStop>())
                ),
                MotionProperty.BoxShadow => new MotionValue.ShadowList(Array.Empty<Shadow>()),
                MotionProperty.ClipInset => new MotionValue.ClipInset(
                    new[]
                    {
                        UiLength.FromComponents(0, 0),
                        UiLength.FromComponents(0, 0),
                        UiLength.FromComponents(0, 0),
                        UiLength.FromComponents(0, 0),
                    }
                ),
                MotionProperty.ClipPolygon => new MotionValue.ClipPolygon(
                    Array.Empty<IReadOnlyList<UiLength>>()
                ),
                _ => throw new InvalidOperationException("Property is not static paint."),
            };
            return BattlementAdvancedPaint.TryGet(target, out BattlementAdvancedPaint paint)
                ? paint.ReadStatic(property, fallback)
                : fallback;
        }

        public static void Validate(Prop<PaintStyle> value)
        {
            if (!value.IsSet)
                return;
            PaintStyle paint = value.Value;
            if (paint.Background is not null)
                ValidateFill(paint.Background);
            if (paint.PaintFilter is not null && paint.Background is null)
                throw Invalid("Paint filters require an owned background.");
            ValidateFilters(paint.PaintFilter);
            ValidatePolygon(paint.ClipPolygon);
            ValidateInsets(paint.ClipInset, "Paint clip insets require four values.");
            foreach (Shadow shadow in paint.BoxShadow ?? Array.Empty<Shadow>())
                ValidateShadow(shadow);
            foreach (PaintLayer layer in paint.Layers ?? Array.Empty<PaintLayer>())
                ValidateLayer(layer);
        }

        private static void ValidateLayer(PaintLayer layer)
        {
            ValidateFill(layer.Background);
            ValidateFilters(layer.PaintFilter);
            ValidatePolygon(layer.ClipPolygon);
            ValidateInsets(layer.ClipInset, "Paint clip insets require four values.");
            ValidateInsets(layer.BoundsInset, "Paint bounds insets require four values.");
            foreach (Shadow shadow in layer.BoxShadow ?? Array.Empty<Shadow>())
                ValidateShadow(shadow);
        }

        private static void ValidateFill(PaintFill fill)
        {
            if (fill is PaintFill.Color color)
                ValidateColor(color.Value);
            else if (fill is PaintFill.Gradient gradient)
                ValidateGradient(gradient.Value);
            else
                throw Invalid("Paint requires a known background fill.");
        }

        private static void ValidateFilters(IReadOnlyList<UiFilterFunction>? filters)
        {
            int dropShadows = 0;
            foreach (UiFilterFunction filter in filters ?? Array.Empty<UiFilterFunction>())
            {
                if (filter is UiFilterFunction.Brightness brightness)
                {
                    Finite(brightness.Value);
                    if (brightness.Value < 0)
                        throw Invalid("Paint brightness must be nonnegative.");
                }
                else if (filter is UiFilterFunction.DropShadow shadow)
                {
                    if (shadow.Value.Inset)
                        throw Invalid("Paint drop-shadow cannot be inset.");
                    ValidateShadow(shadow.Value);
                    dropShadows++;
                    if (dropShadows > 1)
                        throw Invalid("Paint filters support one drop-shadow.");
                }
                else
                    throw Invalid("Paint filter received an unsupported operation.");
            }
        }

        private static void ValidatePolygon(IReadOnlyList<IReadOnlyList<UiLength>>? polygon)
        {
            if (polygon is null)
                return;
            if (polygon.Count < 3)
                throw Invalid("Paint clip polygon must contain at least three vertices.");
            foreach (IReadOnlyList<UiLength> point in polygon)
            {
                if (point.Count != 2 || !Finite(point[0].Pixels, point[0].Percentage))
                    throw Invalid("Paint clip polygon must contain finite two-axis vertices.");
                if (!Finite(point[1].Pixels, point[1].Percentage))
                    throw Invalid("Paint clip polygon must contain finite two-axis vertices.");
            }
        }

        private static void ValidateInsets(IReadOnlyList<UiLength>? insets, string message)
        {
            if (insets is null)
                return;
            if (insets.Count != 4)
                throw Invalid(message);
            foreach (UiLength inset in insets)
                if (!Finite(inset.Pixels, inset.Percentage))
                    throw Invalid("Paint insets must be finite.");
        }

        private static void ValidateGradient(Gradient value)
        {
            IReadOnlyList<GradientStop> stops;
            if (value is Gradient.Linear linear)
            {
                if (!Finite(linear.Angle))
                    throw Invalid("Paint gradient geometry must be finite.");
                stops = linear.Stops;
            }
            else if (value is Gradient.Radial radial)
            {
                if (radial.Center.Count != 2 || radial.Radius.Count != 2)
                    throw Invalid("Paint radial gradients require two-axis geometry.");
                if (!Finite(radial.Center[0], radial.Center[1]))
                    throw Invalid("Paint gradient geometry must be finite.");
                if (!Finite(radial.Radius[0], radial.Radius[1]))
                    throw Invalid("Paint gradient geometry must be finite.");
                stops = radial.Stops;
            }
            else
                throw Invalid("Unknown paint gradient.");
            if (stops.Count == 0)
                throw Invalid("Paint gradients require at least one stop.");
            foreach (GradientStop stop in stops)
            {
                if (!Finite(stop.Position))
                    throw Invalid("Paint gradient stops must be finite.");
                ValidateColor(stop.Color);
            }
        }

        private static void ValidateColor(Color value)
        {
            if (!Finite(value.Red, value.Green, value.Blue, value.Alpha))
                throw Invalid("Paint colors must be finite.");
        }

        private static void ValidateShadow(Shadow value)
        {
            if (!Finite(value.X, value.Y, value.Blur, value.Spread))
                throw Invalid("Paint shadows must be finite.");
            if (value.Blur < 0)
                throw Invalid("Paint shadow blur must be nonnegative.");
            ValidateColor(value.Color);
        }

        private static bool Finite(params double[] values)
        {
            foreach (double value in values)
                if (double.IsNaN(value) || double.IsInfinity(value))
                    return false;
            return true;
        }

        private static BattlementUiException Invalid(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}
