#nullable enable

using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementUiTypographyProperties
    {
        public static void Apply(
            UnityEngine.UIElements.TextElement target,
            UiElement.Label value
        ) =>
            Apply(
                target,
                value.Text,
                value.EnableRichText,
                value.EmojiFallbackSupport,
                value.ParseEscapeSequences,
                value.DisplayTooltipWhenElided,
                value.Selectable,
                value.DoubleClickSelectsWord,
                value.TripleClickSelectsLine,
                value.SelectAllOnFocus,
                value.SelectAllOnMouseUp
            );

        public static void Apply(
            UnityEngine.UIElements.TextElement target,
            UiElement.TextElement value
        ) =>
            Apply(
                target,
                value.Text,
                value.EnableRichText,
                value.EmojiFallbackSupport,
                value.ParseEscapeSequences,
                value.DisplayTooltipWhenElided,
                value.Selectable,
                value.DoubleClickSelectsWord,
                value.TripleClickSelectsLine,
                value.SelectAllOnFocus,
                value.SelectAllOnMouseUp
            );

        public static void ApplyStyle(
            IStyle target,
            UiStyle value,
            BattlementUiStyleFontProperties.FontLeases? fonts
        )
        {
            Apply(
                value.FontSize,
                item => target.fontSize = ToUnity(item),
                () => target.fontSize = StyleKeyword.Initial
            );
            Apply(
                value.LetterSpacing,
                item => target.letterSpacing = ToUnity(item),
                () => target.letterSpacing = StyleKeyword.Initial
            );
            Apply(
                value.TextOverflow,
                item =>
                    target.textOverflow =
                        item == UiTextOverflow.Clip
                            ? UnityEngine.UIElements.TextOverflow.Clip
                            : UnityEngine.UIElements.TextOverflow.Ellipsis,
                () => target.textOverflow = StyleKeyword.Initial
            );
            Apply(
                value.TextShadow,
                item =>
                    target.textShadow = new UnityEngine.UIElements.TextShadow
                    {
                        offset = new Vector2(item.X, item.Y),
                        blurRadius = item.BlurRadius,
                        color = ToUnity(item.Color),
                    },
                () => target.textShadow = StyleKeyword.Initial
            );
            Apply(
                value.UnityEditorTextRenderingMode,
                item =>
                    target.unityEditorTextRenderingMode =
                        item == UiEditorTextRenderingMode.Sdf
                            ? EditorTextRenderingMode.SDF
                            : EditorTextRenderingMode.Bitmap,
                () => target.unityEditorTextRenderingMode = StyleKeyword.Initial
            );
            Apply(
                value.UnityFont,
                _ => target.unityFont = (Font)fonts!.UnityFont!.Value,
                () => target.unityFont = StyleKeyword.Initial
            );
            Apply(
                value.UnityFontDefinition,
                _ =>
                    target.unityFontDefinition = new FontDefinition
                    {
                        fontAsset = (UnityEngine.TextCore.Text.FontAsset)
                            fonts!.FontDefinition!.Value,
                    },
                () => target.unityFontDefinition = StyleKeyword.Initial
            );
            Apply(
                value.UnityFontStyleAndWeight,
                item =>
                    target.unityFontStyleAndWeight = item switch
                    {
                        UiFontStyle.Bold => FontStyle.Bold,
                        UiFontStyle.Italic => FontStyle.Italic,
                        UiFontStyle.BoldAndItalic => FontStyle.BoldAndItalic,
                        _ => FontStyle.Normal,
                    },
                () => target.unityFontStyleAndWeight = StyleKeyword.Initial
            );
            Apply(
                value.UnityParagraphSpacing,
                item => target.unityParagraphSpacing = ToUnity(item),
                () => target.unityParagraphSpacing = StyleKeyword.Initial
            );
            Apply(
                value.UnityTextAlign,
                item =>
                    target.unityTextAlign = item switch
                    {
                        UiTextAnchor.UpperCenter => TextAnchor.UpperCenter,
                        UiTextAnchor.UpperRight => TextAnchor.UpperRight,
                        UiTextAnchor.MiddleLeft => TextAnchor.MiddleLeft,
                        UiTextAnchor.MiddleCenter => TextAnchor.MiddleCenter,
                        UiTextAnchor.MiddleRight => TextAnchor.MiddleRight,
                        UiTextAnchor.LowerLeft => TextAnchor.LowerLeft,
                        UiTextAnchor.LowerCenter => TextAnchor.LowerCenter,
                        UiTextAnchor.LowerRight => TextAnchor.LowerRight,
                        _ => TextAnchor.UpperLeft,
                    },
                () => target.unityTextAlign = StyleKeyword.Initial
            );
            Apply(
                value.UnityTextAutoSize,
                item =>
                    target.unityTextAutoSize = item switch
                    {
                        UiTextAutoSize.BestFit fit => new TextAutoSize(
                            TextAutoSizeMode.BestFit,
                            new Length(fit.MinSize),
                            new Length(fit.MaxSize)
                        ),
                        _ => new TextAutoSize(
                            TextAutoSizeMode.None,
                            new Length(10),
                            new Length(100)
                        ),
                    },
                () => target.unityTextAutoSize = StyleKeyword.Initial
            );
            Apply(
                value.UnityTextGenerator,
                item =>
                    target.unityTextGenerator =
                        item == UiTextGenerator.Advanced
                            ? TextGeneratorType.Advanced
                            : TextGeneratorType.Standard,
                () => target.unityTextGenerator = StyleKeyword.Initial
            );
            Apply(
                value.UnityTextOutlineColor,
                item => target.unityTextOutlineColor = ToUnity(item),
                () => target.unityTextOutlineColor = StyleKeyword.Initial
            );
            Apply(
                value.UnityTextOutlineWidth,
                item => target.unityTextOutlineWidth = item,
                () => target.unityTextOutlineWidth = StyleKeyword.Initial
            );
            Apply(
                value.UnityTextOverflowPosition,
                item =>
                    target.unityTextOverflowPosition = item switch
                    {
                        UiTextOverflowPosition.Start => TextOverflowPosition.Start,
                        UiTextOverflowPosition.Middle => TextOverflowPosition.Middle,
                        _ => TextOverflowPosition.End,
                    },
                () => target.unityTextOverflowPosition = StyleKeyword.Initial
            );
            Apply(
                value.WhiteSpace,
                item =>
                    target.whiteSpace = item switch
                    {
                        UiWhiteSpace.NoWrap => WhiteSpace.NoWrap,
                        UiWhiteSpace.Pre => WhiteSpace.Pre,
                        UiWhiteSpace.PreWrap => WhiteSpace.PreWrap,
                        _ => WhiteSpace.Normal,
                    },
                () => target.whiteSpace = StyleKeyword.Initial
            );
            Apply(
                value.WordSpacing,
                item => target.wordSpacing = ToUnity(item),
                () => target.wordSpacing = StyleKeyword.Initial
            );
        }

        private static void Apply(
            UnityEngine.UIElements.TextElement target,
            string? text,
            bool? richText,
            bool? emojiFallback,
            bool? parseEscapes,
            bool? elisionTooltip,
            bool? selectable,
            bool? doubleWord,
            bool? tripleLine,
            bool? selectFocus,
            bool? selectMouseUp
        )
        {
            if (text is not null)
                ((INotifyValueChanged<string>)target).SetValueWithoutNotify(text);
            if (richText is bool rich)
                target.enableRichText = rich;
            if (emojiFallback is bool emoji)
                target.emojiFallbackSupport = emoji;
            if (parseEscapes is bool escapes)
                target.parseEscapeSequences = escapes;
            if (elisionTooltip is bool tooltip)
                target.displayTooltipWhenElided = tooltip;
            ITextSelection selection = target;
            if (selectable is bool canSelect)
                selection.isSelectable = canSelect;
            if (doubleWord is bool word)
                selection.doubleClickSelectsWord = word;
            if (tripleLine is bool line)
                selection.tripleClickSelectsLine = line;
            if (selectFocus is bool focus)
                selection.selectAllOnFocus = focus;
            if (selectMouseUp is bool mouseUp)
                selection.selectAllOnMouseUp = mouseUp;
        }

        private static void Apply<T>(
            UiStyleValue<T>? value,
            System.Action<T> concrete,
            System.Action initial
        )
        {
            if (value is null)
                return;
            if (value.Keyword is UiInlineKeyword.Initial)
                initial();
            else
                concrete(value.Value);
        }

        private static StyleLength ToUnity(UiLength value) =>
            value switch
            {
                UiLength.Px item => new Length(item.Value, LengthUnit.Pixel),
                UiLength.Percent item => new Length(item.Value, LengthUnit.Percent),
                _ => throw new System.InvalidOperationException("Unknown UI length kind."),
            };

        private static UnityEngine.Color ToUnity(Color value) =>
            new((float)value.Red, (float)value.Green, (float)value.Blue, (float)value.Alpha);
    }
}
