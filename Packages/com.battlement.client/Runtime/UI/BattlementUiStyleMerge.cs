#nullable enable

namespace Battlement.UI
{
    internal static class BattlementUiStyleMerge
    {
        public static UiStyle Merge(UiStyle current, UiStyle update) =>
            current with
            {
                AlignContent = update.AlignContent ?? current.AlignContent,
                AlignItems = update.AlignItems ?? current.AlignItems,
                AlignSelf = update.AlignSelf ?? current.AlignSelf,
                AspectRatio = update.AspectRatio ?? current.AspectRatio,
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
                BorderBottomWidth = update.BorderBottomWidth ?? current.BorderBottomWidth,
                BorderLeftColor = update.BorderLeftColor ?? current.BorderLeftColor,
                BorderLeftWidth = update.BorderLeftWidth ?? current.BorderLeftWidth,
                BorderRightColor = update.BorderRightColor ?? current.BorderRightColor,
                BorderRightWidth = update.BorderRightWidth ?? current.BorderRightWidth,
                BorderTopColor = update.BorderTopColor ?? current.BorderTopColor,
                BorderTopLeftRadius = update.BorderTopLeftRadius ?? current.BorderTopLeftRadius,
                BorderTopRightRadius = update.BorderTopRightRadius ?? current.BorderTopRightRadius,
                BorderTopWidth = update.BorderTopWidth ?? current.BorderTopWidth,
                Bottom = update.Bottom ?? current.Bottom,
                Color = update.Color ?? current.Color,
                Cursor = update.Cursor ?? current.Cursor,
                Display = update.Display ?? current.Display,
                Filter = update.Filter ?? current.Filter,
                FlexBasis = update.FlexBasis ?? current.FlexBasis,
                FlexDirection = update.FlexDirection ?? current.FlexDirection,
                FlexGrow = update.FlexGrow ?? current.FlexGrow,
                FlexShrink = update.FlexShrink ?? current.FlexShrink,
                FlexWrap = update.FlexWrap ?? current.FlexWrap,
                FontSize = update.FontSize ?? current.FontSize,
                Height = update.Height ?? current.Height,
                JustifyContent = update.JustifyContent ?? current.JustifyContent,
                LetterSpacing = update.LetterSpacing ?? current.LetterSpacing,
                Left = update.Left ?? current.Left,
                MarginBottom = update.MarginBottom ?? current.MarginBottom,
                MarginLeft = update.MarginLeft ?? current.MarginLeft,
                MarginRight = update.MarginRight ?? current.MarginRight,
                MarginTop = update.MarginTop ?? current.MarginTop,
                MaxHeight = update.MaxHeight ?? current.MaxHeight,
                MaxWidth = update.MaxWidth ?? current.MaxWidth,
                MinHeight = update.MinHeight ?? current.MinHeight,
                MinWidth = update.MinWidth ?? current.MinWidth,
                Opacity = update.Opacity ?? current.Opacity,
                Overflow = update.Overflow ?? current.Overflow,
                PaddingBottom = update.PaddingBottom ?? current.PaddingBottom,
                PaddingLeft = update.PaddingLeft ?? current.PaddingLeft,
                PaddingRight = update.PaddingRight ?? current.PaddingRight,
                PaddingTop = update.PaddingTop ?? current.PaddingTop,
                Position = update.Position ?? current.Position,
                Right = update.Right ?? current.Right,
                Rotate = update.Rotate ?? current.Rotate,
                Scale = update.Scale ?? current.Scale,
                TextOverflow = update.TextOverflow ?? current.TextOverflow,
                TextShadow = update.TextShadow ?? current.TextShadow,
                Top = update.Top ?? current.Top,
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
                Width = update.Width ?? current.Width,
                WordSpacing = update.WordSpacing ?? current.WordSpacing,
            };
    }
}
