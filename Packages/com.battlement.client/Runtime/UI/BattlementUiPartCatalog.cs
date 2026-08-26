#nullable enable

using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UIElements;
using NativeButton = UnityEngine.UIElements.Button;
using NativeDropdownField = UnityEngine.UIElements.DropdownField;
using NativeGroupBox = UnityEngine.UIElements.GroupBox;
using NativeMinMaxSlider = UnityEngine.UIElements.MinMaxSlider;
using NativePopupWindow = UnityEngine.UIElements.PopupWindow;
using NativeProgressBar = UnityEngine.UIElements.ProgressBar;
using NativeRadioButton = UnityEngine.UIElements.RadioButton;
using NativeRadioButtonGroup = UnityEngine.UIElements.RadioButtonGroup;
using NativeScroller = UnityEngine.UIElements.Scroller;
using NativeScrollView = UnityEngine.UIElements.ScrollView;
using NativeSlider = UnityEngine.UIElements.Slider;
using NativeSliderInt = UnityEngine.UIElements.SliderInt;
using NativeTab = UnityEngine.UIElements.Tab;
using NativeTabView = UnityEngine.UIElements.TabView;
using NativeTextField = UnityEngine.UIElements.TextField;
using NativeToggle = UnityEngine.UIElements.Toggle;
using NativeToggleButtonGroup = UnityEngine.UIElements.ToggleButtonGroup;

namespace Battlement.UI
{
    internal static class BattlementUiPartCatalog
    {
        private static readonly string[] InnerTextFieldClassNames =
        {
            "unity-text-element--inner-input-field-component",
            "unity-text-element--inner-input-field-component--vertical",
            "unity-text-element--inner-input-field-component--scroll-view",
        };

