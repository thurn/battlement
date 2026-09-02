#nullable enable

using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementAuthoredMotionStyle
    {
        public static bool Changed(
            UiStyle style,
            MotionProperty property,
            VisualElement target,
            out System.Action? reset
        )
        {
            (bool changed, System.Action? restore) = property switch
            {
                MotionProperty.AlignContent => Change(
                    style.AlignContent,
                    () => target.style.alignContent = Keyword(style.AlignContent)
                ),
                MotionProperty.AlignItems => Change(
                    style.AlignItems,
                    () => target.style.alignItems = Keyword(style.AlignItems)
                ),
                MotionProperty.AlignSelf => Change(
                    style.AlignSelf,
                    () => target.style.alignSelf = Keyword(style.AlignSelf)
                ),
                MotionProperty.AspectRatio => Change(
                    style.AspectRatio,
                    () => target.style.aspectRatio = Keyword(style.AspectRatio)
                ),
                MotionProperty.BackgroundColor => Change(
                    style.BackgroundColor,
                    () => target.style.backgroundColor = Keyword(style.BackgroundColor)
                ),
                MotionProperty.BackgroundImage => Change(
                    style.BackgroundImage,
                    () => target.style.backgroundImage = Keyword(style.BackgroundImage)
                ),
                MotionProperty.BackgroundPositionX => Change(
                    style.BackgroundPositionX,
                    () => target.style.backgroundPositionX = Keyword(style.BackgroundPositionX)
                ),
                MotionProperty.BackgroundPositionY => Change(
                    style.BackgroundPositionY,
                    () => target.style.backgroundPositionY = Keyword(style.BackgroundPositionY)
                ),
                MotionProperty.BackgroundRepeat => Change(
                    style.BackgroundRepeat,
                    () => target.style.backgroundRepeat = Keyword(style.BackgroundRepeat)
                ),
                MotionProperty.BackgroundSize => Change(
                    style.BackgroundSize,
                    () => target.style.backgroundSize = Keyword(style.BackgroundSize)
                ),
                MotionProperty.BorderBottomColor => Change(
                    style.BorderBottomColor,
                    () => target.style.borderBottomColor = Keyword(style.BorderBottomColor)
                ),
                MotionProperty.BorderBottomLeftRadius => Change(
                    style.BorderBottomLeftRadius,
                    () =>
                        target.style.borderBottomLeftRadius = Keyword(style.BorderBottomLeftRadius)
                ),
                MotionProperty.BorderBottomRightRadius => Change(
                    style.BorderBottomRightRadius,
                    () =>
                        target.style.borderBottomRightRadius = Keyword(
                            style.BorderBottomRightRadius
                        )
                ),
                MotionProperty.BorderBottomWidth => Change(
                    style.BorderBottomWidth,
                    () => target.style.borderBottomWidth = Keyword(style.BorderBottomWidth)
                ),
                MotionProperty.BorderLeftColor => Change(
                    style.BorderLeftColor,
                    () => target.style.borderLeftColor = Keyword(style.BorderLeftColor)
                ),
                MotionProperty.BorderLeftWidth => Change(
                    style.BorderLeftWidth,
                    () => target.style.borderLeftWidth = Keyword(style.BorderLeftWidth)
                ),
                MotionProperty.BorderRightColor => Change(
                    style.BorderRightColor,
                    () => target.style.borderRightColor = Keyword(style.BorderRightColor)
                ),
                MotionProperty.BorderRightWidth => Change(
                    style.BorderRightWidth,
                    () => target.style.borderRightWidth = Keyword(style.BorderRightWidth)
                ),
                MotionProperty.BorderTopColor => Change(
                    style.BorderTopColor,
                    () => target.style.borderTopColor = Keyword(style.BorderTopColor)
                ),
                MotionProperty.BorderTopLeftRadius => Change(
                    style.BorderTopLeftRadius,
                    () => target.style.borderTopLeftRadius = Keyword(style.BorderTopLeftRadius)
                ),
                MotionProperty.BorderTopRightRadius => Change(
                    style.BorderTopRightRadius,
                    () => target.style.borderTopRightRadius = Keyword(style.BorderTopRightRadius)
                ),
                MotionProperty.BorderTopWidth => Change(
                    style.BorderTopWidth,
                    () => target.style.borderTopWidth = Keyword(style.BorderTopWidth)
                ),
                MotionProperty.Bottom => Change(
                    style.Bottom,
                    () => target.style.bottom = Keyword(style.Bottom)
                ),
                MotionProperty.Color => Change(
                    style.Color,
                    () => target.style.color = Keyword(style.Color)
                ),
                MotionProperty.Cursor => Change(
                    style.Cursor,
                    () => target.style.cursor = Keyword(style.Cursor)
                ),
                MotionProperty.Display => Change(
                    style.Display,
                    () => target.style.display = Keyword(style.Display)
                ),
                MotionProperty.Filter => Change(
                    style.Filter,
                    () => target.style.filter = Keyword(style.Filter)
                ),
                MotionProperty.FlexBasis => Change(
                    style.FlexBasis,
                    () => target.style.flexBasis = Keyword(style.FlexBasis)
                ),
                MotionProperty.FlexDirection => Change(
                    style.FlexDirection,
                    () => target.style.flexDirection = Keyword(style.FlexDirection)
                ),
                MotionProperty.FlexGrow => Change(
                    style.FlexGrow,
                    () => target.style.flexGrow = Keyword(style.FlexGrow)
                ),
                MotionProperty.FlexShrink => Change(
                    style.FlexShrink,
                    () => target.style.flexShrink = Keyword(style.FlexShrink)
                ),
                MotionProperty.FlexWrap => Change(
                    style.FlexWrap,
                    () => target.style.flexWrap = Keyword(style.FlexWrap)
                ),
                MotionProperty.FontSize => Change(
                    style.FontSize,
                    () => target.style.fontSize = Keyword(style.FontSize)
                ),
                MotionProperty.Height => Change(
                    style.Height,
                    () => target.style.height = Keyword(style.Height)
                ),
                MotionProperty.JustifyContent => Change(
                    style.JustifyContent,
                    () => target.style.justifyContent = Keyword(style.JustifyContent)
                ),
                MotionProperty.LetterSpacing => Change(
                    style.LetterSpacing,
                    () => target.style.letterSpacing = Keyword(style.LetterSpacing)
                ),
                MotionProperty.Left => Change(
                    style.Left,
                    () => target.style.left = Keyword(style.Left)
                ),
                MotionProperty.MarginBottom => Change(
                    style.MarginBottom,
                    () => target.style.marginBottom = Keyword(style.MarginBottom)
                ),
                MotionProperty.MarginLeft => Change(
                    style.MarginLeft,
                    () => target.style.marginLeft = Keyword(style.MarginLeft)
                ),
                MotionProperty.MarginRight => Change(
                    style.MarginRight,
                    () => target.style.marginRight = Keyword(style.MarginRight)
                ),
                MotionProperty.MarginTop => Change(
                    style.MarginTop,
                    () => target.style.marginTop = Keyword(style.MarginTop)
                ),
                MotionProperty.MaxHeight => Change(
                    style.MaxHeight,
                    () => target.style.maxHeight = Keyword(style.MaxHeight)
                ),
                MotionProperty.MaxWidth => Change(
                    style.MaxWidth,
                    () => target.style.maxWidth = Keyword(style.MaxWidth)
                ),
                MotionProperty.MinHeight => Change(
                    style.MinHeight,
                    () => target.style.minHeight = Keyword(style.MinHeight)
                ),
                MotionProperty.MinWidth => Change(
                    style.MinWidth,
                    () => target.style.minWidth = Keyword(style.MinWidth)
                ),
                MotionProperty.Opacity => Change(
                    style.Opacity,
                    () => target.style.opacity = Keyword(style.Opacity)
                ),
                MotionProperty.Overflow => Change(
                    style.Overflow,
                    () => target.style.overflow = Keyword(style.Overflow)
                ),
                MotionProperty.PaddingBottom => Change(
                    style.PaddingBottom,
                    () => target.style.paddingBottom = Keyword(style.PaddingBottom)
                ),
                MotionProperty.PaddingLeft => Change(
                    style.PaddingLeft,
                    () => target.style.paddingLeft = Keyword(style.PaddingLeft)
                ),
                MotionProperty.PaddingRight => Change(
                    style.PaddingRight,
                    () => target.style.paddingRight = Keyword(style.PaddingRight)
                ),
                MotionProperty.PaddingTop => Change(
                    style.PaddingTop,
                    () => target.style.paddingTop = Keyword(style.PaddingTop)
                ),
                MotionProperty.Position => Change(
                    style.Position,
                    () => target.style.position = Keyword(style.Position)
                ),
                MotionProperty.Right => Change(
                    style.Right,
                    () => target.style.right = Keyword(style.Right)
                ),
                MotionProperty.Rotate => Change(
                    style.Rotate,
                    () => target.style.rotate = Keyword(style.Rotate)
                ),
                MotionProperty.Scale => Change(
                    style.Scale,
                    () => target.style.scale = Keyword(style.Scale)
                ),
                MotionProperty.TextOverflow => Change(
                    style.TextOverflow,
                    () => target.style.textOverflow = Keyword(style.TextOverflow)
                ),
                MotionProperty.TextShadow => Change(
                    style.TextShadow,
                    () => target.style.textShadow = Keyword(style.TextShadow)
                ),
                MotionProperty.Top => Change(
                    style.Top,
                    () => target.style.top = Keyword(style.Top)
                ),
                MotionProperty.TransformOrigin => Change(
                    style.TransformOrigin,
                    () => target.style.transformOrigin = Keyword(style.TransformOrigin)
                ),
                MotionProperty.Translate => Change(
                    style.Translate,
                    () => target.style.translate = Keyword(style.Translate)
                ),
                MotionProperty.UnityBackgroundImageTintColor => Change(
                    style.UnityBackgroundImageTintColor,
                    () =>
                        target.style.unityBackgroundImageTintColor = Keyword(
                            style.UnityBackgroundImageTintColor
                        )
                ),
                MotionProperty.UnityEditorTextRenderingMode => Change(
                    style.UnityEditorTextRenderingMode,
                    () =>
                        target.style.unityEditorTextRenderingMode = Keyword(
                            style.UnityEditorTextRenderingMode
                        )
                ),
                MotionProperty.UnityFontDefinition => Change(
                    style.UnityFontDefinition,
                    () => target.style.unityFontDefinition = Keyword(style.UnityFontDefinition)
                ),
                MotionProperty.UnityFontStyleAndWeight => Change(
                    style.UnityFontStyleAndWeight,
                    () =>
                        target.style.unityFontStyleAndWeight = Keyword(
                            style.UnityFontStyleAndWeight
                        )
                ),
                MotionProperty.UnityMaterial => Change(
                    style.UnityMaterial,
                    () => target.style.unityMaterial = Keyword(style.UnityMaterial)
                ),
                MotionProperty.UnityOverflowClipBox => Change(
                    style.UnityOverflowClipBox,
                    () => target.style.unityOverflowClipBox = Keyword(style.UnityOverflowClipBox)
                ),
                MotionProperty.UnityParagraphSpacing => Change(
                    style.UnityParagraphSpacing,
                    () => target.style.unityParagraphSpacing = Keyword(style.UnityParagraphSpacing)
                ),
                MotionProperty.UnitySliceBottom => Change(
                    style.UnitySliceBottom,
                    () => target.style.unitySliceBottom = Keyword(style.UnitySliceBottom)
                ),
                MotionProperty.UnitySliceLeft => Change(
                    style.UnitySliceLeft,
                    () => target.style.unitySliceLeft = Keyword(style.UnitySliceLeft)
                ),
                MotionProperty.UnitySliceRight => Change(
                    style.UnitySliceRight,
                    () => target.style.unitySliceRight = Keyword(style.UnitySliceRight)
                ),
                MotionProperty.UnitySliceScale => Change(
                    style.UnitySliceScale,
                    () => target.style.unitySliceScale = Keyword(style.UnitySliceScale)
                ),
                MotionProperty.UnitySliceTop => Change(
                    style.UnitySliceTop,
                    () => target.style.unitySliceTop = Keyword(style.UnitySliceTop)
                ),
                MotionProperty.UnitySliceType => Change(
                    style.UnitySliceType,
                    () => target.style.unitySliceType = Keyword(style.UnitySliceType)
                ),
                MotionProperty.UnityTextAlign => Change(
                    style.UnityTextAlign,
                    () => target.style.unityTextAlign = Keyword(style.UnityTextAlign)
                ),
                MotionProperty.UnityTextAutoSize => Change(
                    style.UnityTextAutoSize,
                    () => target.style.unityTextAutoSize = Keyword(style.UnityTextAutoSize)
                ),
                MotionProperty.UnityTextGenerator => Change(
                    style.UnityTextGenerator,
                    () => target.style.unityTextGenerator = Keyword(style.UnityTextGenerator)
                ),
                MotionProperty.UnityTextOutlineColor => Change(
                    style.UnityTextOutlineColor,
                    () => target.style.unityTextOutlineColor = Keyword(style.UnityTextOutlineColor)
                ),
                MotionProperty.UnityTextOutlineWidth => Change(
                    style.UnityTextOutlineWidth,
                    () => target.style.unityTextOutlineWidth = Keyword(style.UnityTextOutlineWidth)
                ),
                MotionProperty.UnityTextOverflowPosition => Change(
                    style.UnityTextOverflowPosition,
                    () =>
                        target.style.unityTextOverflowPosition = Keyword(
                            style.UnityTextOverflowPosition
                        )
                ),
                MotionProperty.Visibility => Change(
                    style.Visibility,
                    () => target.style.visibility = Keyword(style.Visibility)
                ),
                MotionProperty.WhiteSpace => Change(
                    style.WhiteSpace,
                    () => target.style.whiteSpace = Keyword(style.WhiteSpace)
                ),
                MotionProperty.Width => Change(
                    style.Width,
                    () => target.style.width = Keyword(style.Width)
                ),
                MotionProperty.WordSpacing => Change(
                    style.WordSpacing,
                    () => target.style.wordSpacing = Keyword(style.WordSpacing)
                ),
                MotionProperty.X or MotionProperty.Y or MotionProperty.Z => Change(
                    style.Translate,
                    () => target.style.translate = Keyword(style.Translate)
                ),
                MotionProperty.ScaleX or MotionProperty.ScaleY => Change(
                    style.Scale,
                    () => target.style.scale = Keyword(style.Scale)
                ),
                _ => (false, null),
            };
            reset = restore;
            return changed;
        }

        private static (bool, System.Action?) Change<T>(
            Prop<UiStyleValue<T>> value,
            System.Action reset
        ) =>
            value.IsUnset
                ? (false, null)
                : (true, value.IsReset || value.Value.Keyword is not null ? reset : null);

        private static StyleKeyword Keyword<T>(Prop<UiStyleValue<T>> value) =>
            value.IsReset ? StyleKeyword.Null : StyleKeyword.Initial;
    }
}
