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
            if (paint.Background is PaintFill.Color color)
                ValidateColor(color.Value);
            if (paint.Background is PaintFill.Gradient gradient)
                ValidateGradient(gradient.Value);
            if (paint.ClipPolygon is not null)
            {
                if (paint.ClipPolygon.Count == 0)
                    throw Invalid("Paint clip polygon cannot be empty.");
                foreach (IReadOnlyList<UiLength> point in paint.ClipPolygon)
                {
                    if (point.Count != 2 || !Finite(point[0].Pixels, point[0].Percentage))
                        throw Invalid("Paint clip polygon must contain finite two-axis vertices.");
                    if (!Finite(point[1].Pixels, point[1].Percentage))
                        throw Invalid("Paint clip polygon must contain finite two-axis vertices.");
                }
            }
            if (paint.ClipInset is not null)
            {
                if (paint.ClipInset.Count != 4)
                    throw Invalid("Paint clip insets require four values.");
                foreach (UiLength inset in paint.ClipInset)
                    if (!Finite(inset.Pixels, inset.Percentage))
                        throw Invalid("Paint clip insets must be finite.");
            }
            foreach (Shadow shadow in paint.BoxShadow ?? Array.Empty<Shadow>())
            {
                if (!Finite(shadow.X, shadow.Y, shadow.Blur, shadow.Spread))
                    throw Invalid("Paint shadows must be finite.");
                ValidateColor(shadow.Color);
                if (shadow.Blur != 0)
                    throw Invalid(
                        "Paint shadow blur is unsupported by Unity; use generated paint."
                    );
            }
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
