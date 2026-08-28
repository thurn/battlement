#nullable enable

namespace Battlement.UI
{
    internal static class BattlementUiStyleMerge
    {
        public static UiStyle Merge(UiStyle current, UiStyle update) =>
            current with
            {
                AlignContent = Merge(current.AlignContent, update.AlignContent),
                AlignItems = Merge(current.AlignItems, update.AlignItems),
                AlignSelf = Merge(current.AlignSelf, update.AlignSelf),
                AspectRatio = Merge(current.AspectRatio, update.AspectRatio),
                BackgroundColor = Merge(current.BackgroundColor, update.BackgroundColor),
                BackgroundImage = Merge(current.BackgroundImage, update.BackgroundImage),
                BackgroundPositionX = Merge(
                    current.BackgroundPositionX,
                    update.BackgroundPositionX
                ),
                BackgroundPositionY = Merge(
                    current.BackgroundPositionY,
                    update.BackgroundPositionY
                ),
                BackgroundRepeat = Merge(current.BackgroundRepeat, update.BackgroundRepeat),
                BackgroundSize = Merge(current.BackgroundSize, update.BackgroundSize),
                BorderBottomColor = Merge(current.BorderBottomColor, update.BorderBottomColor),
                BorderBottomLeftRadius = Merge(
                    current.BorderBottomLeftRadius,
                    update.BorderBottomLeftRadius
                ),
                BorderBottomRightRadius = Merge(
                    current.BorderBottomRightRadius,
                    update.BorderBottomRightRadius
                ),
                BorderBottomWidth = Merge(current.BorderBottomWidth, update.BorderBottomWidth),
                BorderLeftColor = Merge(current.BorderLeftColor, update.BorderLeftColor),
                BorderLeftWidth = Merge(current.BorderLeftWidth, update.BorderLeftWidth),
                BorderRightColor = Merge(current.BorderRightColor, update.BorderRightColor),
                BorderRightWidth = Merge(current.BorderRightWidth, update.BorderRightWidth),
                BorderTopColor = Merge(current.BorderTopColor, update.BorderTopColor),
                BorderTopLeftRadius = Merge(
                    current.BorderTopLeftRadius,
                    update.BorderTopLeftRadius
                ),
                BorderTopRightRadius = Merge(
                    current.BorderTopRightRadius,
                    update.BorderTopRightRadius
                ),
                BorderTopWidth = Merge(current.BorderTopWidth, update.BorderTopWidth),
                Bottom = Merge(current.Bottom, update.Bottom),
                Color = Merge(current.Color, update.Color),
                Cursor = Merge(current.Cursor, update.Cursor),
                Display = Merge(current.Display, update.Display),
                Filter = Merge(current.Filter, update.Filter),
                FlexBasis = Merge(current.FlexBasis, update.FlexBasis),
                FlexDirection = Merge(current.FlexDirection, update.FlexDirection),
                FlexGrow = Merge(current.FlexGrow, update.FlexGrow),
                FlexShrink = Merge(current.FlexShrink, update.FlexShrink),
                FlexWrap = Merge(current.FlexWrap, update.FlexWrap),
                FontSize = Merge(current.FontSize, update.FontSize),
                Height = Merge(current.Height, update.Height),
                JustifyContent = Merge(current.JustifyContent, update.JustifyContent),
                LetterSpacing = Merge(current.LetterSpacing, update.LetterSpacing),
                Left = Merge(current.Left, update.Left),
                MarginBottom = Merge(current.MarginBottom, update.MarginBottom),
                MarginLeft = Merge(current.MarginLeft, update.MarginLeft),
                MarginRight = Merge(current.MarginRight, update.MarginRight),
                MarginTop = Merge(current.MarginTop, update.MarginTop),
                MaxHeight = Merge(current.MaxHeight, update.MaxHeight),
                MaxWidth = Merge(current.MaxWidth, update.MaxWidth),
                MinHeight = Merge(current.MinHeight, update.MinHeight),
                MinWidth = Merge(current.MinWidth, update.MinWidth),
                Opacity = Merge(current.Opacity, update.Opacity),
                Overflow = Merge(current.Overflow, update.Overflow),
                PaddingBottom = Merge(current.PaddingBottom, update.PaddingBottom),
                PaddingLeft = Merge(current.PaddingLeft, update.PaddingLeft),
                PaddingRight = Merge(current.PaddingRight, update.PaddingRight),
                PaddingTop = Merge(current.PaddingTop, update.PaddingTop),
                Position = Merge(current.Position, update.Position),
                Right = Merge(current.Right, update.Right),
                Rotate = Merge(current.Rotate, update.Rotate),
                Scale = Merge(current.Scale, update.Scale),
                TextOverflow = Merge(current.TextOverflow, update.TextOverflow),
                TextShadow = Merge(current.TextShadow, update.TextShadow),
                Top = Merge(current.Top, update.Top),
                TransformOrigin = Merge(current.TransformOrigin, update.TransformOrigin),
                TransitionDelay = Merge(current.TransitionDelay, update.TransitionDelay),
                TransitionDuration = Merge(current.TransitionDuration, update.TransitionDuration),
                TransitionProperty = Merge(current.TransitionProperty, update.TransitionProperty),
                TransitionTimingFunction = Merge(
                    current.TransitionTimingFunction,
                    update.TransitionTimingFunction
                ),
                Translate = Merge(current.Translate, update.Translate),
                UnityBackgroundImageTintColor = Merge(
                    current.UnityBackgroundImageTintColor,
                    update.UnityBackgroundImageTintColor
                ),
                UnityEditorTextRenderingMode = Merge(
                    current.UnityEditorTextRenderingMode,
                    update.UnityEditorTextRenderingMode
                ),
                UnityFontDefinition = Merge(
                    current.UnityFontDefinition,
                    update.UnityFontDefinition
                ),
                UnityFontStyleAndWeight = Merge(
                    current.UnityFontStyleAndWeight,
                    update.UnityFontStyleAndWeight
                ),
                UnityMaterial = Merge(current.UnityMaterial, update.UnityMaterial),
                UnityOverflowClipBox = Merge(
                    current.UnityOverflowClipBox,
                    update.UnityOverflowClipBox
                ),
                UnityParagraphSpacing = Merge(
                    current.UnityParagraphSpacing,
                    update.UnityParagraphSpacing
                ),
                UnitySliceBottom = Merge(current.UnitySliceBottom, update.UnitySliceBottom),
                UnitySliceLeft = Merge(current.UnitySliceLeft, update.UnitySliceLeft),
                UnitySliceRight = Merge(current.UnitySliceRight, update.UnitySliceRight),
                UnitySliceScale = Merge(current.UnitySliceScale, update.UnitySliceScale),
                UnitySliceTop = Merge(current.UnitySliceTop, update.UnitySliceTop),
                UnitySliceType = Merge(current.UnitySliceType, update.UnitySliceType),
                UnityTextAlign = Merge(current.UnityTextAlign, update.UnityTextAlign),
                UnityTextAutoSize = Merge(current.UnityTextAutoSize, update.UnityTextAutoSize),
                UnityTextGenerator = Merge(current.UnityTextGenerator, update.UnityTextGenerator),
                UnityTextOutlineColor = Merge(
                    current.UnityTextOutlineColor,
                    update.UnityTextOutlineColor
                ),
                UnityTextOutlineWidth = Merge(
                    current.UnityTextOutlineWidth,
                    update.UnityTextOutlineWidth
                ),
                UnityTextOverflowPosition = Merge(
                    current.UnityTextOverflowPosition,
                    update.UnityTextOverflowPosition
                ),
                Visibility = Merge(current.Visibility, update.Visibility),
                WhiteSpace = Merge(current.WhiteSpace, update.WhiteSpace),
                Width = Merge(current.Width, update.Width),
                WordSpacing = Merge(current.WordSpacing, update.WordSpacing),
            };

        private static Prop<T> Merge<T>(Prop<T> current, Prop<T> update) =>
            update.IsUnset ? current : update;
    }
}