        public static IReadOnlyList<VisualElement> Resolve(
            VisualElement owner,
            UiPart part,
            uint? index
        ) =>
            part switch
            {
                UiPart.ButtonIcon => One(owner, NativeButton.iconUssClassName),
                UiPart.GroupBoxTitle => One(owner, NativeGroupBox.labelUssClassName),
                UiPart.PopupWindowContentContainer => Single(
                    ((NativePopupWindow)owner).contentContainer
                ),
                UiPart.ToggleLabel => One(owner, NativeToggle.labelUssClassName),
                UiPart.ToggleInput => One(owner, NativeToggle.inputUssClassName),
                UiPart.ToggleCheckmark => One(owner, NativeToggle.checkmarkUssClassName),
                UiPart.ToggleText => One(owner, NativeToggle.textUssClassName),
                UiPart.RadioButtonLabel => One(owner, NativeRadioButton.labelUssClassName),
                UiPart.RadioButtonInput => One(owner, NativeRadioButton.inputUssClassName),
                UiPart.RadioButtonCheckmarkBackground => One(
                    owner,
                    NativeRadioButton.checkmarkBackgroundUssClassName
                ),
                UiPart.RadioButtonCheckmark => One(owner, NativeRadioButton.checkmarkUssClassName),
                UiPart.RadioButtonText => One(owner, NativeRadioButton.textUssClassName),
                UiPart.DropdownFieldLabel => One(owner, NativeDropdownField.labelUssClassName),
                UiPart.DropdownFieldInput => One(owner, NativeDropdownField.inputUssClassName),
                UiPart.DropdownFieldText => One(owner, NativeDropdownField.textUssClassName),
                UiPart.DropdownFieldArrow => One(owner, NativeDropdownField.arrowUssClassName),
                UiPart.ProgressBarContainer => One(owner, NativeProgressBar.containerUssClassName),
                UiPart.ProgressBarBackground => One(
                    owner,
                    NativeProgressBar.backgroundUssClassName
                ),
                UiPart.ProgressBarProgress => One(owner, NativeProgressBar.progressUssClassName),
                UiPart.ProgressBarTitleContainer => One(
                    owner,
                    NativeProgressBar.titleContainerUssClassName
                ),
                UiPart.ProgressBarTitle => One(owner, NativeProgressBar.titleUssClassName),
                UiPart.ScrollViewContentAndVerticalScrollContainer => One(
                    owner,
                    NativeScrollView.contentAndVerticalScrollUssClassName
                ),
                UiPart.ScrollViewViewport => Single(((NativeScrollView)owner).contentViewport),
                UiPart.ScrollViewContentContainer => Single(
                    ((NativeScrollView)owner).contentContainer
                ),
                UiPart.ScrollViewHorizontalScroller => Single(
                    ((NativeScrollView)owner).horizontalScroller
                ),
                UiPart.ScrollViewVerticalScroller => Single(
                    ((NativeScrollView)owner).verticalScroller
                ),
                UiPart.ScrollViewHorizontalSlider => Single(
                    ((NativeScrollView)owner).horizontalScroller.slider
                ),
                UiPart.ScrollViewHorizontalLowButton => Single(
                    ((NativeScrollView)owner).horizontalScroller.lowButton
                ),
                UiPart.ScrollViewHorizontalHighButton => Single(
                    ((NativeScrollView)owner).horizontalScroller.highButton
                ),
                UiPart.ScrollViewHorizontalTrack => One(
                    ((NativeScrollView)owner).horizontalScroller.slider,
                    NativeSlider.trackerUssClassName
                ),
                UiPart.ScrollViewHorizontalDragger => One(
                    ((NativeScrollView)owner).horizontalScroller.slider,
                    NativeSlider.draggerUssClassName
                ),
                UiPart.ScrollViewHorizontalDraggerBorder => One(
                    ((NativeScrollView)owner).horizontalScroller.slider,
                    NativeSlider.draggerBorderUssClassName
                ),
                UiPart.ScrollViewVerticalSlider => Single(
                    ((NativeScrollView)owner).verticalScroller.slider
                ),
                UiPart.ScrollViewVerticalLowButton => Single(
                    ((NativeScrollView)owner).verticalScroller.lowButton
                ),
                UiPart.ScrollViewVerticalHighButton => Single(
                    ((NativeScrollView)owner).verticalScroller.highButton
                ),
                UiPart.ScrollViewVerticalTrack => One(
                    ((NativeScrollView)owner).verticalScroller.slider,
                    NativeSlider.trackerUssClassName
                ),
                UiPart.ScrollViewVerticalDragger => One(
                    ((NativeScrollView)owner).verticalScroller.slider,
                    NativeSlider.draggerUssClassName
                ),
                UiPart.ScrollViewVerticalDraggerBorder => One(
                    ((NativeScrollView)owner).verticalScroller.slider,
                    NativeSlider.draggerBorderUssClassName
                ),
                UiPart.ScrollerSlider => Single(((NativeScroller)owner).slider),
                UiPart.ScrollerLowButton => Single(((NativeScroller)owner).lowButton),
                UiPart.ScrollerHighButton => Single(((NativeScroller)owner).highButton),
                UiPart.ScrollerTrack => One(
                    ((NativeScroller)owner).slider,
                    NativeSlider.trackerUssClassName
                ),
                UiPart.ScrollerDragger => One(
                    ((NativeScroller)owner).slider,
                    NativeSlider.draggerUssClassName
                ),
                UiPart.ScrollerDraggerBorder => One(
                    ((NativeScroller)owner).slider,
                    NativeSlider.draggerBorderUssClassName
                ),
                UiPart.TabHeader => Single(((NativeTab)owner).tabHeader),
                UiPart.TabLabel => One(
                    ((NativeTab)owner).tabHeader,
                    NativeTab.tabHeaderLabelUssClassName
                ),
                UiPart.TabIcon => One(
                    ((NativeTab)owner).tabHeader,
                    NativeTab.tabHeaderImageUssClassName
                ),
                UiPart.TabUnderline => One(
                    ((NativeTab)owner).tabHeader,
                    NativeTab.tabHeaderUnderlineUssClassName
                ),
                UiPart.TabCloseButton => One(
                    ((NativeTab)owner).tabHeader,
                    NativeTab.closeButtonUssClassName
                ),
                UiPart.TabDragHandle => One(
                    ((NativeTab)owner).tabHeader,
                    NativeTab.reorderableItemHandleUssClassName
                ),
                UiPart.TabDragHandleLeadingBar => IndexedClass(
                    ((NativeTab)owner).tabHeader,
                    NativeTab.reorderableItemHandleBarUssClassName,
                    0
                ),
                UiPart.TabDragHandleTrailingBar => IndexedClass(
                    ((NativeTab)owner).tabHeader,
                    NativeTab.reorderableItemHandleBarUssClassName,
                    1
                ),
                UiPart.TabContentContainer => Single(((NativeTab)owner).contentContainer),
                UiPart.TabViewContentViewport => Single(((NativeTabView)owner).contentViewport),
                UiPart.TabViewHeaderContainer => One(owner, NativeTabView.headerContainerClassName),
                UiPart.TabViewContentContainer => Single(((NativeTabView)owner).contentContainer),
                UiPart.TabViewPreviousButton => One(
                    owner,
                    NativeTabView.previousButtonUssClassName
                ),
                UiPart.TabViewNextButton => One(owner, NativeTabView.nextButtonUssClassName),
                UiPart.TextFieldLabel => One(owner, NativeTextField.labelUssClassName),
                UiPart.TextFieldInput => One(owner, NativeTextField.inputUssClassName),
                UiPart.TextFieldTextElement => TextInputElement(owner, part),
                UiPart.TextFieldMultilineScrollView => Single(RequireMultiline(owner, part)),
                UiPart.TextFieldVerticalScroller => MultilineScroller(owner, part),
                UiPart.TextFieldVerticalSlider => MultilineScrollerPart(owner, part),
                UiPart.TextFieldVerticalLowButton => MultilineScrollerPart(owner, part),
                UiPart.TextFieldVerticalHighButton => MultilineScrollerPart(owner, part),
                UiPart.TextFieldVerticalTrack => MultilineScrollerPart(owner, part),
                UiPart.TextFieldVerticalDragger => MultilineScrollerPart(owner, part),
                UiPart.TextFieldVerticalDraggerBorder => MultilineScrollerPart(owner, part),
                UiPart.RadioButtonGroupLabel => One(
                    owner,
                    NativeRadioButtonGroup.labelUssClassName
                ),
                UiPart.RadioButtonGroupInput => One(
                    owner,
                    NativeRadioButtonGroup.inputUssClassName
                ),
                UiPart.RadioButtonGroupChoicesContainer => One(
                    owner,
                    NativeRadioButtonGroup.containerUssClassName
                ),
                UiPart.RadioButtonGroupContentContainer => Single(
                    ((NativeRadioButtonGroup)owner).contentContainer
                ),
                UiPart.RadioButtonGroupAllOptions => AllOptions(owner),
                UiPart.RadioButtonGroupOption => Option(owner, index),
                UiPart.RadioButtonGroupOptionCheckmarkBackground => OptionPart(
                    owner,
                    index,
                    NativeRadioButton.checkmarkBackgroundUssClassName
                ),
                UiPart.RadioButtonGroupOptionCheckmark => OptionPart(
                    owner,
                    index,
                    NativeRadioButton.checkmarkUssClassName
                ),
                UiPart.RadioButtonGroupOptionText => OptionPart(
                    owner,
                    index,
                    NativeRadioButton.textUssClassName
                ),
                UiPart.ToggleButtonGroupLabel => One(
                    owner,
                    NativeToggleButtonGroup.labelUssClassName
                ),
                UiPart.ToggleButtonGroupInput => One(
                    owner,
                    NativeToggleButtonGroup.inputUssClassName
                ),
                UiPart.SliderLabel => One(owner, NativeSlider.labelUssClassName),
                UiPart.SliderInput => One(owner, NativeSlider.inputUssClassName),
                UiPart.SliderTrack => One(owner, NativeSlider.trackerUssClassName),
                UiPart.SliderDragger => One(owner, NativeSlider.draggerUssClassName),
                UiPart.SliderDraggerBorder => One(owner, NativeSlider.draggerBorderUssClassName),
                UiPart.SliderFill => One(owner, NativeSlider.fillUssClassName),
                UiPart.SliderTextInput => One(owner, NativeSlider.textFieldClassName),
                UiPart.SliderIntLabel => One(owner, NativeSliderInt.labelUssClassName),
                UiPart.SliderIntInput => One(owner, NativeSliderInt.inputUssClassName),
                UiPart.SliderIntTrack => One(owner, NativeSliderInt.trackerUssClassName),
                UiPart.SliderIntDragger => One(owner, NativeSliderInt.draggerUssClassName),
                UiPart.SliderIntDraggerBorder => One(
                    owner,
                    NativeSliderInt.draggerBorderUssClassName
                ),
                UiPart.SliderIntFill => One(owner, NativeSliderInt.fillUssClassName),
                UiPart.SliderIntTextInput => One(owner, NativeSliderInt.textFieldClassName),
                UiPart.MinMaxSliderLabel => One(owner, NativeMinMaxSlider.labelUssClassName),
                UiPart.MinMaxSliderInput => One(owner, NativeMinMaxSlider.inputUssClassName),
                UiPart.MinMaxSliderTrack => One(owner, NativeMinMaxSlider.trackerUssClassName),
                UiPart.MinMaxSliderMinimumThumb => One(
                    owner,
                    NativeMinMaxSlider.minThumbUssClassName
                ),
                UiPart.MinMaxSliderMaximumThumb => One(
                    owner,
                    NativeMinMaxSlider.maxThumbUssClassName
                ),
                UiPart.MinMaxSliderRangeDragger => One(
                    owner,
                    NativeMinMaxSlider.draggerUssClassName
                ),
                _ => throw Failure($"Unsupported UI part {part}."),
            };

