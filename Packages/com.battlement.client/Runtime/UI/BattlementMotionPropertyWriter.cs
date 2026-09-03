#nullable enable

using System;
using System.Collections.Generic;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementMotionPropertyWriter
    {
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
                    or MotionProperty.PaintFilter
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
                    and not MotionProperty.PaintFilter
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
                MotionProperty.AspectRatio => Scalar(
                    Inline(target.style.aspectRatio, target.resolvedStyle.aspectRatio)
                ),
                MotionProperty.FlexGrow => Scalar(
                    Inline(target.style.flexGrow, target.resolvedStyle.flexGrow)
                ),
                MotionProperty.FlexShrink => Scalar(
                    Inline(target.style.flexShrink, target.resolvedStyle.flexShrink)
                ),
                MotionProperty.UnitySliceBottom => Scalar(
                    Inline(target.style.unitySliceBottom, target.resolvedStyle.unitySliceBottom)
                ),
                MotionProperty.UnitySliceLeft => Scalar(
                    Inline(target.style.unitySliceLeft, target.resolvedStyle.unitySliceLeft)
                ),
                MotionProperty.UnitySliceRight => Scalar(
                    Inline(target.style.unitySliceRight, target.resolvedStyle.unitySliceRight)
                ),
                MotionProperty.UnitySliceScale => Scalar(
                    Inline(target.style.unitySliceScale, target.resolvedStyle.unitySliceScale)
                ),
                MotionProperty.UnitySliceTop => Scalar(
                    Inline(target.style.unitySliceTop, target.resolvedStyle.unitySliceTop)
                ),
                MotionProperty.UnityTextOutlineWidth => Scalar(
                    Inline(
                        target.style.unityTextOutlineWidth,
                        target.resolvedStyle.unityTextOutlineWidth
                    )
                ),
                MotionProperty.BackgroundColor => Stored(
                    target,
                    property,
                    Color(
                        Inline(target.style.backgroundColor, target.resolvedStyle.backgroundColor)
                    )
                ),
                MotionProperty.BorderBottomColor => Color(
                    Inline(target.style.borderBottomColor, target.resolvedStyle.borderBottomColor)
                ),
                MotionProperty.BorderLeftColor => Color(
                    Inline(target.style.borderLeftColor, target.resolvedStyle.borderLeftColor)
                ),
                MotionProperty.BorderRightColor => Color(
                    Inline(target.style.borderRightColor, target.resolvedStyle.borderRightColor)
                ),
                MotionProperty.BorderTopColor => Color(
                    Inline(target.style.borderTopColor, target.resolvedStyle.borderTopColor)
                ),
                MotionProperty.Color => Color(
                    Inline(target.style.color, target.resolvedStyle.color)
                ),
                MotionProperty.UnityBackgroundImageTintColor => Color(
                    Inline(
                        target.style.unityBackgroundImageTintColor,
                        target.resolvedStyle.unityBackgroundImageTintColor
                    )
                ),
                MotionProperty.UnityTextOutlineColor => Color(
                    Inline(
                        target.style.unityTextOutlineColor,
                        target.resolvedStyle.unityTextOutlineColor
                    )
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
                MotionProperty.X => Length(
                    Inline(target.style.translate, target.resolvedStyle.translate).x
                ),
                MotionProperty.Y => Length(
                    Inline(target.style.translate, target.resolvedStyle.translate).y
                ),
                MotionProperty.Z => new MotionValue.Length(
                    UiLength.FromComponents(
                        Inline(target.style.translate, target.resolvedStyle.translate).z,
                        0
                    )
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
                    new MotionValue.TransformList(Array.Empty<TransformOperation>())
                ),
                MotionProperty.Filter => Stored(
                    target,
                    property,
                    new MotionValue.FilterList(Array.Empty<UiFilterFunction>())
                ),
                MotionProperty.PaintFilter => Stored(
                    target,
                    property,
                    new MotionValue.FilterList(Array.Empty<UiFilterFunction>())
                ),
                MotionProperty.BackgroundImage => Stored(
                    target,
                    property,
                    new MotionValue.Discrete(Newtonsoft.Json.Linq.JValue.CreateNull())
                ),
                MotionProperty.BackgroundGradient => Stored(
                    target,
                    property,
                    new MotionValue.Gradient(new Gradient.Linear(0, Array.Empty<GradientStop>()))
                ),
                MotionProperty.BoxShadow => Stored(
                    target,
                    property,
                    new MotionValue.ShadowList(Array.Empty<Shadow>())
                ),
                MotionProperty.ClipInset => Stored(
                    target,
                    property,
                    new MotionValue.ClipInset(
                        new[]
                        {
                            UiLength.FromComponents(0, 0),
                            UiLength.FromComponents(0, 0),
                            UiLength.FromComponents(0, 0),
                            UiLength.FromComponents(0, 0),
                        }
                    )
                ),
                MotionProperty.ClipPolygon => Stored(
                    target,
                    property,
                    new MotionValue.ClipPolygon(Array.Empty<IReadOnlyList<UiLength>>())
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
                MotionProperty.BorderBottomLeftRadius => ReadLength(
                    target.style.borderBottomLeftRadius,
                    target.resolvedStyle.borderBottomLeftRadius
                ),
                MotionProperty.BorderBottomRightRadius => ReadLength(
                    target.style.borderBottomRightRadius,
                    target.resolvedStyle.borderBottomRightRadius
                ),
                MotionProperty.BorderBottomWidth => ReadLength(
                    target.style.borderBottomWidth,
                    target.resolvedStyle.borderBottomWidth
                ),
                MotionProperty.BorderLeftWidth => ReadLength(
                    target.style.borderLeftWidth,
                    target.resolvedStyle.borderLeftWidth
                ),
                MotionProperty.BorderRightWidth => ReadLength(
                    target.style.borderRightWidth,
                    target.resolvedStyle.borderRightWidth
                ),
                MotionProperty.BorderTopLeftRadius => ReadLength(
                    target.style.borderTopLeftRadius,
                    target.resolvedStyle.borderTopLeftRadius
                ),
                MotionProperty.BorderTopRightRadius => ReadLength(
                    target.style.borderTopRightRadius,
                    target.resolvedStyle.borderTopRightRadius
                ),
                MotionProperty.BorderTopWidth => ReadLength(
                    target.style.borderTopWidth,
                    target.resolvedStyle.borderTopWidth
                ),
                MotionProperty.Bottom => ReadLength(
                    target.style.bottom,
                    target.resolvedStyle.bottom
                ),
                MotionProperty.FlexBasis => ReadLength(
                    target.style.flexBasis,
                    target.resolvedStyle.flexBasis.value
                ),
                MotionProperty.FontSize => ReadLength(
                    target.style.fontSize,
                    target.resolvedStyle.fontSize
                ),
                MotionProperty.Height => ReadLength(
                    target.style.height,
                    target.resolvedStyle.height
                ),
                MotionProperty.Left => ReadLength(target.style.left, target.resolvedStyle.left),
                MotionProperty.LetterSpacing => ReadLength(
                    BattlementTextSpacing.ReadStyle(target),
                    BattlementTextSpacing.ReadPixels(target)
                ),
                MotionProperty.MarginBottom => ReadLength(
                    target.style.marginBottom,
                    target.resolvedStyle.marginBottom
                ),
                MotionProperty.MarginLeft => ReadLength(
                    target.style.marginLeft,
                    target.resolvedStyle.marginLeft
                ),
                MotionProperty.MarginRight => ReadLength(
                    target.style.marginRight,
                    target.resolvedStyle.marginRight
                ),
                MotionProperty.MarginTop => ReadLength(
                    target.style.marginTop,
                    target.resolvedStyle.marginTop
                ),
                MotionProperty.MaxHeight => ReadLength(
                    target.style.maxHeight,
                    target.resolvedStyle.maxHeight.value
                ),
                MotionProperty.MaxWidth => ReadLength(
                    target.style.maxWidth,
                    target.resolvedStyle.maxWidth.value
                ),
                MotionProperty.MinHeight => ReadLength(
                    target.style.minHeight,
                    target.resolvedStyle.minHeight.value
                ),
                MotionProperty.MinWidth => ReadLength(
                    target.style.minWidth,
                    target.resolvedStyle.minWidth.value
                ),
                MotionProperty.PaddingBottom => ReadLength(
                    target.style.paddingBottom,
                    target.resolvedStyle.paddingBottom
                ),
                MotionProperty.PaddingLeft => ReadLength(
                    target.style.paddingLeft,
                    target.resolvedStyle.paddingLeft
                ),
                MotionProperty.PaddingRight => ReadLength(
                    target.style.paddingRight,
                    target.resolvedStyle.paddingRight
                ),
                MotionProperty.PaddingTop => ReadLength(
                    target.style.paddingTop,
                    target.resolvedStyle.paddingTop
                ),
                MotionProperty.Right => ReadLength(target.style.right, target.resolvedStyle.right),
                MotionProperty.Top => ReadLength(target.style.top, target.resolvedStyle.top),
                MotionProperty.UnityParagraphSpacing => ReadLength(
                    target.style.unityParagraphSpacing,
                    target.resolvedStyle.unityParagraphSpacing
                ),
                MotionProperty.Width => ReadLength(target.style.width, target.resolvedStyle.width),
                MotionProperty.WordSpacing => ReadLength(
                    target.style.wordSpacing,
                    target.resolvedStyle.wordSpacing
                ),
                _ => throw Unsupported(property),
            };

        public static MotionValue ReadNativeBackgroundColor(VisualElement target) =>
            Color(Inline(target.style.backgroundColor, target.resolvedStyle.backgroundColor));

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
                    or MotionProperty.PaintFilter
                    or MotionProperty.BackgroundImage
                    or MotionProperty.BackgroundGradient
                    or MotionProperty.BoxShadow
                    or MotionProperty.ClipInset
                    or MotionProperty.ClipPolygon
                    or MotionProperty.Mask
                    or MotionProperty.UnityMaterial
            )
            {
                BattlementAdvancedPaint.For(target).Write(property, value);
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
            BattlementAdvancedPaint.For(target).Configure(assets);

        public static bool HasStaticFill(VisualElement target) =>
            BattlementAdvancedPaint.TryGet(target, out BattlementAdvancedPaint paint)
            && paint.HasStaticFill;

        public static void CommitAuthoredStyle(VisualElement target, UiStyle style)
        {
            if (BattlementAdvancedPaint.TryGet(target, out BattlementAdvancedPaint paint))
                paint.CommitAuthoredStyle(style);
        }

        public static void Release(VisualElement target)
        {
            if (!BattlementAdvancedPaint.TryGet(target, out BattlementAdvancedPaint paint))
                return;
            paint.ClearMotion();
            if (!paint.HasStaticPaint)
                BattlementAdvancedPaint.Release(target);
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

        public static void WriteAdaptedScalar(
            VisualElement target,
            MotionProperty property,
            double value
        )
        {
            float number = checked((float)value);
            if (property == MotionProperty.Scale)
            {
                target.style.scale = new Scale(new UnityEngine.Vector2(number, number));
                return;
            }
            if (
                property
                is MotionProperty.X
                    or MotionProperty.Y
                    or MotionProperty.Z
                    or MotionProperty.Width
                    or MotionProperty.Height
                    or MotionProperty.MinWidth
                    or MotionProperty.MinHeight
                    or MotionProperty.MaxWidth
                    or MotionProperty.MaxHeight
            )
            {
                WriteLength(
                    target,
                    property,
                    new StyleLength(new Length(number, LengthUnit.Pixel))
                );
                return;
            }
            WriteScalar(target, property, value);
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
            {
                if (
                    BattlementAdvancedPaint.TryGet(target, out BattlementAdvancedPaint paint)
                    && paint.HasStaticFill
                )
                    paint.Write(property, Color(value));
                else
                {
                    if (paint is not null)
                        paint.ClearMotionValue(property);
                    target.style.backgroundColor = value;
                }
            }
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
                BattlementTextSpacing.Set(target, value);
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
            BattlementAdvancedPaint.TryGet(target, out BattlementAdvancedPaint paint)
                ? paint.Read(property, fallback)
                : fallback;

        private static MotionValue ReadLength(StyleLength value, float resolved) =>
            value.keyword == StyleKeyword.Undefined
                ? new MotionValue.Length(
                    value.value.unit == LengthUnit.Percent
                        ? UiLength.FromComponents(0, value.value.value)
                        : UiLength.FromComponents(value.value.value, 0)
                )
                : Length(resolved);

        private static MotionValue ReadLength(StyleFloat value, float resolved) =>
            Length(Inline(value, resolved));

        private static float Inline(StyleRatio value, float resolved) =>
            value.keyword == StyleKeyword.Undefined ? value.value : resolved;

        private static MotionValue Length(Length value) =>
            new MotionValue.Length(
                value.unit == LengthUnit.Percent
                    ? UiLength.FromComponents(0, value.value)
                    : UiLength.FromComponents(value.value, 0)
            );

        private static float Inline(StyleInt value, float resolved) =>
            value.keyword == StyleKeyword.Undefined ? value.value : resolved;

        private static Translate Inline(StyleTranslate value, Translate resolved) =>
            value.keyword == StyleKeyword.Undefined ? value.value : resolved;

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
            new MotionValue.Color(new Color(value.r, value.g, value.b, value.a));

        private static MotionValue Length(float value) =>
            new MotionValue.Length(UiLength.FromComponents(value, 0));

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
            if (length.Value.Pixels != 0 && length.Value.Percentage != 0)
                throw new InvalidOperationException(
                    "A mixed motion length must be resolved first."
                );
            return length.Value.Percentage == 0
                ? new StyleLength(new Length(checked((float)length.Value.Pixels), LengthUnit.Pixel))
                : new StyleLength(
                    new Length(checked((float)length.Value.Percentage), LengthUnit.Percent)
                );
        }

        private static InvalidOperationException Unsupported(MotionProperty property) =>
            new($"Motion property {property} has no Task 02 Unity writer.");
    }
}
