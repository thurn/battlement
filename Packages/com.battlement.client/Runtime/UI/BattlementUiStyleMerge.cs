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
                Filter = update.Filter ?? current.Filter,
                FlexBasis = Merge(current.FlexBasis, update.FlexBasis),
                FlexDirection = Merge(current.FlexDirection, update.FlexDirection),
                FlexGrow = Merge(current.FlexGrow, update.FlexGrow),
                FlexShrink = Merge(current.FlexShrink, update.FlexShrink),
                FlexWrap = Merge(current.FlexWrap, update.FlexWrap),
                FontSize = update.FontSize ?? current.FontSize,
                Height = Merge(current.Height, update.Height),
                JustifyContent = Merge(current.JustifyContent, update.JustifyContent),
                LetterSpacing = update.LetterSpacing ?? current.LetterSpacing,
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
                Rotate = update.Rotate ?? current.Rotate,
                Scale = update.Scale ?? current.Scale,
                TextOverflow = update.TextOverflow ?? current.TextOverflow,
                TextShadow = update.TextShadow ?? current.TextShadow,
                Top = Merge(current.Top, update.Top),
                TransformOrigin = update.TransformOrigin ?? current.TransformOrigin,
                TransitionDelay = update.TransitionDelay ?? current.TransitionDelay,
                TransitionDuration = update.TransitionDuration ?? current.TransitionDuration,
                TransitionProperty = update.TransitionProperty ?? current.TransitionProperty,
                TransitionTimingFunction =
                    update.TransitionTimingFunction ?? current.TransitionTimingFunction,
                Translate = update.Translate ?? current.Translate,
                UnityBackgroundImageTintColor = Merge(
                    current.UnityBackgroundImageTintColor,
                    update.UnityBackgroundImageTintColor
                ),
                UnityEditorTextRenderingMode =
                    update.UnityEditorTextRenderingMode ?? current.UnityEditorTextRenderingMode,
                UnityFontDefinition = update.UnityFontDefinition ?? current.UnityFontDefinition,
                UnityFontStyleAndWeight =
                    update.UnityFontStyleAndWeight ?? current.UnityFontStyleAndWeight,
                UnityMaterial = Merge(current.UnityMaterial, update.UnityMaterial),
                UnityOverflowClipBox = Merge(
                    current.UnityOverflowClipBox,
                    update.UnityOverflowClipBox
                ),
                UnityParagraphSpacing =
                    update.UnityParagraphSpacing ?? current.UnityParagraphSpacing,
                UnitySliceBottom = Merge(current.UnitySliceBottom, update.UnitySliceBottom),
                UnitySliceLeft = Merge(current.UnitySliceLeft, update.UnitySliceLeft),
                UnitySliceRight = Merge(current.UnitySliceRight, update.UnitySliceRight),
                UnitySliceScale = Merge(current.UnitySliceScale, update.UnitySliceScale),
                UnitySliceTop = Merge(current.UnitySliceTop, update.UnitySliceTop),
                UnitySliceType = Merge(current.UnitySliceType, update.UnitySliceType),
                UnityTextAlign = update.UnityTextAlign ?? current.UnityTextAlign,
                UnityTextAutoSize = update.UnityTextAutoSize ?? current.UnityTextAutoSize,
                UnityTextGenerator = update.UnityTextGenerator ?? current.UnityTextGenerator,
                UnityTextOutlineColor =
                    update.UnityTextOutlineColor ?? current.UnityTextOutlineColor,
                UnityTextOutlineWidth =
                    update.UnityTextOutlineWidth ?? current.UnityTextOutlineWidth,
                UnityTextOverflowPosition =
                    update.UnityTextOverflowPosition ?? current.UnityTextOverflowPosition,
                Visibility = update.Visibility ?? current.Visibility,
                WhiteSpace = update.WhiteSpace ?? current.WhiteSpace,
                Width = Merge(current.Width, update.Width),
                WordSpacing = update.WordSpacing ?? current.WordSpacing,
            };

        private static Prop<T> Merge<T>(Prop<T> current, Prop<T> update) =>
            update.IsUnset ? current : update;
    }
}
