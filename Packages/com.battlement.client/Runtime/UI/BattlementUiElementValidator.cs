#nullable enable

using System;
using System.Collections.Generic;

namespace Battlement.UI
{
    internal static class BattlementUiElementValidator
    {
        public static void Validate(UiElement element, bool allowUsageHints)
        {
            ValidateString(element.Name, allowEmpty: true, "UI name");
            var classes = new HashSet<string>(StringComparer.Ordinal);
            foreach (string className in element.Classes ?? Array.Empty<string>())
            {
                ValidateString(className, allowEmpty: false, "UI class");
                if (!classes.Add(className))
                    throw Failure(CoreErrorCode.InvalidProperty, "UI classes must be unique.");
            }
            ValidateUnique(element.Events, "UI event subscriptions must be unique.");
            if (!allowUsageHints && element.UsageHints is not null)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "UI usage hints can only be assigned during creation."
                );
            ValidateUnique(element.UsageHints, "UI usage hints must be unique.");
            UiStyleValidator.Validate(
                element.Style,
                message => Failure(CoreErrorCode.InvalidProperty, message)
            );
            switch (element)
            {
                case UiElement.Label label:
                    ValidateString(label.Text, allowEmpty: true, "label text");
                    break;
                case UiElement.TextElement text:
                    ValidateString(text.Text, allowEmpty: true, "text element text");
                    break;
                case UiElement.Button button:
                    ValidateString(button.Text, allowEmpty: true, "button text");
                    break;
                case UiElement.RepeatButton repeat:
                    ValidateString(repeat.Text, allowEmpty: true, "repeat button text");
                    if (repeat.IntervalMs == 0)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "A repeat button interval must be positive."
                        );
                    break;
                case UiElement.GroupBox group:
                    ValidateString(group.Text, allowEmpty: true, "group box text");
                    break;
                case UiElement.PopupWindow popup:
                    ValidateString(popup.Text, allowEmpty: true, "popup window text");
                    break;
                case UiElement.Image image:
                    BattlementUiImageProperties.Validate(image);
                    break;
                case UiElement.ScrollView scroll:
                    ValidateFinite(
                        scroll.ScrollOffset?.X,
                        scroll.ScrollOffset?.Y,
                        scroll.HorizontalPageSize,
                        scroll.VerticalPageSize,
                        scroll.MouseWheelScrollSize,
                        scroll.ScrollDecelerationRate,
                        scroll.Elasticity
                    );
                    break;
                case UiElement.Scroller scroller:
                    ValidateFinite(scroller.LowValue, scroller.HighValue, scroller.Value);
                    if (scroller.LowValue > scroller.HighValue)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "Scroller limits are reversed."
                        );
                    break;
                case UiElement.Tab tab:
                    ValidateString(tab.Text, allowEmpty: true, "tab text");
                    break;
                default:
                    break;
            }
        }

        private static void ValidateFinite(params float?[] values)
        {
            foreach (float? value in values)
            {
                if (value is float number && (float.IsNaN(number) || float.IsInfinity(number)))
                    throw Failure(
                        CoreErrorCode.InvalidProperty,
                        "UI numeric values must be finite."
                    );
            }
        }

        private static void ValidateUnique<T>(IReadOnlyList<T>? values, string message)
        {
            IReadOnlyList<T> items = values ?? Array.Empty<T>();
            if (items.Count != new HashSet<T>(items).Count)
                throw Failure(CoreErrorCode.InvalidProperty, message);
        }

        private static void ValidateString(string? value, bool allowEmpty, string description)
        {
            if (value is null)
                return;
            if (!allowEmpty && value.Length == 0)
                throw Failure(CoreErrorCode.InvalidProperty, $"{description} cannot be empty.");
            if (System.Text.Encoding.UTF8.GetByteCount(value) > 65_536)
                throw Failure(CoreErrorCode.LimitExceeded, $"{description} is too long.");
        }

        private static BattlementUiException Failure(CoreErrorCode code, string message) =>
            new(code, message);
    }
}