        private static IReadOnlyList<VisualElement> MultilineScroller(
            VisualElement owner,
            UiPart part
        ) => Single(RequireMultiline(owner, part).verticalScroller);

        private static IReadOnlyList<VisualElement> MultilineScrollerPart(
            VisualElement owner,
            UiPart part
        )
        {
            NativeScroller scroller = RequireMultiline(owner, part).verticalScroller;
            return part switch
            {
                UiPart.TextFieldVerticalSlider => Single(scroller.slider),
                UiPart.TextFieldVerticalLowButton => Single(scroller.lowButton),
                UiPart.TextFieldVerticalHighButton => Single(scroller.highButton),
                UiPart.TextFieldVerticalTrack => One(
                    scroller.slider,
                    NativeSlider.trackerUssClassName
                ),
                UiPart.TextFieldVerticalDragger => One(
                    scroller.slider,
                    NativeSlider.draggerUssClassName
                ),
                UiPart.TextFieldVerticalDraggerBorder => One(
                    scroller.slider,
                    NativeSlider.draggerBorderUssClassName
                ),
                _ => throw Failure($"Unsupported multiline part {part}."),
            };
        }

        private static NativeScrollView RequireMultiline(VisualElement owner, UiPart part)
        {
            List<NativeScrollView> matches = owner.Query<NativeScrollView>().ToList();
            if (matches.Count != 1)
                throw Failure(
                    $"Native part {part} matched {matches.Count} multiline scroll views."
                );
            return matches[0];
        }

