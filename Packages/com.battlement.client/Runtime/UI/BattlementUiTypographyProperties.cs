#nullable enable

using UnityEngine;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementUiTypographyProperties
    {
        public static void Apply(UnityEngine.UIElements.TextElement target, UiElement.Label value)
        {
            var defaults = new Label();
            ApplyText(target, value.Text, defaults.text);
            ApplySelectableText(
                target,
                value.EnableRichText,
                value.EmojiFallbackSupport,
                value.ParseEscapeSequences,
                value.DisplayTooltipWhenElided,
                value.Selectable,
                value.DoubleClickSelectsWord,
                value.TripleClickSelectsLine,
                value.SelectAllOnFocus,
                value.SelectAllOnMouseUp,
                defaults
            );
        }

        public static void Apply(
            UnityEngine.UIElements.TextElement target,
            UiElement.TextElement value
        )
        {
            var defaults = new UnityEngine.UIElements.TextElement();
            ApplyText(target, value.Text, defaults.text);
            ApplySelectableText(
                target,
                value.EnableRichText,
                value.EmojiFallbackSupport,
                value.ParseEscapeSequences,
                value.DisplayTooltipWhenElided,
                value.Selectable,
                value.DoubleClickSelectsWord,
                value.TripleClickSelectsLine,
                value.SelectAllOnFocus,
                value.SelectAllOnMouseUp,
                defaults
            );
        }

        public static void Apply(UnityEngine.UIElements.Button target, UiElement.Button value)
        {
            var defaults = new UnityEngine.UIElements.Button();
            ApplyCaptionText(target, value.Text, defaults.text);
            ApplyCaption(
                target,
                value.EnableRichText,
                value.EmojiFallbackSupport,
                value.ParseEscapeSequences,
                value.DisplayTooltipWhenElided,
                defaults
            );
        }

        public static void Apply(
            UnityEngine.UIElements.TextElement target,
            UiElement.RepeatButton value
        )
        {
            var defaults = new UnityEngine.UIElements.RepeatButton();
            ApplyCaptionText(target, value.Text, defaults.text);
            ApplyCaption(
                target,
                value.EnableRichText,
                value.EmojiFallbackSupport,
                value.ParseEscapeSequences,
                value.DisplayTooltipWhenElided,
                defaults
            );
        }

        public static void Apply(
            UnityEngine.UIElements.TextElement target,
            UiElement.PopupWindow value
        )
        {
            var defaults = new PopupWindow();
            ApplyText(target, value.Text, defaults.text);
            ApplyProperty(
                value.EnableRichText,
                item => target.enableRichText = item,
                defaults.enableRichText
            );
            ApplyProperty(
                value.EmojiFallbackSupport,
                item => target.emojiFallbackSupport = item,
                defaults.emojiFallbackSupport
            );
            ApplyProperty(
                value.ParseEscapeSequences,
                item => target.parseEscapeSequences = item,
                defaults.parseEscapeSequences
            );
            ApplyProperty(
                value.DisplayTooltipWhenElided,
                item => target.displayTooltipWhenElided = item,
                defaults.displayTooltipWhenElided
            );
            ITextSelection selection = target;
            ITextSelection defaultSelection = defaults;
            ApplyProperty(
                value.Selectable,
                item => selection.isSelectable = item,
                defaultSelection.isSelectable
            );
            ApplyProperty(
                value.DoubleClickSelectsWord,
                item => selection.doubleClickSelectsWord = item,
                defaultSelection.doubleClickSelectsWord
            );
            ApplyProperty(
                value.TripleClickSelectsLine,
                item => selection.tripleClickSelectsLine = item,
                defaultSelection.tripleClickSelectsLine
            );
            ApplyProperty(
                value.SelectAllOnFocus,
                item => selection.selectAllOnFocus = item,
                defaultSelection.selectAllOnFocus
            );
            ApplyProperty(
                value.SelectAllOnMouseUp,
                item => selection.selectAllOnMouseUp = item,
                defaultSelection.selectAllOnMouseUp
            );
        }

        public static void ApplyStyle(
            VisualElement element,
            UiStyle value,
            BattlementUiStyleFontProperties.FontLeases? fonts
        )
        {
            IStyle target = element.style;
            Apply(
                value.FontSize,
                item => target.fontSize = ToUnity(item),
                keyword => target.fontSize = keyword
            );
            Apply(
                value.LetterSpacing,
                item => BattlementTextSpacing.Set(element, ToUnity(item)),
                keyword => BattlementTextSpacing.Set(element, keyword)
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
            Prop<bool> richText,
            Prop<bool> emojiFallback,
            Prop<bool> parseEscapes,
            Prop<bool> elisionTooltip,
            Prop<bool> selectable,
            Prop<bool> doubleWord,
            Prop<bool> tripleLine,
            Prop<bool> selectFocus,
            Prop<bool> selectMouseUp,
            UnityEngine.UIElements.TextElement defaults
        )
        {
            ApplyCaption(target, richText, emojiFallback, parseEscapes, elisionTooltip, defaults);
            ITextSelection selection = target;
            ITextSelection defaultSelection = defaults;
            ApplyProperty(
                selectable,
                item => selection.isSelectable = item,
                defaultSelection.isSelectable
            );
            ApplyProperty(
                doubleWord,
                item => selection.doubleClickSelectsWord = item,
                defaultSelection.doubleClickSelectsWord
            );
            ApplyProperty(
                tripleLine,
                item => selection.tripleClickSelectsLine = item,
                defaultSelection.tripleClickSelectsLine
            );
            ApplyProperty(
                selectFocus,
                item => selection.selectAllOnFocus = item,
                defaultSelection.selectAllOnFocus
            );
            ApplyProperty(
                selectMouseUp,
                item => selection.selectAllOnMouseUp = item,
                defaultSelection.selectAllOnMouseUp
            );
        }

        private static void ApplyCaption(
            UnityEngine.UIElements.TextElement target,
            Prop<bool> richText,
            Prop<bool> emojiFallback,
            Prop<bool> parseEscapes,
            Prop<bool> elisionTooltip,
            UnityEngine.UIElements.TextElement defaults
        )
        {
            ApplyProperty(richText, item => target.enableRichText = item, defaults.enableRichText);
            ApplyProperty(
                emojiFallback,
                item => target.emojiFallbackSupport = item,
                defaults.emojiFallbackSupport
            );
            ApplyProperty(
                parseEscapes,
                item => target.parseEscapeSequences = item,
                defaults.parseEscapeSequences
            );
            ApplyProperty(
                elisionTooltip,
                item => target.displayTooltipWhenElided = item,
                defaults.displayTooltipWhenElided
            );
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

        private static void ApplyCaptionText(
            UnityEngine.UIElements.TextElement target,
            Prop<string> value,
            string constructorDefault
        )
        {
            if (value.IsSet)
                target.text = value.Value;
            else if (value.IsReset)
                target.text = constructorDefault;
        }

        private static void ApplyProperty<T>(Prop<T> value, System.Action<T> set, T resetValue)
        {
            if (value.IsSet)
                set(value.Value);
            else if (value.IsReset)
                set(resetValue);
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
