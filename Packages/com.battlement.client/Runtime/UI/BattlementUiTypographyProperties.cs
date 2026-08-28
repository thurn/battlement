#nullable enable

using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementUiTypographyProperties
    {
        public static void Apply(UnityEngine.UIElements.TextElement target, UiElement.Label value)
        {
            ApplyText(target, value.Text, new Label().text);
            ApplySelectableText(
                target,
                null,
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
        }

        public static void Apply(
            UnityEngine.UIElements.TextElement target,
            UiElement.TextElement value
        )
        {
            ApplyText(target, value.Text, new UnityEngine.UIElements.TextElement().text);
            ApplySelectableText(
                target,
                null,
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
        }

        public static void Apply(UnityEngine.UIElements.Button target, UiElement.Button value)
        {
            if (value.Text.IsSet)
                target.text = value.Text.Value;
            else if (value.Text.IsReset)
                target.text = new UnityEngine.UIElements.Button().text;
            ApplyCaption(
                target,
                null,
                value.EnableRichText,
                value.EmojiFallbackSupport,
                value.ParseEscapeSequences,
                value.DisplayTooltipWhenElided
            );
        }

        public static void Apply(
            UnityEngine.UIElements.TextElement target,
            UiElement.RepeatButton value
        ) =>
            ApplyCaption(
                target,
                value.Text,
                value.EnableRichText,
                value.EmojiFallbackSupport,
                value.ParseEscapeSequences,
                value.DisplayTooltipWhenElided
            );

        public static void Apply(
            UnityEngine.UIElements.TextElement target,
            UiElement.PopupWindow value
        ) =>
            ApplySelectableText(
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
                keyword => target.fontSize = keyword
            );
            Apply(
                value.LetterSpacing,
                item => target.letterSpacing = ToUnity(item),
                keyword => target.letterSpacing = keyword
            );
            Apply(
                value.TextOverflow,
                item =>
                    target.textOverflow =
                        item == UiTextOverflow.Clip
                            ? UnityEngine.UIElements.TextOverflow.Clip
                            : UnityEngine.UIElements.TextOverflow.Ellipsis,
                keyword => target.textOverflow = keyword
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
                keyword => target.textShadow = keyword
            );
            Apply(
                value.UnityEditorTextRenderingMode,
                item =>
                    target.unityEditorTextRenderingMode =
                        item == UiEditorTextRenderingMode.Sdf
                            ? EditorTextRenderingMode.SDF
                            : EditorTextRenderingMode.Bitmap,
                keyword => target.unityEditorTextRenderingMode = keyword
            );
            Apply(
                value.UnityFontDefinition,
                _ =>
                    target.unityFontDefinition = new FontDefinition
                    {
                        fontAsset = (UnityEngine.TextCore.Text.FontAsset)
                            fonts!.FontDefinition!.Value,
                    },
                keyword => target.unityFontDefinition = keyword
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
                keyword => target.unityFontStyleAndWeight = keyword
            );
            Apply(
                value.UnityParagraphSpacing,
                item => target.unityParagraphSpacing = ToUnity(item),
                keyword => target.unityParagraphSpacing = keyword
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
                keyword => target.unityTextAlign = keyword
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
                keyword => target.unityTextAutoSize = keyword
            );
            Apply(
                value.UnityTextGenerator,
                item =>
                    target.unityTextGenerator =
                        item == UiTextGenerator.Advanced
                            ? TextGeneratorType.Advanced
                            : TextGeneratorType.Standard,
                keyword => target.unityTextGenerator = keyword
            );
            Apply(
                value.UnityTextOutlineColor,
                item => target.unityTextOutlineColor = ToUnity(item),
                keyword => target.unityTextOutlineColor = keyword
            );
            Apply(
                value.UnityTextOutlineWidth,
                item => target.unityTextOutlineWidth = item,
                keyword => target.unityTextOutlineWidth = keyword
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
                keyword => target.unityTextOverflowPosition = keyword
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
                keyword => target.whiteSpace = keyword
            );
            Apply(
                value.WordSpacing,
                item => target.wordSpacing = ToUnity(item),
                keyword => target.wordSpacing = keyword
            );
        }

        private static void ApplySelectableText(
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
            ApplyCaption(target, text, richText, emojiFallback, parseEscapes, elisionTooltip);
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

        private static void ApplyCaption(
            UnityEngine.UIElements.TextElement target,
            string? text,
            bool? richText,
            bool? emojiFallback,
            bool? parseEscapes,
            bool? elisionTooltip
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
        }

        private static void ApplyText(
            UnityEngine.UIElements.TextElement target,
            Prop<string> value,
            string constructorDefault
        )
        {
            if (value.IsSet)
                ((INotifyValueChanged<string>)target).SetValueWithoutNotify(value.Value);
            else if (value.IsReset)
                ((INotifyValueChanged<string>)target).SetValueWithoutNotify(constructorDefault);
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

        private static void Apply<T>(
            Prop<UiStyleValue<T>> value,
            System.Action<T> concrete,
            System.Action<StyleKeyword> keyword
        )
        {
            if (value.IsUnset)
                return;
            if (value.IsReset)
            {
                keyword(StyleKeyword.Null);
                return;
            }
            Apply(value.Value, concrete, () => keyword(StyleKeyword.Initial));
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
