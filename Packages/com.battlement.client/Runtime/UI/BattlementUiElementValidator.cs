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
                case UiElement.Toggle toggle:
                    ValidateString(toggle.Label, allowEmpty: true, "toggle label");
                    ValidateString(toggle.Text, allowEmpty: true, "toggle text");
                    break;
                case UiElement.RadioButton radio:
                    ValidateString(radio.Label, allowEmpty: true, "radio button label");
                    ValidateString(radio.Text, allowEmpty: true, "radio button text");
                    break;
                case UiElement.RadioButtonGroup radioGroup:
                    ValidateString(radioGroup.Label, allowEmpty: true, "radio group label");
                    foreach (string choice in radioGroup.Choices ?? Array.Empty<string>())
                        ValidateString(choice, allowEmpty: true, "radio choice");
                    break;
                case UiElement.ToggleButtonGroup toggleGroup:
                    ValidateString(toggleGroup.Label, allowEmpty: true, "toggle group label");
                    ValidateSorted(toggleGroup.SelectedIndices);
                    break;
                case UiElement.DropdownField dropdown:
                    ValidateString(dropdown.Label, allowEmpty: true, "dropdown label");
                    var choices = new HashSet<string>(StringComparer.Ordinal);
                    foreach (string choice in dropdown.Choices ?? Array.Empty<string>())
                    {
                        ValidateString(choice, allowEmpty: false, "dropdown choice");
                        if (!choices.Add(choice))
                            throw Failure(
                                CoreErrorCode.InvalidProperty,
                                "Dropdown choices must be unique."
                            );
                    }
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
                case UiElement.Slider slider:
                    ValidateString(slider.Label, allowEmpty: true, "slider label");
                    ValidateSparseRange(
                        slider.LowValue,
                        slider.HighValue,
                        slider.Value,
                        slider.PageSize
                    );
                    break;
                case UiElement.SliderInt slider:
                    ValidateString(slider.Label, allowEmpty: true, "slider label");
                    ValidateSparseRange(
                        slider.LowValue,
                        slider.HighValue,
                        slider.Value,
                        slider.PageSize
                    );
                    break;
                case UiElement.MinMaxSlider range:
                    float? lowLimit = range.LowLimit is LowerLimit.Inclusive low ? low.Value : null;
                    float? highLimit = range.HighLimit is UpperLimit.Inclusive high
                        ? high.Value
                        : null;
                    ValidateFinite(range.MinValue, range.MaxValue, lowLimit, highLimit);
                    if (range.MinValue > range.MaxValue || lowLimit > highLimit)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "MinMaxSlider range is invalid."
                        );
                    if (range.MinValue < lowLimit || range.MaxValue > highLimit)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "MinMaxSlider range is invalid."
                        );
                    break;
                case UiElement.ProgressBar progress:
                    ValidateString(progress.Title, allowEmpty: true, "progress title");
                    ValidateFinite(progress.LowValue, progress.HighValue, progress.Value);
                    if (progress.LowValue > progress.HighValue)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "ProgressBar range is invalid."
                        );
                    if (progress.Value < progress.LowValue || progress.Value > progress.HighValue)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "ProgressBar range is invalid."
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

        private static void ValidateRange(float low, float high, float selected, float pageSize)
        {
            ValidateFinite(low, high, selected, pageSize);
            if (low > high || selected < low || selected > high || pageSize < 0)
                throw Failure(CoreErrorCode.InvalidProperty, "Slider range is invalid.");
        }

        private static void ValidateRange(int low, int high, int selected, float pageSize)
        {
            ValidateFinite(pageSize);
            if (low > high || selected < low || selected > high || pageSize < 0)
                throw Failure(CoreErrorCode.InvalidProperty, "Slider range is invalid.");
        }

        private static void ValidateSparseRange(
            float? low,
            float? high,
            float? selected,
            float? pageSize
        )
        {
            ValidateFinite(low, high, selected, pageSize);
            if (pageSize < 0 || low > high)
                throw Failure(CoreErrorCode.InvalidProperty, "Slider range is invalid.");
            if (
                low is float minimum
                && high is float maximum
                && selected is float value
                && (value < minimum || value > maximum)
            )
                throw Failure(CoreErrorCode.InvalidProperty, "Slider range is invalid.");
        }

        private static void ValidateSparseRange(int? low, int? high, int? selected, float? pageSize)
        {
            ValidateFinite(pageSize);
            if (pageSize < 0 || low > high)
                throw Failure(CoreErrorCode.InvalidProperty, "Slider range is invalid.");
            if (
                low is int minimum
                && high is int maximum
                && selected is int value
                && (value < minimum || value > maximum)
            )
                throw Failure(CoreErrorCode.InvalidProperty, "Slider range is invalid.");
        }

        private static void ValidateSorted(IReadOnlyList<uint>? values)
        {
            uint? previous = null;
            foreach (uint value in values ?? Array.Empty<uint>())
            {
                if (previous is uint last && last >= value)
                    throw Failure(
                        CoreErrorCode.InvalidProperty,
                        "Selection indices must be unique and sorted."
                    );
                previous = value;
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