        private static IReadOnlyList<VisualElement> AllOptions(VisualElement owner)
        {
            List<NativeRadioButton> matches = owner.Query<NativeRadioButton>().ToList();
            if (matches.Count == 0)
                throw Failure("RadioButtonGroup has no native options.");
            return matches;
        }

        private static IReadOnlyList<VisualElement> Option(VisualElement owner, uint? index)
        {
            List<NativeRadioButton> matches = owner.Query<NativeRadioButton>().ToList();
            if (index is not uint value || value >= matches.Count)
                throw Failure($"RadioButtonGroup option index {index} is unavailable.");
            return Single(matches[(int)value]);
        }

        private static IReadOnlyList<VisualElement> OptionPart(
            VisualElement owner,
            uint? index,
            string className
        ) => One(Option(owner, index)[0], className);

        private static IReadOnlyList<VisualElement> IndexedClass(
            VisualElement owner,
            string className,
            int index
        )
        {
            List<VisualElement> matches = owner.Query<VisualElement>(className: className).ToList();
            if (matches.Count != 2)
                throw Failure($"Native part .{className} matched {matches.Count} elements.");
            return Single(matches[index]);
        }

        private static IReadOnlyList<VisualElement> One(VisualElement owner, string className)
        {
            List<VisualElement> matches = owner.Query<VisualElement>(className: className).ToList();
            if (matches.Count != 1)
                throw new UnityException(
                    $"Native part .{className} matched {matches.Count} elements beneath "
                        + $"{owner.GetType().Name}."
                );
            return matches;
        }

        private static IReadOnlyList<VisualElement> Single(VisualElement value) => new[] { value };

        private static IReadOnlyList<VisualElement> TextInputElement(
            VisualElement owner,
            UiPart part
        )
        {
            List<VisualElement> inputs = owner
                .Query<VisualElement>(name: NativeTextField.textInputUssName)
                .ToList();
            if (inputs.Count != 1)
                throw Failure($"Native part {part} matched {inputs.Count} named text inputs.");
            var text = new HashSet<TextElement>();
            foreach (string className in InnerTextFieldClassNames)
            foreach (
                TextElement match in inputs[0].Query<TextElement>(className: className).ToList()
            )
                text.Add(match);
            if (text.Count != 1)
                throw Failure($"Native part {part} matched {text.Count} inner text elements.");
            foreach (TextElement match in text)
                return new VisualElement[] { match };
            throw Failure($"Native part {part} is missing.");
        }

        private static BattlementUiException Failure(string message) =>
            new(CoreErrorCode.InvalidProperty, message);
    }
}
