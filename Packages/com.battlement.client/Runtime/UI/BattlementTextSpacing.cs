#nullable enable

using System.Runtime.CompilerServices;
using UnityEngine.UIElements;

namespace Battlement.UI
{
    internal static class BattlementTextSpacing
    {
        private static readonly ConditionalWeakTable<VisualElement, Spacing> authored = new();
        private static readonly ConditionalWeakTable<TextElement, Spacing> converted = new();

        public static void Set(VisualElement target, StyleLength value)
        {
            if (value.keyword == StyleKeyword.Null)
                authored.Remove(target);
            else if (authored.TryGetValue(target, out Spacing current))
                current.Value = value;
            else
                authored.Add(target, new Spacing(value));
            if (target is TextElement text && converted.TryGetValue(text, out Spacing previous))
                previous.Value = value;
            target.style.letterSpacing = value;
        }

        public static StyleLength ReadStyle(VisualElement target)
        {
            if (authored.TryGetValue(target, out Spacing value))
                return value.Value;
            if (target is TextElement text && converted.TryGetValue(text, out Spacing original))
                return original.Value;
            return target.style.letterSpacing;
        }

        public static float ReadPixels(VisualElement target)
        {
            for (VisualElement? ancestor = target; ancestor is not null; ancestor = ancestor.parent)
            {
                if (authored.TryGetValue(ancestor, out Spacing value))
                    return Pixels(value.Value, target.resolvedStyle.fontSize);
            }
            return target.resolvedStyle.letterSpacing;
        }

        public static void Refresh(VisualElement root) => Refresh(root, null);

        private static void Refresh(VisualElement element, StyleLength? inherited)
        {
            StyleLength? value = authored.TryGetValue(element, out Spacing own)
                ? own.Value
                : inherited;
            if (element is TextElement text)
                RefreshText(text, value);
            for (int index = 0; index < element.hierarchy.childCount; index++)
                Refresh(element.hierarchy[index], value);
        }

        private static void RefreshText(TextElement text, StyleLength? value)
        {
            if (!value.HasValue)
            {
                if (converted.TryGetValue(text, out Spacing original))
                {
                    text.style.letterSpacing = original.Value;
                    converted.Remove(text);
                }
                return;
            }
            float size = text.resolvedStyle.fontSize;
            if (!float.IsFinite(size) || size <= 0)
                return;
            if (!converted.TryGetValue(text, out _))
                converted.Add(text, new Spacing(text.style.letterSpacing));
            float pixels = Pixels(value.Value, size);
            StyleLength next = pixels * 100 / size;
            if (text.style.letterSpacing != next)
                text.style.letterSpacing = next;
        }

        private static float Pixels(StyleLength value, float size)
        {
            if (value.keyword != StyleKeyword.Undefined)
                return 0;
            return value.value.unit == LengthUnit.Percent
                ? size * value.value.value / 100
                : value.value.value;
        }

        private sealed class Spacing
        {
            public Spacing(StyleLength value) => Value = value;

            public StyleLength Value { get; set; }
        }
    }
}
