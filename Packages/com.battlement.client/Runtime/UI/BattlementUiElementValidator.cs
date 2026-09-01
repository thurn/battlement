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
            ValidateString(SetValue(element.Name), allowEmpty: true, "UI name");
            var classes = new HashSet<string>(StringComparer.Ordinal);
            foreach (string className in SetValues(element.Classes))
            {
                ValidateString(className, allowEmpty: false, "UI class");
                if (!classes.Add(className))
                    throw Failure(CoreErrorCode.InvalidProperty, "UI classes must be unique.");
            }
            ValidateUnique(SetValue(element.Events), "UI event subscriptions must be unique.");
            ValidateUnique(
                SetValue(element.EventSubscriptions),
                "UI routed event subscriptions must be unique."
            );
            foreach (
                UiEventSubscription subscription in SetValue(element.EventSubscriptions)
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
                    && (SetValue(element.Events)?.Contains(subscription.Kind) ?? false)
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
            if (element.Motion.IsSet)
                BattlementMotionValidator.Validate(element.Motion.Value);
            ValidateLayout(element);
            ValidateParts(element);
            switch (element)
            {
                case UiElement.Label label:
                    ValidateString(SetValue(label.Text), allowEmpty: true, "label text");
                    break;
                case UiElement.TextElement text:
                    ValidateString(SetValue(text.Text), allowEmpty: true, "text element text");
                    break;
                case UiElement.Toggle toggle:
                    ValidateString(SetValue(toggle.Label), allowEmpty: true, "toggle label");
                    ValidateString(SetValue(toggle.Text), allowEmpty: true, "toggle text");
                    break;
                case UiElement.RadioButton radio:
                    ValidateString(SetValue(radio.Label), allowEmpty: true, "radio button label");
                    ValidateString(SetValue(radio.Text), allowEmpty: true, "radio button text");
                    break;
                case UiElement.RadioButtonGroup radioGroup:
                    ValidateString(
                        SetValue(radioGroup.Label),
                        allowEmpty: true,
                        "radio group label"
                    );
                    foreach (string choice in SetValues(radioGroup.Choices))
                        ValidateString(choice, allowEmpty: true, "radio choice");
                    break;
                case UiElement.ToggleButtonGroup toggleGroup:
                    ValidateString(
                        SetValue(toggleGroup.Label),
                        allowEmpty: true,
                        "toggle group label"
                    );
                    ValidateSorted(SetValue(toggleGroup.SelectedIndices));
                    break;
                case UiElement.DropdownField dropdown:
                    ValidateString(SetValue(dropdown.Label), allowEmpty: true, "dropdown label");
                    var choices = new HashSet<string>(StringComparer.Ordinal);
                    foreach (string choice in SetValues(dropdown.Choices))
                    {
                        ValidateString(choice, allowEmpty: false, "dropdown choice");
                        if (!choices.Add(choice))
                            throw Failure(
                                CoreErrorCode.InvalidProperty,
                                "Dropdown choices must be unique."
                            );
                    }
                    break;
                case UiElement.TextField field:
                    ValidateString(SetValue(field.Label), allowEmpty: true, "text field label");
                    ValidateString(SetValue(field.Value), allowEmpty: true, "text field value");
                    ValidateString(
                        SetValue(field.Placeholder),
                        allowEmpty: true,
                        "text field placeholder"
                    );
                    break;
                case UiElement.Button button:
                    ValidateString(SetValue(button.Text), allowEmpty: true, "button text");
                    break;
                case UiElement.RepeatButton repeat:
                    ValidateString(SetValue(repeat.Text), allowEmpty: true, "repeat button text");
                    if (SetStructValue(repeat.IntervalMs) == 0)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "A repeat button interval must be positive."
                        );
                    break;
                case UiElement.GroupBox group:
                    ValidateString(SetValue(group.Text), allowEmpty: true, "group box text");
                    break;
                case UiElement.PopupWindow popup:
                    ValidateString(SetValue(popup.Text), allowEmpty: true, "popup window text");
                    break;
                case UiElement.Image image:
                    BattlementUiImageProperties.Validate(image);
                    break;
                case UiElement.ScrollView scroll:
                    Battlement.Vector? offset = SetValue(scroll.ScrollOffset);
                    ValidateFinite(
                        offset?.X,
                        offset?.Y,
                        SetStructValue(scroll.HorizontalPageSize),
                        SetStructValue(scroll.VerticalPageSize),
                        SetStructValue(scroll.MouseWheelScrollSize),
                        SetStructValue(scroll.ScrollDecelerationRate),
                        SetStructValue(scroll.Elasticity)
                    );
                    break;
                case UiElement.Scroller scroller:
                    float? scrollerLow = SetStructValue(scroller.LowValue);
                    float? scrollerHigh = SetStructValue(scroller.HighValue);
                    ValidateFinite(scrollerLow, scrollerHigh, SetStructValue(scroller.Value));
                    if (scrollerLow > scrollerHigh)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "Scroller limits are reversed."
                        );
                    break;
                case UiElement.Slider slider:
                    ValidateString(SetValue(slider.Label), allowEmpty: true, "slider label");
                    ValidateSparseRange(
                        SetStructValue(slider.LowValue),
                        SetStructValue(slider.HighValue),
                        SetStructValue(slider.Value),
                        SetStructValue(slider.PageSize)
                    );
                    break;
                case UiElement.SliderInt slider:
                    ValidateString(SetValue(slider.Label), allowEmpty: true, "slider label");
                    ValidateSparseRange(
                        SetStructValue(slider.LowValue),
                        SetStructValue(slider.HighValue),
                        SetStructValue(slider.Value),
                        SetStructValue(slider.PageSize)
                    );
                    break;
                case UiElement.MinMaxSlider range:
                    ValidateString(SetValue(range.Label), allowEmpty: true, "range slider label");
                    float? lowLimit = SetValue(range.LowLimit) is LowerLimit.Inclusive low
                        ? low.Value
                        : null;
                    float? highLimit = SetValue(range.HighLimit) is UpperLimit.Inclusive high
                        ? high.Value
                        : null;
                    float? minValue = SetStructValue(range.MinValue);
                    float? maxValue = SetStructValue(range.MaxValue);
                    ValidateFinite(minValue, maxValue, lowLimit, highLimit);
                    if (minValue > maxValue || lowLimit > highLimit)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "MinMaxSlider range is invalid."
                        );
                    if (minValue < lowLimit || maxValue > highLimit)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "MinMaxSlider range is invalid."
                        );
                    break;
                case UiElement.ProgressBar progress:
                    ValidateString(SetValue(progress.Title), allowEmpty: true, "progress title");
                    float? progressLow = SetStructValue(progress.LowValue);
                    float? progressHigh = SetStructValue(progress.HighValue);
                    float? progressValue = SetStructValue(progress.Value);
                    ValidateFinite(progressLow, progressHigh, progressValue);
                    if (progressLow > progressHigh)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "ProgressBar range is invalid."
                        );
                    if (progressValue < progressLow || progressValue > progressHigh)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "ProgressBar range is invalid."
                        );
                    break;
                case UiElement.Tab tab:
                    ValidateString(SetValue(tab.Text), allowEmpty: true, "tab text");
                    break;
                default:
                    break;
            }
            RejectUnavailableLayout(element);
        }

        private static void ValidateLayout(UiElement element)
        {
            if (element.GridItem.IsSet)
                ValidateGridItem(element.GridItem.Value);
            if (element.StackItem.IsSet)
                ValidateStackItem(element.StackItem.Value);
            if (element.Sticky.IsSet)
                ValidateSticky(element.Sticky.Value);
            if (element.OverlayPlacement.IsSet)
                ValidateOverlay(element.OverlayPlacement.Value);

            switch (element)
            {
                case UiElement.Flex flex:
                    ValidateEnum<UiFlexDirection>(SetStructValue(flex.Direction), "Flex direction");
                    ValidateEnum<UiFlexWrap>(SetStructValue(flex.Wrap), "Flex wrap");
                    ValidateGap(SetStructValue(flex.RowGap));
                    ValidateGap(SetStructValue(flex.ColumnGap));
                    ValidateContainerAlign(SetStructValue(flex.AlignItems));
                    ValidateEnum<UiJustify>(
                        SetStructValue(flex.JustifyContent),
                        "Flex justification"
                    );
                    break;
                case UiElement.Grid grid:
                    foreach (GridTrack track in SetValues(grid.Columns))
                        ValidateGridTrack(track);
                    foreach (GridTrack track in SetValues(grid.Rows))
                        ValidateGridTrack(track);
                    if (grid.AutoColumns.IsSet)
                        ValidateGridTrack(grid.AutoColumns.Value);
                    if (grid.AutoRows.IsSet)
                        ValidateGridTrack(grid.AutoRows.Value);
                    ValidateEnum<GridAutoFlow>(SetStructValue(grid.AutoFlow), "Grid auto-flow");
                    ValidateGap(SetStructValue(grid.RowGap));
                    ValidateGap(SetStructValue(grid.ColumnGap));
                    ValidateContainerAlign(SetStructValue(grid.AlignItems));
                    ValidateContainerAlign(SetStructValue(grid.JustifyItems));
                    break;
                case UiElement.Stack stack:
                    ValidateContainerAlign(SetStructValue(stack.AlignItems));
                    ValidateContainerAlign(SetStructValue(stack.JustifyItems));
                    break;
                default:
                    break;
            }
        }

        private static void ValidateGridTrack(GridTrack track)
        {
            switch (track)
            {
                case GridTrack.Px px:
                    ValidateFinite(px.Value);
                    if (px.Value < 0)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "Grid pixel tracks must be nonnegative."
                        );
                    break;
                case GridTrack.Fraction fraction:
                    ValidateFinite(fraction.Value);
                    if (fraction.Value <= 0)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "Grid fraction tracks must be positive."
                        );
                    break;
                case GridTrack.Auto:
                    break;
                default:
                    throw Failure(CoreErrorCode.InvalidProperty, "Unknown Grid track.");
            }
        }

        private static void ValidateGridItem(GridItem item)
        {
            ValidateEnum<UiAlign>(item.AlignSelf, "Grid item alignment");
            ValidateEnum<UiAlign>(item.JustifySelf, "Grid item justification");
            bool startsArePositive = item.Row is null or > 0 && item.Column is null or > 0;
            bool spansArePositive = item.RowSpan > 0 && item.ColumnSpan > 0;
            bool rowFits = item.Row is not uint row || row <= uint.MaxValue - (item.RowSpan - 1);
            bool columnFits =
                item.Column is not uint column || column <= uint.MaxValue - (item.ColumnSpan - 1);
            if (!startsArePositive || !spansArePositive || !rowFits || !columnFits)
                throw Failure(CoreErrorCode.InvalidProperty, "Grid item placement is invalid.");
        }

        private static void ValidateStackItem(StackItem item)
        {
            ValidateEnum<UiAlign>(item.AlignSelf, "Stack item alignment");
            ValidateEnum<UiAlign>(item.JustifySelf, "Stack item justification");
            ValidateFinite(item.Top, item.Right, item.Bottom, item.Left);
            if (item.Top < 0 || item.Right < 0 || item.Bottom < 0 || item.Left < 0)
                throw Failure(CoreErrorCode.InvalidProperty, "Stack insets must be nonnegative.");
        }

        private static void ValidateSticky(Sticky sticky)
        {
            ValidateFinite(sticky.Top, sticky.Right, sticky.Bottom, sticky.Left);
            int horizontalEdges = (sticky.Left.HasValue ? 1 : 0) + (sticky.Right.HasValue ? 1 : 0);
            int verticalEdges = (sticky.Top.HasValue ? 1 : 0) + (sticky.Bottom.HasValue ? 1 : 0);
            if (horizontalEdges + verticalEdges == 0)
                throw Failure(CoreErrorCode.InvalidProperty, "Sticky requires an inset edge.");
            if (horizontalEdges > 1 || verticalEdges > 1)
                throw Failure(CoreErrorCode.InvalidProperty, "Sticky edges are contradictory.");
        }

        private static void ValidateOverlay(OverlayPlacement overlay)
        {
            switch (overlay)
            {
                case OverlayPlacement.Layer layer:
                    ValidateEnum<OverlayLayer>(layer.Value, "Overlay layer");
                    break;
                case OverlayPlacement.Popover popover:
                    ValidateEnum<PlacementSide>(popover.Placement.Side, "Popover side");
                    ValidateEnum<PlacementAlign>(popover.Placement.Align, "Popover alignment");
                    ValidateFinite(
                        popover.Placement.MainOffset,
                        popover.Placement.CrossOffset,
                        popover.Placement.CollisionPadding
                    );
                    if (popover.Placement.CollisionPadding < 0)
                        throw Failure(
                            CoreErrorCode.InvalidProperty,
                            "Popover collision padding must be nonnegative."
                        );
                    break;
                case OverlayPlacement.Modal:
                    break;
                default:
                    throw Failure(CoreErrorCode.InvalidProperty, "Unknown overlay placement.");
            }
        }

        private static void ValidateGap(float? gap)
        {
            ValidateFinite(gap);
            if (gap < 0)
                throw Failure(CoreErrorCode.InvalidProperty, "Layout gaps must be nonnegative.");
        }

        private static void ValidateContainerAlign(UiAlign? align)
        {
            ValidateEnum<UiAlign>(align, "Container alignment");
            if (align == UiAlign.Auto)
                throw Failure(CoreErrorCode.InvalidProperty, "Container alignment cannot be Auto.");
        }

        private static void ValidateEnum<T>(T? value, string name)
            where T : struct, Enum
        {
            if (value.HasValue && !Enum.IsDefined(typeof(T), value.Value))
                throw Failure(CoreErrorCode.InvalidProperty, $"{name} is not recognized.");
        }

        private static void RejectUnavailableLayout(UiElement element)
        {
            bool unavailableHost = element is UiElement.Grid or UiElement.Stack;
            bool unavailableDescriptor =
                element.GridItem.IsSet
                || element.StackItem.IsSet
                || element.Sticky.IsSet
                || element.OverlayPlacement.IsSet;
            if (unavailableHost || unavailableDescriptor)
                throw Failure(
                    CoreErrorCode.InvalidProperty,
                    "The authored layout host or descriptor is not enabled "
                        + "by its native layout task."
                );
        }

        private static T? SetValue<T>(Prop<T> value)
            where T : class => value.IsSet ? value.Value : null;

        private static T? SetStructValue<T>(Prop<T> value)
            where T : struct => value.IsSet ? value.Value : null;

        private static IReadOnlyList<T> SetValues<T>(Prop<IReadOnlyList<T>> value) =>
            value.IsSet ? value.Value : Array.Empty<T>();

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
                    || element is not UiElement.RadioButtonGroup group
                    || !group.Choices.IsSet
                    || index < group.Choices.Value.Count;
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
                (UiElement.GroupBox group, UiPart.GroupBoxTitle)
                    when group.Text.IsReset || (group.Text.IsSet && group.Text.Value.Length == 0) =>
                    false,
                (UiElement.Tab tab, UiPart.TabCloseButton)
                    when tab.Closeable.IsReset || (tab.Closeable.IsSet && !tab.Closeable.Value) =>
                    false,
                (UiElement.Toggle toggle, UiPart.ToggleLabel) when toggle.Label.IsReset => false,
                (UiElement.Toggle toggle, UiPart.ToggleText) when toggle.Text.IsReset => false,
                (UiElement.RadioButton radio, UiPart.RadioButtonLabel) when radio.Label.IsReset =>
                    false,
                (UiElement.RadioButton radio, UiPart.RadioButtonText) when radio.Text.IsReset =>
                    false,
                (UiElement.DropdownField dropdown, UiPart.DropdownFieldLabel)
                    when dropdown.Label.IsReset => false,
                (UiElement.TextField field, UiPart.TextFieldLabel) when field.Label.IsReset =>
                    false,
                (UiElement.RadioButtonGroup group, UiPart.RadioButtonGroupLabel)
                    when group.Label.IsReset => false,
                (UiElement.ToggleButtonGroup group, UiPart.ToggleButtonGroupLabel)
                    when group.Label.IsReset => false,
                (
                    UiElement.RadioButtonGroup group,
                    UiPart.RadioButtonGroupAllOptions
                        or UiPart.RadioButtonGroupOption
                        or UiPart.RadioButtonGroupOptionCheckmarkBackground
                        or UiPart.RadioButtonGroupOptionCheckmark
                        or UiPart.RadioButtonGroupOptionText
                ) when group.Choices.IsReset => false,
                (
                    UiElement.TextField text,
                    UiPart.TextFieldMultilineScrollView
                        or UiPart.TextFieldVerticalScroller
                        or UiPart.TextFieldVerticalSlider
                        or UiPart.TextFieldVerticalLowButton
                        or UiPart.TextFieldVerticalHighButton
                        or UiPart.TextFieldVerticalTrack
                        or UiPart.TextFieldVerticalDragger
                        or UiPart.TextFieldVerticalDraggerBorder
                ) when text.Multiline.IsReset || (text.Multiline.IsSet && !text.Multiline.Value) =>
                    false,
                (UiElement.Slider slider, UiPart.SliderFill)
                    when slider.Fill.IsReset || (slider.Fill.IsSet && !slider.Fill.Value) => false,
                (UiElement.Slider slider, UiPart.SliderTextInput)
                    when slider.ShowInputField.IsReset
                        || (slider.ShowInputField.IsSet && !slider.ShowInputField.Value) => false,
                (UiElement.SliderInt slider, UiPart.SliderIntFill)
                    when slider.Fill.IsReset || (slider.Fill.IsSet && !slider.Fill.Value) => false,
                (UiElement.SliderInt slider, UiPart.SliderIntTextInput)
                    when slider.ShowInputField.IsReset
                        || (slider.ShowInputField.IsSet && !slider.ShowInputField.Value) => false,
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
                    or UiEventKind.KeyDown
                    or UiEventKind.KeyUp
                    or UiEventKind.NavigationMove
                    or UiEventKind.NavigationCancel
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
