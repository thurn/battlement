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
                BackgroundColor = update.BackgroundColor ?? current.BackgroundColor,
                BackgroundImage = update.BackgroundImage ?? current.BackgroundImage,
                BackgroundPositionX = update.BackgroundPositionX ?? current.BackgroundPositionX,
                BackgroundPositionY = update.BackgroundPositionY ?? current.BackgroundPositionY,
                BackgroundRepeat = update.BackgroundRepeat ?? current.BackgroundRepeat,
                BackgroundSize = update.BackgroundSize ?? current.BackgroundSize,
                BorderBottomColor = update.BorderBottomColor ?? current.BorderBottomColor,
                BorderBottomLeftRadius =
                    update.BorderBottomLeftRadius ?? current.BorderBottomLeftRadius,
                BorderBottomRightRadius =
                    update.BorderBottomRightRadius ?? current.BorderBottomRightRadius,
                BorderBottomWidth = Merge(current.BorderBottomWidth, update.BorderBottomWidth),
                BorderLeftColor = update.BorderLeftColor ?? current.BorderLeftColor,
                BorderLeftWidth = Merge(current.BorderLeftWidth, update.BorderLeftWidth),
                BorderRightColor = update.BorderRightColor ?? current.BorderRightColor,
                BorderRightWidth = Merge(current.BorderRightWidth, update.BorderRightWidth),
                BorderTopColor = update.BorderTopColor ?? current.BorderTopColor,
                BorderTopLeftRadius = update.BorderTopLeftRadius ?? current.BorderTopLeftRadius,
                BorderTopRightRadius = update.BorderTopRightRadius ?? current.BorderTopRightRadius,
                BorderTopWidth = Merge(current.BorderTopWidth, update.BorderTopWidth),
                Bottom = Merge(current.Bottom, update.Bottom),
                Color = update.Color ?? current.Color,
                Cursor = update.Cursor ?? current.Cursor,
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
                Opacity = update.Opacity ?? current.Opacity,
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
                UnityBackgroundImageTintColor =
                    update.UnityBackgroundImageTintColor ?? current.UnityBackgroundImageTintColor,
                UnityEditorTextRenderingMode =
                    update.UnityEditorTextRenderingMode ?? current.UnityEditorTextRenderingMode,
                UnityFontDefinition = update.UnityFontDefinition ?? current.UnityFontDefinition,
                UnityFontStyleAndWeight =
                    update.UnityFontStyleAndWeight ?? current.UnityFontStyleAndWeight,
                UnityMaterial = update.UnityMaterial ?? current.UnityMaterial,
                UnityOverflowClipBox = update.UnityOverflowClipBox ?? current.UnityOverflowClipBox,
                UnityParagraphSpacing =
                    update.UnityParagraphSpacing ?? current.UnityParagraphSpacing,
                UnitySliceBottom = update.UnitySliceBottom ?? current.UnitySliceBottom,
                UnitySliceLeft = update.UnitySliceLeft ?? current.UnitySliceLeft,
                UnitySliceRight = update.UnitySliceRight ?? current.UnitySliceRight,
                UnitySliceScale = update.UnitySliceScale ?? current.UnitySliceScale,
                UnitySliceTop = update.UnitySliceTop ?? current.UnitySliceTop,
                UnitySliceType = update.UnitySliceType ?? current.UnitySliceType,
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
