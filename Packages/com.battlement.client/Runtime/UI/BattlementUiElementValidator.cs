#nullable enable

using System;
using System.Collections.Generic;
using System.Linq;

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
            ValidateUnique(
                element.EventSubscriptions,
                "UI routed event subscriptions must be unique."
            );
            foreach (
                UiEventSubscription subscription in element.EventSubscriptions
                    ?? Array.Empty<UiEventSubscription>()
            )
            {
                if (subscription.Phase != UiEventPhase.Target && !Propagates(subscription.Kind))
                    throw Failure(
                        CoreErrorCode.InvalidProperty,
                        "Target-only UI events cannot use ancestor phases."
                    );
                if (
                    subscription.Phase == UiEventPhase.Target
                    && (element.Events?.Contains(subscription.Kind) ?? false)
                )
                    throw Failure(
                        CoreErrorCode.InvalidProperty,
                        "UI event subscriptions must be unique across shorthand and routed values."
                    );
            }
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
            ValidateParts(element);
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
                    ValidateString(range.Label, allowEmpty: true, "range slider label");
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

        private static void ValidateParts(UiElement element)
        {
            IReadOnlyList<UiPartStyle>? declarations = element switch
            {
                UiElement.Button value => value.Parts,
                UiElement.GroupBox value => value.Parts,
                UiElement.PopupWindow value => value.Parts,
                UiElement.Toggle value => value.Parts,
                UiElement.RadioButton value => value.Parts,
                UiElement.DropdownField value => value.Parts,
                UiElement.ProgressBar value => value.Parts,
                UiElement.ScrollView value => value.Parts,
                UiElement.Scroller value => value.Parts,
                UiElement.Tab value => value.Parts,
                UiElement.TabView value => value.Parts,
                UiElement.TextField value => value.Parts,
                UiElement.RadioButtonGroup value => value.Parts,
                UiElement.ToggleButtonGroup value => value.Parts,
                UiElement.Slider value => value.Parts,
                UiElement.SliderInt value => value.Parts,
                UiElement.MinMaxSlider value => value.Parts,
                _ => null,
            };
            var keys = new HashSet<(UiPart, uint?)>();
            foreach (UiPartStyle declaration in declarations ?? Array.Empty<UiPartStyle>())
            {
                bool indexed = IsIndexed(declaration.Part);
                bool availableIndex =
                    declaration.Index is not uint index
                    || element is not UiElement.RadioButtonGroup { Choices: { } choices }
                    || index < choices.Count;
                if (
                    !keys.Add((declaration.Part, declaration.Index))
                    || !PartBelongsTo(element, declaration.Part)
                    || indexed != declaration.Index.HasValue
                    || !availableIndex
                )
                    throw Failure(
                        CoreErrorCode.InvalidProperty,
                        "UI part keys must be unique, correctly indexed, "
                            + "and belong to their control."
                    );
                if (!ConditionalPartExists(element, declaration.Part))
                    throw Failure(
                        CoreErrorCode.InvalidProperty,
                        $"The authored state does not contain UI part {declaration.Part}."
                    );
                UiStyleValidator.Validate(
                    declaration.Style,
                    message => Failure(CoreErrorCode.InvalidProperty, message)
                );
            }
        }

        private static bool IsIndexed(UiPart part) =>
            part
                is UiPart.RadioButtonGroupOption
                    or UiPart.RadioButtonGroupOptionCheckmarkBackground
                    or UiPart.RadioButtonGroupOptionCheckmark
                    or UiPart.RadioButtonGroupOptionText;

        private static bool ConditionalPartExists(UiElement element, UiPart part) =>
            (element, part) switch
            {
                (UiElement.GroupBox { Text: "" }, UiPart.GroupBoxTitle) => false,
                (UiElement.Tab { Closeable: false }, UiPart.TabCloseButton) => false,
                (
                    UiElement.TextField { Multiline: false },
                    UiPart.TextFieldMultilineScrollView
                        or UiPart.TextFieldVerticalScroller
                        or UiPart.TextFieldVerticalSlider
                        or UiPart.TextFieldVerticalLowButton
                        or UiPart.TextFieldVerticalHighButton
                        or UiPart.TextFieldVerticalTrack
                        or UiPart.TextFieldVerticalDragger
                        or UiPart.TextFieldVerticalDraggerBorder
                ) => false,
                (UiElement.Slider { Fill: false }, UiPart.SliderFill) => false,
                (UiElement.Slider { ShowInputField: false }, UiPart.SliderTextInput) => false,
                (UiElement.SliderInt { Fill: false }, UiPart.SliderIntFill) => false,
                (UiElement.SliderInt { ShowInputField: false }, UiPart.SliderIntTextInput) => false,
                _ => true,
            };

        private static bool PartBelongsTo(UiElement element, UiPart part) =>
            (element, part) switch
            {
                (UiElement.Button, UiPart.ButtonIcon) => true,
                (UiElement.GroupBox, UiPart.GroupBoxTitle) => true,
                (UiElement.PopupWindow, UiPart.PopupWindowContentContainer) => true,
                (
                    UiElement.Toggle,
                    UiPart.ToggleLabel
                        or UiPart.ToggleInput
                        or UiPart.ToggleCheckmark
                        or UiPart.ToggleText
                ) => true,
                (
                    UiElement.RadioButton,
                    UiPart.RadioButtonLabel
                        or UiPart.RadioButtonInput
                        or UiPart.RadioButtonCheckmarkBackground
                        or UiPart.RadioButtonCheckmark
                        or UiPart.RadioButtonText
                ) => true,
                (
                    UiElement.DropdownField,
                    UiPart.DropdownFieldLabel
                        or UiPart.DropdownFieldInput
                        or UiPart.DropdownFieldText
                        or UiPart.DropdownFieldArrow
                ) => true,
                (
                    UiElement.ProgressBar,
                    UiPart.ProgressBarContainer
                        or UiPart.ProgressBarBackground
                        or UiPart.ProgressBarProgress
                        or UiPart.ProgressBarTitleContainer
                        or UiPart.ProgressBarTitle
                ) => true,
                (
                    UiElement.ScrollView,
                    UiPart.ScrollViewContentAndVerticalScrollContainer
                        or UiPart.ScrollViewViewport
                        or UiPart.ScrollViewContentContainer
                        or UiPart.ScrollViewHorizontalScroller
                        or UiPart.ScrollViewHorizontalSlider
                        or UiPart.ScrollViewHorizontalLowButton
                        or UiPart.ScrollViewHorizontalHighButton
                        or UiPart.ScrollViewHorizontalTrack
                        or UiPart.ScrollViewHorizontalDragger
                        or UiPart.ScrollViewHorizontalDraggerBorder
                        or UiPart.ScrollViewVerticalScroller
                        or UiPart.ScrollViewVerticalSlider
                        or UiPart.ScrollViewVerticalLowButton
                        or UiPart.ScrollViewVerticalHighButton
                        or UiPart.ScrollViewVerticalTrack
                        or UiPart.ScrollViewVerticalDragger
                        or UiPart.ScrollViewVerticalDraggerBorder
                ) => true,
                (
                    UiElement.Scroller,
                    UiPart.ScrollerSlider
                        or UiPart.ScrollerLowButton
                        or UiPart.ScrollerHighButton
                        or UiPart.ScrollerTrack
                        or UiPart.ScrollerDragger
                        or UiPart.ScrollerDraggerBorder
                ) => true,
                (
                    UiElement.Tab,
                    UiPart.TabHeader
                        or UiPart.TabLabel
                        or UiPart.TabIcon
                        or UiPart.TabUnderline
                        or UiPart.TabCloseButton
                        or UiPart.TabDragHandle
                        or UiPart.TabDragHandleLeadingBar
                        or UiPart.TabDragHandleTrailingBar
                        or UiPart.TabContentContainer
                ) => true,
                (
                    UiElement.TabView,
                    UiPart.TabViewContentViewport
                        or UiPart.TabViewHeaderContainer
                        or UiPart.TabViewContentContainer
                        or UiPart.TabViewPreviousButton
                        or UiPart.TabViewNextButton
                ) => true,
                (
                    UiElement.TextField,
                    UiPart.TextFieldLabel
                        or UiPart.TextFieldInput
                        or UiPart.TextFieldTextElement
                        or UiPart.TextFieldMultilineScrollView
                        or UiPart.TextFieldVerticalScroller
                        or UiPart.TextFieldVerticalSlider
                        or UiPart.TextFieldVerticalLowButton
                        or UiPart.TextFieldVerticalHighButton
                        or UiPart.TextFieldVerticalTrack
                        or UiPart.TextFieldVerticalDragger
                        or UiPart.TextFieldVerticalDraggerBorder
                ) => true,
                (
                    UiElement.RadioButtonGroup,
                    UiPart.RadioButtonGroupLabel
                        or UiPart.RadioButtonGroupInput
                        or UiPart.RadioButtonGroupChoicesContainer
                        or UiPart.RadioButtonGroupContentContainer
                        or UiPart.RadioButtonGroupAllOptions
                        or UiPart.RadioButtonGroupOption
                        or UiPart.RadioButtonGroupOptionCheckmarkBackground
                        or UiPart.RadioButtonGroupOptionCheckmark
                        or UiPart.RadioButtonGroupOptionText
                ) => true,
                (
                    UiElement.ToggleButtonGroup,
                    UiPart.ToggleButtonGroupLabel
                        or UiPart.ToggleButtonGroupInput
                ) => true,
                (
                    UiElement.Slider,
                    UiPart.SliderLabel
                        or UiPart.SliderInput
                        or UiPart.SliderTrack
                        or UiPart.SliderDragger
                        or UiPart.SliderDraggerBorder
                        or UiPart.SliderFill
                        or UiPart.SliderTextInput
                ) => true,
                (
                    UiElement.SliderInt,
                    UiPart.SliderIntLabel
                        or UiPart.SliderIntInput
                        or UiPart.SliderIntTrack
                        or UiPart.SliderIntDragger
                        or UiPart.SliderIntDraggerBorder
                        or UiPart.SliderIntFill
                        or UiPart.SliderIntTextInput
                ) => true,
                (
                    UiElement.MinMaxSlider,
                    UiPart.MinMaxSliderLabel
                        or UiPart.MinMaxSliderInput
                        or UiPart.MinMaxSliderTrack
                        or UiPart.MinMaxSliderMinimumThumb
                        or UiPart.MinMaxSliderMaximumThumb
                        or UiPart.MinMaxSliderRangeDragger
                ) => true,
                _ => false,
            };

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

        private static bool Propagates(UiEventKind kind) =>
            kind
                is UiEventKind.PointerDown
                    or UiEventKind.PointerMove
                    or UiEventKind.PointerUp
                    or UiEventKind.PointerCancel
                    or UiEventKind.Click
                    or UiEventKind.PointerOver
                    or UiEventKind.PointerOut
                    or UiEventKind.Wheel
                    or UiEventKind.PointerCapture
                    or UiEventKind.PointerCaptureOut
                    or UiEventKind.FocusIn
                    or UiEventKind.FocusOut;

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
