#nullable enable

using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementMotionPropertyWriter
    {
        private static readonly ConditionalWeakTable<
            VisualElement,
            BattlementAdvancedPaint
        > Advanced = new();

        public static bool Supports(MotionProperty property) =>
            property
                is MotionProperty.Opacity
                    or MotionProperty.AspectRatio
                    or MotionProperty.FlexGrow
                    or MotionProperty.FlexShrink
                    or MotionProperty.UnitySliceBottom
                    or MotionProperty.UnitySliceLeft
                    or MotionProperty.UnitySliceRight
                    or MotionProperty.UnitySliceScale
                    or MotionProperty.UnitySliceTop
                    or MotionProperty.UnityTextOutlineWidth
                    or MotionProperty.BackgroundColor
                    or MotionProperty.BorderBottomColor
                    or MotionProperty.BorderLeftColor
                    or MotionProperty.BorderRightColor
                    or MotionProperty.BorderTopColor
                    or MotionProperty.Color
                    or MotionProperty.UnityBackgroundImageTintColor
                    or MotionProperty.UnityTextOutlineColor
                    or MotionProperty.Scale
                    or MotionProperty.ScaleX
                    or MotionProperty.ScaleY
                    or MotionProperty.X
                    or MotionProperty.Y
                    or MotionProperty.Z
                    or MotionProperty.Rotate
                    or MotionProperty.RotateX
                    or MotionProperty.RotateY
                    or MotionProperty.SkewX
                    or MotionProperty.SkewY
                    or MotionProperty.TransformList
                    or MotionProperty.Filter
                    or MotionProperty.BackgroundImage
                    or MotionProperty.BackgroundGradient
                    or MotionProperty.BoxShadow
                    or MotionProperty.ClipInset
                    or MotionProperty.ClipPolygon
                    or MotionProperty.Mask
                    or MotionProperty.UnityMaterial
                    or MotionProperty.Display
                    or MotionProperty.Visibility
                    or MotionProperty.BorderBottomLeftRadius
                    or MotionProperty.BorderBottomRightRadius
                    or MotionProperty.BorderBottomWidth
                    or MotionProperty.BorderLeftWidth
                    or MotionProperty.BorderRightWidth
                    or MotionProperty.BorderTopLeftRadius
                    or MotionProperty.BorderTopRightRadius
                    or MotionProperty.BorderTopWidth
                    or MotionProperty.Bottom
                    or MotionProperty.FlexBasis
                    or MotionProperty.FontSize
                    or MotionProperty.Height
                    or MotionProperty.Left
                    or MotionProperty.LetterSpacing
                    or MotionProperty.MarginBottom
                    or MotionProperty.MarginLeft
                    or MotionProperty.MarginRight
                    or MotionProperty.MarginTop
                    or MotionProperty.MaxHeight
                    or MotionProperty.MaxWidth
                    or MotionProperty.MinHeight
                    or MotionProperty.MinWidth
                    or MotionProperty.PaddingBottom
                    or MotionProperty.PaddingLeft
                    or MotionProperty.PaddingRight
                    or MotionProperty.PaddingTop
                    or MotionProperty.Right
                    or MotionProperty.Top
                    or MotionProperty.UnityParagraphSpacing
                    or MotionProperty.Width
                    or MotionProperty.WordSpacing;

        public static bool IsSpatial(MotionProperty property) =>
            property
                is MotionProperty.X
                    or MotionProperty.Y
                    or MotionProperty.Z
                    or MotionProperty.Translate
                    or MotionProperty.Rotate
                    or MotionProperty.RotateX
                    or MotionProperty.RotateY
                    or MotionProperty.Scale
                    or MotionProperty.ScaleX
                    or MotionProperty.ScaleY
                    or MotionProperty.SkewX
                    or MotionProperty.SkewY
                    or MotionProperty.TransformList;

        public static bool IsDiscrete(MotionProperty property) =>
            property
                is MotionProperty.BackgroundImage
                    or MotionProperty.Display
                    or MotionProperty.Mask
                    or MotionProperty.UnityMaterial
                    or MotionProperty.Visibility;

        public static bool IsLayout(MotionProperty property) =>
            property
                is not MotionProperty.Opacity
                    and not MotionProperty.BackgroundColor
                    and not MotionProperty.BorderBottomColor
                    and not MotionProperty.BorderLeftColor
                    and not MotionProperty.BorderRightColor
                    and not MotionProperty.BorderTopColor
                    and not MotionProperty.Color
                    and not MotionProperty.UnityBackgroundImageTintColor
                    and not MotionProperty.UnityTextOutlineColor
                    and not MotionProperty.UnityTextOutlineWidth
                    and not MotionProperty.Scale
                    and not MotionProperty.ScaleX
                    and not MotionProperty.ScaleY
                    and not MotionProperty.X
                    and not MotionProperty.Y
                    and not MotionProperty.Z
                    and not MotionProperty.Rotate
                    and not MotionProperty.RotateX
                    and not MotionProperty.RotateY
                    and not MotionProperty.SkewX
                    and not MotionProperty.SkewY
                    and not MotionProperty.TransformList
                    and not MotionProperty.Filter
                    and not MotionProperty.BackgroundImage
                    and not MotionProperty.BackgroundGradient
                    and not MotionProperty.BoxShadow
                    and not MotionProperty.ClipInset
                    and not MotionProperty.ClipPolygon
                    and not MotionProperty.Mask
                    and not MotionProperty.UnityMaterial
                    and not MotionProperty.Display
                    and not MotionProperty.Visibility;

        public static MotionValue Read(VisualElement target, MotionProperty property) =>
            property switch
            {
                MotionProperty.Opacity => Scalar(
                    Inline(target.style.opacity, target.resolvedStyle.opacity)
                ),
                MotionProperty.AspectRatio => Scalar(target.resolvedStyle.aspectRatio),
                MotionProperty.FlexGrow => Scalar(target.resolvedStyle.flexGrow),
                MotionProperty.FlexShrink => Scalar(target.resolvedStyle.flexShrink),
                MotionProperty.UnitySliceBottom => Scalar(target.resolvedStyle.unitySliceBottom),
                MotionProperty.UnitySliceLeft => Scalar(target.resolvedStyle.unitySliceLeft),
                MotionProperty.UnitySliceRight => Scalar(target.resolvedStyle.unitySliceRight),
                MotionProperty.UnitySliceScale => Scalar(target.resolvedStyle.unitySliceScale),
                MotionProperty.UnitySliceTop => Scalar(target.resolvedStyle.unitySliceTop),
                MotionProperty.UnityTextOutlineWidth => Scalar(
                    target.resolvedStyle.unityTextOutlineWidth
                ),
                MotionProperty.BackgroundColor => Color(
                    Inline(target.style.backgroundColor, target.resolvedStyle.backgroundColor)
                ),
                MotionProperty.BorderBottomColor => Color(target.resolvedStyle.borderBottomColor),
                MotionProperty.BorderLeftColor => Color(target.resolvedStyle.borderLeftColor),
                MotionProperty.BorderRightColor => Color(target.resolvedStyle.borderRightColor),
                MotionProperty.BorderTopColor => Color(target.resolvedStyle.borderTopColor),
                MotionProperty.Color => Color(
                    Inline(target.style.color, target.resolvedStyle.color)
                ),
                MotionProperty.UnityBackgroundImageTintColor => Color(
                    target.resolvedStyle.unityBackgroundImageTintColor
                ),
                MotionProperty.UnityTextOutlineColor => Color(
                    target.resolvedStyle.unityTextOutlineColor
                ),
                MotionProperty.Scale => new MotionValue.Vector2(
                    new double[]
                    {
                        Inline(target.style.scale, target.resolvedStyle.scale).value.x,
                        Inline(target.style.scale, target.resolvedStyle.scale).value.y,
                    }
                ),
                MotionProperty.ScaleX => Scalar(
                    Inline(target.style.scale, target.resolvedStyle.scale).value.x
                ),
                MotionProperty.ScaleY => Scalar(
                    Inline(target.style.scale, target.resolvedStyle.scale).value.y
                ),
                MotionProperty.X => Length(target.resolvedStyle.translate.x),
                MotionProperty.Y => Length(target.resolvedStyle.translate.y),
                MotionProperty.Z => new MotionValue.Length(
                    new MotionLength(target.resolvedStyle.translate.z, 0)
                ),
                MotionProperty.Rotate => new MotionValue.Angle(
                    Inline(target.style.rotate, target.resolvedStyle.rotate).angle.value
                ),
                MotionProperty.RotateX
                or MotionProperty.RotateY
                or MotionProperty.SkewX
                or MotionProperty.SkewY => Stored(target, property, new MotionValue.Angle(0)),
                MotionProperty.TransformList => Stored(
                    target,
                    property,
                    new MotionValue.TransformList(Array.Empty<MotionTransform>())
                ),
                MotionProperty.Filter => Stored(
                    target,
                    property,
                    new MotionValue.FilterList(Array.Empty<MotionFilter>())
                ),
                MotionProperty.BackgroundImage => Stored(
                    target,
                    property,
                    new MotionValue.Discrete(Newtonsoft.Json.Linq.JValue.CreateNull())
                ),
                MotionProperty.BackgroundGradient => Stored(
                    target,
                    property,
                    new MotionValue.Gradient(
                        new MotionGradient.Linear(0, Array.Empty<MotionGradientStop>())
                    )
                ),
                MotionProperty.BoxShadow => Stored(
                    target,
                    property,
                    new MotionValue.ShadowList(Array.Empty<MotionShadow>())
                ),
                MotionProperty.ClipInset => Stored(
                    target,
                    property,
                    new MotionValue.ClipInset(
                        new[]
                        {
                            new MotionLength(0, 0),
                            new MotionLength(0, 0),
                            new MotionLength(0, 0),
                            new MotionLength(0, 0),
                        }
                    )
                ),
                MotionProperty.ClipPolygon => Stored(
                    target,
                    property,
                    new MotionValue.ClipPolygon(Array.Empty<IReadOnlyList<MotionLength>>())
                ),
                MotionProperty.Mask => Stored(
                    target,
                    property,
                    new MotionValue.Discrete(Newtonsoft.Json.Linq.JValue.CreateNull())
                ),
                MotionProperty.UnityMaterial => Stored(
                    target,
                    property,
                    new MotionValue.Discrete(Newtonsoft.Json.Linq.JValue.CreateNull())
                ),
                MotionProperty.Display => new MotionValue.Discrete(
                    target.resolvedStyle.display == DisplayStyle.Flex ? "flex" : "none"
                ),
                MotionProperty.Visibility => new MotionValue.Discrete(
                    target.resolvedStyle.visibility == Visibility.Visible ? "visible" : "hidden"
                ),
                MotionProperty.BorderBottomLeftRadius => Length(
                    target.resolvedStyle.borderBottomLeftRadius
                ),
                MotionProperty.BorderBottomRightRadius => Length(
                    target.resolvedStyle.borderBottomRightRadius
                ),
                MotionProperty.BorderBottomWidth => Length(target.resolvedStyle.borderBottomWidth),
                MotionProperty.BorderLeftWidth => Length(target.resolvedStyle.borderLeftWidth),
                MotionProperty.BorderRightWidth => Length(target.resolvedStyle.borderRightWidth),
                MotionProperty.BorderTopLeftRadius => Length(
                    target.resolvedStyle.borderTopLeftRadius
                ),
                MotionProperty.BorderTopRightRadius => Length(
                    target.resolvedStyle.borderTopRightRadius
                ),
                MotionProperty.BorderTopWidth => Length(target.resolvedStyle.borderTopWidth),
                MotionProperty.Bottom => Length(target.resolvedStyle.bottom),
                MotionProperty.FlexBasis => Length(target.resolvedStyle.flexBasis.value),
                MotionProperty.FontSize => Length(target.resolvedStyle.fontSize),
                MotionProperty.Height => Length(target.resolvedStyle.height),
                MotionProperty.Left => Length(target.resolvedStyle.left),
                MotionProperty.LetterSpacing => Length(target.resolvedStyle.letterSpacing),
                MotionProperty.MarginBottom => Length(target.resolvedStyle.marginBottom),
                MotionProperty.MarginLeft => Length(target.resolvedStyle.marginLeft),
                MotionProperty.MarginRight => Length(target.resolvedStyle.marginRight),
                MotionProperty.MarginTop => Length(target.resolvedStyle.marginTop),
                MotionProperty.MaxHeight => Length(target.resolvedStyle.maxHeight.value),
                MotionProperty.MaxWidth => Length(target.resolvedStyle.maxWidth.value),
                MotionProperty.MinHeight => Length(target.resolvedStyle.minHeight.value),
                MotionProperty.MinWidth => Length(target.resolvedStyle.minWidth.value),
                MotionProperty.PaddingBottom => Length(target.resolvedStyle.paddingBottom),
                MotionProperty.PaddingLeft => Length(target.resolvedStyle.paddingLeft),
                MotionProperty.PaddingRight => Length(target.resolvedStyle.paddingRight),
                MotionProperty.PaddingTop => Length(target.resolvedStyle.paddingTop),
                MotionProperty.Right => Length(target.resolvedStyle.right),
                MotionProperty.Top => Length(target.resolvedStyle.top),
                MotionProperty.UnityParagraphSpacing => Length(
                    target.resolvedStyle.unityParagraphSpacing
                ),
                MotionProperty.Width => Length(target.resolvedStyle.width),
                MotionProperty.WordSpacing => Length(target.resolvedStyle.wordSpacing),
                _ => throw Unsupported(property),
            };

        public static void Write(VisualElement target, MotionProperty property, MotionValue value)
        {
            if (value is MotionValue.Scalar scalar)
            {
                WriteScalar(target, property, scalar.Value);
                return;
            }
            if (value is MotionValue.Length)
            {
                WriteLength(target, property, UnityLength(value));
                return;
            }
            if (value is MotionValue.Color)
            {
                WriteColor(target, property, UnityColor(value));
                return;
            }
            if (value is MotionValue.Vector2 vector && property == MotionProperty.Scale)
            {
                if (vector.Value.Count != 2)
                    throw new InvalidOperationException("Motion scale requires two channels.");
                target.style.scale = new Scale(
                    new UnityEngine.Vector2(
                        checked((float)vector.Value[0]),
                        checked((float)vector.Value[1])
                    )
                );
                return;
            }
            if (value is MotionValue.Angle angle && property == MotionProperty.Rotate)
            {
                target.style.rotate = new Rotate(new Angle((float)angle.Value, AngleUnit.Degree));
                return;
            }
            if (
                property
                is MotionProperty.RotateX
                    or MotionProperty.RotateY
                    or MotionProperty.SkewX
                    or MotionProperty.SkewY
                    or MotionProperty.TransformList
                    or MotionProperty.Filter
                    or MotionProperty.BackgroundImage
                    or MotionProperty.BackgroundGradient
                    or MotionProperty.BoxShadow
                    or MotionProperty.ClipInset
                    or MotionProperty.ClipPolygon
                    or MotionProperty.Mask
                    or MotionProperty.UnityMaterial
            )
            {
                Advanced
                    .GetValue(target, element => new BattlementAdvancedPaint(element))
                    .Write(property, value);
                return;
            }
            if (value is MotionValue.Discrete discrete && property == MotionProperty.Visibility)
            {
                target.style.visibility = discrete.Value.ToObject<string>() switch
                {
                    "visible" => Visibility.Visible,
                    "hidden" => Visibility.Hidden,
                    _ => throw new InvalidOperationException(
                        "Motion visibility must be visible or hidden."
                    ),
                };
                return;
            }
            if (value is MotionValue.Discrete display && property == MotionProperty.Display)
            {
                target.style.display = display.Value.ToObject<string>() switch
                {
                    "flex" => DisplayStyle.Flex,
                    "none" => DisplayStyle.None,
                    _ => throw new InvalidOperationException(
                        "Motion display must be flex or none."
                    ),
                };
                return;
            }
            throw Unsupported(property);
        }

        public static void Configure(VisualElement target, IBattlementUiAssetLookup? assets) =>
            Advanced
                .GetValue(target, element => new BattlementAdvancedPaint(element))
                .Configure(assets);

        public static void Release(VisualElement target)
        {
            if (Advanced.TryGetValue(target, out BattlementAdvancedPaint paint))
                paint.Dispose();
            Advanced.Remove(target);
        }

        public static void WriteScalar(VisualElement target, MotionProperty property, double value)
        {
            float number = checked((float)value);
            if (property == MotionProperty.Opacity)
                target.style.opacity = number;
            else if (property == MotionProperty.AspectRatio)
                target.style.aspectRatio = number;
            else if (property == MotionProperty.FlexGrow)
                target.style.flexGrow = number;
            else if (property == MotionProperty.FlexShrink)
                target.style.flexShrink = number;
            else if (property == MotionProperty.UnitySliceBottom)
                target.style.unitySliceBottom = checked((int)Math.Round(number));
            else if (property == MotionProperty.UnitySliceLeft)
                target.style.unitySliceLeft = checked((int)Math.Round(number));
            else if (property == MotionProperty.UnitySliceRight)
                target.style.unitySliceRight = checked((int)Math.Round(number));
            else if (property == MotionProperty.UnitySliceScale)
                target.style.unitySliceScale = number;
            else if (property == MotionProperty.UnitySliceTop)
                target.style.unitySliceTop = checked((int)Math.Round(number));
            else if (property == MotionProperty.UnityTextOutlineWidth)
                target.style.unityTextOutlineWidth = number;
            else if (property == MotionProperty.ScaleX)
                target.style.scale = new Scale(
                    new UnityEngine.Vector2(number, target.resolvedStyle.scale.value.y)
                );
            else if (property == MotionProperty.ScaleY)
                target.style.scale = new Scale(
                    new UnityEngine.Vector2(target.resolvedStyle.scale.value.x, number)
                );
            else
                throw Unsupported(property);
        }

        public static void WriteTranslation(VisualElement target, float x, float y) =>
            target.style.translate = new Translate(
                new Length(x, LengthUnit.Pixel),
                new Length(y, LengthUnit.Pixel),
                target.resolvedStyle.translate.z
            );

        private static void WriteColor(
            VisualElement target,
            MotionProperty property,
            UnityEngine.Color value
        )
        {
            if (property == MotionProperty.BackgroundColor)
                target.style.backgroundColor = value;
            else if (property == MotionProperty.BorderBottomColor)
                target.style.borderBottomColor = value;
            else if (property == MotionProperty.BorderLeftColor)
                target.style.borderLeftColor = value;
            else if (property == MotionProperty.BorderRightColor)
                target.style.borderRightColor = value;
            else if (property == MotionProperty.BorderTopColor)
                target.style.borderTopColor = value;
            else if (property == MotionProperty.Color)
                target.style.color = value;
            else if (property == MotionProperty.UnityBackgroundImageTintColor)
                target.style.unityBackgroundImageTintColor = value;
            else if (property == MotionProperty.UnityTextOutlineColor)
                target.style.unityTextOutlineColor = value;
            else
                throw Unsupported(property);
        }

        private static void WriteLength(
            VisualElement target,
            MotionProperty property,
            StyleLength value
        )
        {
            if (property == MotionProperty.BorderBottomLeftRadius)
                target.style.borderBottomLeftRadius = value;
            else if (property == MotionProperty.BorderBottomRightRadius)
                target.style.borderBottomRightRadius = value;
            else if (property == MotionProperty.BorderBottomWidth)
                target.style.borderBottomWidth = value.value.value;
            else if (property == MotionProperty.BorderLeftWidth)
                target.style.borderLeftWidth = value.value.value;
            else if (property == MotionProperty.BorderRightWidth)
                target.style.borderRightWidth = value.value.value;
            else if (property == MotionProperty.BorderTopLeftRadius)
                target.style.borderTopLeftRadius = value;
            else if (property == MotionProperty.BorderTopRightRadius)
                target.style.borderTopRightRadius = value;
            else if (property == MotionProperty.BorderTopWidth)
                target.style.borderTopWidth = value.value.value;
            else if (property == MotionProperty.Bottom)
                target.style.bottom = value;
            else if (property == MotionProperty.FlexBasis)
                target.style.flexBasis = value;
            else if (property == MotionProperty.FontSize)
                target.style.fontSize = value;
            else if (property == MotionProperty.Height)
                target.style.height = value;
            else if (property == MotionProperty.Left)
                target.style.left = value;
            else if (property == MotionProperty.LetterSpacing)
                target.style.letterSpacing = value;
            else if (property == MotionProperty.MarginBottom)
                target.style.marginBottom = value;
            else if (property == MotionProperty.MarginLeft)
                target.style.marginLeft = value;
            else if (property == MotionProperty.MarginRight)
                target.style.marginRight = value;
            else if (property == MotionProperty.MarginTop)
                target.style.marginTop = value;
            else if (property == MotionProperty.MaxHeight)
                target.style.maxHeight = value;
            else if (property == MotionProperty.MaxWidth)
                target.style.maxWidth = value;
            else if (property == MotionProperty.MinHeight)
                target.style.minHeight = value;
            else if (property == MotionProperty.MinWidth)
                target.style.minWidth = value;
            else if (property == MotionProperty.PaddingBottom)
                target.style.paddingBottom = value;
            else if (property == MotionProperty.PaddingLeft)
                target.style.paddingLeft = value;
            else if (property == MotionProperty.PaddingRight)
                target.style.paddingRight = value;
            else if (property == MotionProperty.PaddingTop)
                target.style.paddingTop = value;
            else if (property == MotionProperty.Right)
                target.style.right = value;
            else if (property == MotionProperty.Top)
                target.style.top = value;
            else if (property == MotionProperty.UnityParagraphSpacing)
                target.style.unityParagraphSpacing = value;
            else if (property == MotionProperty.Width)
                target.style.width = value;
            else if (property == MotionProperty.WordSpacing)
                target.style.wordSpacing = value;
            else if (property == MotionProperty.X)
                target.style.translate = new Translate(
                    value.value,
                    target.resolvedStyle.translate.y,
                    target.resolvedStyle.translate.z
                );
            else if (property == MotionProperty.Y)
                target.style.translate = new Translate(
                    target.resolvedStyle.translate.x,
                    value.value,
                    target.resolvedStyle.translate.z
                );
            else if (property == MotionProperty.Z)
                target.style.translate = new Translate(
                    target.resolvedStyle.translate.x,
                    target.resolvedStyle.translate.y,
                    value.value.value
                );
            else
                throw Unsupported(property);
        }

        private static MotionValue Stored(
            VisualElement target,
            MotionProperty property,
            MotionValue fallback
        ) =>
            Advanced.TryGetValue(target, out BattlementAdvancedPaint paint)
                ? paint.Read(property, fallback)
                : fallback;

        private static float Inline(StyleFloat value, float resolved) =>
            value.keyword == StyleKeyword.Undefined ? value.value : resolved;

        private static UnityEngine.Color Inline(StyleColor value, UnityEngine.Color resolved) =>
            value.keyword == StyleKeyword.Undefined ? value.value : resolved;

        private static Scale Inline(StyleScale value, Scale resolved) =>
            value.keyword == StyleKeyword.Undefined ? value.value : resolved;

        private static Rotate Inline(StyleRotate value, Rotate resolved) =>
            value.keyword == StyleKeyword.Undefined ? value.value : resolved;

        private static MotionValue Scalar(float value) => new MotionValue.Scalar(value);

        private static MotionValue Color(UnityEngine.Color value) =>
            new MotionValue.Color(new MotionColor(value.r, value.g, value.b, value.a));

        private static MotionValue Length(float value) =>
            new MotionValue.Length(new MotionLength(value, 0));

        private static UnityEngine.Color UnityColor(MotionValue value) =>
            value is MotionValue.Color color
                ? new UnityEngine.Color(
                    checked((float)color.Value.Red),
                    checked((float)color.Value.Green),
                    checked((float)color.Value.Blue),
                    checked((float)color.Value.Alpha)
                )
                : throw new InvalidOperationException("Motion writer expected a color.");

        private static StyleLength UnityLength(MotionValue value)
        {
            if (value is not MotionValue.Length length)
                throw new InvalidOperationException("Motion writer expected a length.");
            if (length.Value.Px != 0 && length.Value.Percent != 0)
                throw new InvalidOperationException(
                    "A mixed motion length must be resolved first."
                );
            return length.Value.Percent == 0
                ? new StyleLength(new Length(checked((float)length.Value.Px), LengthUnit.Pixel))
                : new StyleLength(
                    new Length(checked((float)length.Value.Percent), LengthUnit.Percent)
                );
        }

        private static InvalidOperationException Unsupported(MotionProperty property) =>
            new($"Motion property {property} has no Task 02 Unity writer.");
    }
}
