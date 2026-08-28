use serde::{Deserialize, Serialize};

use crate::{Prop, Style, UiElement};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub(crate) enum Part {
  ButtonIcon,
  GroupBoxTitle,
  PopupWindowContentContainer,
  ToggleLabel,
  ToggleInput,
  ToggleCheckmark,
  ToggleText,
  RadioButtonLabel,
  RadioButtonInput,
  RadioButtonCheckmarkBackground,
  RadioButtonCheckmark,
  RadioButtonText,
  DropdownFieldLabel,
  DropdownFieldInput,
  DropdownFieldText,
  DropdownFieldArrow,
  ProgressBarContainer,
  ProgressBarBackground,
  ProgressBarProgress,
  ProgressBarTitleContainer,
  ProgressBarTitle,
  ScrollViewContentAndVerticalScrollContainer,
  ScrollViewViewport,
  ScrollViewContentContainer,
  ScrollViewHorizontalScroller,
  ScrollViewHorizontalSlider,
  ScrollViewHorizontalLowButton,
  ScrollViewHorizontalHighButton,
  ScrollViewHorizontalTrack,
  ScrollViewHorizontalDragger,
  ScrollViewHorizontalDraggerBorder,
  ScrollViewVerticalScroller,
  ScrollViewVerticalSlider,
  ScrollViewVerticalLowButton,
  ScrollViewVerticalHighButton,
  ScrollViewVerticalTrack,
  ScrollViewVerticalDragger,
  ScrollViewVerticalDraggerBorder,
  ScrollerSlider,
  ScrollerLowButton,
  ScrollerHighButton,
  ScrollerTrack,
  ScrollerDragger,
  ScrollerDraggerBorder,
  TabHeader,
  TabLabel,
  TabIcon,
  TabUnderline,
  TabCloseButton,
  TabDragHandle,
  TabDragHandleLeadingBar,
  TabDragHandleTrailingBar,
  TabContentContainer,
  TabViewContentViewport,
  TabViewHeaderContainer,
  TabViewContentContainer,
  TabViewPreviousButton,
  TabViewNextButton,
  TextFieldLabel,
  TextFieldInput,
  TextFieldTextElement,
  TextFieldMultilineScrollView,
  TextFieldVerticalScroller,
  TextFieldVerticalSlider,
  TextFieldVerticalLowButton,
  TextFieldVerticalHighButton,
  TextFieldVerticalTrack,
  TextFieldVerticalDragger,
  TextFieldVerticalDraggerBorder,
  RadioButtonGroupLabel,
  RadioButtonGroupInput,
  RadioButtonGroupChoicesContainer,
  RadioButtonGroupContentContainer,
  RadioButtonGroupAllOptions,
  RadioButtonGroupOption,
  RadioButtonGroupOptionCheckmarkBackground,
  RadioButtonGroupOptionCheckmark,
  RadioButtonGroupOptionText,
  ToggleButtonGroupLabel,
  ToggleButtonGroupInput,
  SliderLabel,
  SliderInput,
  SliderTrack,
  SliderDragger,
  SliderDraggerBorder,
  SliderFill,
  SliderTextInput,
  SliderIntLabel,
  SliderIntInput,
  SliderIntTrack,
  SliderIntDragger,
  SliderIntDraggerBorder,
  SliderIntFill,
  SliderIntTextInput,
  MinMaxSliderLabel,
  MinMaxSliderInput,
  MinMaxSliderTrack,
  MinMaxSliderMinimumThumb,
  MinMaxSliderMaximumThumb,
  MinMaxSliderRangeDragger,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct PartStyle {
  pub part: Part,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub index: Option<u32>,
  pub style: Style,
}

pub(crate) fn append(parts: &mut Option<Vec<PartStyle>>, part: Part, style: Style) {
  parts.get_or_insert_with(Vec::new).push(PartStyle {
    part,
    index: None,
    style,
  });
}

pub(crate) fn append_indexed(
  parts: &mut Option<Vec<PartStyle>>,
  part: Part,
  index: u32,
  style: Style,
) {
  parts.get_or_insert_with(Vec::new).push(PartStyle {
    part,
    index: Some(index),
    style,
  });
}

pub(crate) fn merge(target: &mut Option<Vec<PartStyle>>, update: &Option<Vec<PartStyle>>) {
  let Some(update) = update else {
    return;
  };
  let target = target.get_or_insert_with(Vec::new);
  for replacement in update {
    if let Some(current) = target
      .iter_mut()
      .find(|value| value.part == replacement.part && value.index == replacement.index)
    {
      current.style = current.style.clone().merge(replacement.style.clone());
    } else {
      target.push(replacement.clone());
    }
  }
}

pub(crate) fn remove(target: &mut Option<Vec<PartStyle>>, removed: &[Part]) {
  if let Some(target) = target {
    target.retain(|value| !removed.contains(&value.part));
  }
}

pub(crate) fn remove_indexed_outside(target: &mut Option<Vec<PartStyle>>, choice_count: usize) {
  if let Some(target) = target {
    target.retain(|value| {
      value
        .index
        .is_none_or(|index| (index as usize) < choice_count)
    });
  }
}

pub(crate) fn styles(value: &UiElement) -> Option<&[PartStyle]> {
  match value {
    UiElement::Button(value) => value.parts.as_deref(),
    UiElement::GroupBox(value) => value.parts.as_deref(),
    UiElement::PopupWindow(value) => value.parts.as_deref(),
    UiElement::Toggle(value) => value.parts.as_deref(),
    UiElement::RadioButton(value) => value.parts.as_deref(),
    UiElement::DropdownField(value) => value.parts.as_deref(),
    UiElement::ProgressBar(value) => value.parts.as_deref(),
    UiElement::ScrollView(value) => value.parts.as_deref(),
    UiElement::Scroller(value) => value.parts.as_deref(),
    UiElement::Tab(value) => value.parts.as_deref(),
    UiElement::TabView(value) => value.parts.as_deref(),
    UiElement::TextField(value) => value.parts.as_deref(),
    UiElement::RadioButtonGroup(value) => value.parts.as_deref(),
    UiElement::ToggleButtonGroup(value) => value.parts.as_deref(),
    UiElement::Slider(value) => value.parts.as_deref(),
    UiElement::SliderInt(value) => value.parts.as_deref(),
    UiElement::MinMaxSlider(value) => value.parts.as_deref(),
    _ => None,
  }
}

pub(crate) fn belongs_to(value: &UiElement, part: Part) -> bool {
  matches!(
    (value, part),
    (UiElement::Button(_), Part::ButtonIcon)
      | (UiElement::GroupBox(_), Part::GroupBoxTitle)
      | (UiElement::PopupWindow(_), Part::PopupWindowContentContainer)
      | (
        UiElement::Toggle(_),
        Part::ToggleLabel | Part::ToggleInput | Part::ToggleCheckmark | Part::ToggleText
      )
      | (
        UiElement::RadioButton(_),
        Part::RadioButtonLabel
          | Part::RadioButtonInput
          | Part::RadioButtonCheckmarkBackground
          | Part::RadioButtonCheckmark
          | Part::RadioButtonText
      )
      | (
        UiElement::DropdownField(_),
        Part::DropdownFieldLabel
          | Part::DropdownFieldInput
          | Part::DropdownFieldText
          | Part::DropdownFieldArrow
      )
      | (
        UiElement::ProgressBar(_),
        Part::ProgressBarContainer
          | Part::ProgressBarBackground
          | Part::ProgressBarProgress
          | Part::ProgressBarTitleContainer
          | Part::ProgressBarTitle
      )
      | (
        UiElement::ScrollView(_),
        Part::ScrollViewContentAndVerticalScrollContainer
          | Part::ScrollViewViewport
          | Part::ScrollViewContentContainer
          | Part::ScrollViewHorizontalScroller
          | Part::ScrollViewHorizontalSlider
          | Part::ScrollViewHorizontalLowButton
          | Part::ScrollViewHorizontalHighButton
          | Part::ScrollViewHorizontalTrack
          | Part::ScrollViewHorizontalDragger
          | Part::ScrollViewHorizontalDraggerBorder
          | Part::ScrollViewVerticalScroller
          | Part::ScrollViewVerticalSlider
          | Part::ScrollViewVerticalLowButton
          | Part::ScrollViewVerticalHighButton
          | Part::ScrollViewVerticalTrack
          | Part::ScrollViewVerticalDragger
          | Part::ScrollViewVerticalDraggerBorder
      )
      | (
        UiElement::Scroller(_),
        Part::ScrollerSlider
          | Part::ScrollerLowButton
          | Part::ScrollerHighButton
          | Part::ScrollerTrack
          | Part::ScrollerDragger
          | Part::ScrollerDraggerBorder
      )
      | (
        UiElement::Tab(_),
        Part::TabHeader
          | Part::TabLabel
          | Part::TabIcon
          | Part::TabUnderline
          | Part::TabCloseButton
          | Part::TabDragHandle
          | Part::TabDragHandleLeadingBar
          | Part::TabDragHandleTrailingBar
          | Part::TabContentContainer
      )
      | (
        UiElement::TabView(_),
        Part::TabViewContentViewport
          | Part::TabViewHeaderContainer
          | Part::TabViewContentContainer
          | Part::TabViewPreviousButton
          | Part::TabViewNextButton
      )
      | (
        UiElement::TextField(_),
        Part::TextFieldLabel
          | Part::TextFieldInput
          | Part::TextFieldTextElement
          | Part::TextFieldMultilineScrollView
          | Part::TextFieldVerticalScroller
          | Part::TextFieldVerticalSlider
          | Part::TextFieldVerticalLowButton
          | Part::TextFieldVerticalHighButton
          | Part::TextFieldVerticalTrack
          | Part::TextFieldVerticalDragger
          | Part::TextFieldVerticalDraggerBorder
      )
      | (
        UiElement::RadioButtonGroup(_),
        Part::RadioButtonGroupLabel
          | Part::RadioButtonGroupInput
          | Part::RadioButtonGroupChoicesContainer
          | Part::RadioButtonGroupContentContainer
          | Part::RadioButtonGroupAllOptions
          | Part::RadioButtonGroupOption
          | Part::RadioButtonGroupOptionCheckmarkBackground
          | Part::RadioButtonGroupOptionCheckmark
          | Part::RadioButtonGroupOptionText
      )
      | (
        UiElement::ToggleButtonGroup(_),
        Part::ToggleButtonGroupLabel | Part::ToggleButtonGroupInput
      )
      | (
        UiElement::Slider(_),
        Part::SliderLabel
          | Part::SliderInput
          | Part::SliderTrack
          | Part::SliderDragger
          | Part::SliderDraggerBorder
          | Part::SliderFill
          | Part::SliderTextInput
      )
      | (
        UiElement::SliderInt(_),
        Part::SliderIntLabel
          | Part::SliderIntInput
          | Part::SliderIntTrack
          | Part::SliderIntDragger
          | Part::SliderIntDraggerBorder
          | Part::SliderIntFill
          | Part::SliderIntTextInput
      )
      | (
        UiElement::MinMaxSlider(_),
        Part::MinMaxSliderLabel
          | Part::MinMaxSliderInput
          | Part::MinMaxSliderTrack
          | Part::MinMaxSliderMinimumThumb
          | Part::MinMaxSliderMaximumThumb
          | Part::MinMaxSliderRangeDragger
      )
  )
}

pub(crate) fn exists_in_complete_state(value: &UiElement, part: Part) -> bool {
  match (value, part) {
    (UiElement::Button(value), Part::ButtonIcon) => matches!(value.icon, Prop::Set(_)),
    (UiElement::GroupBox(value), Part::GroupBoxTitle) => {
      value.text.as_deref().is_some_and(|text| !text.is_empty())
    }
    (UiElement::Toggle(value), Part::ToggleLabel) => value.label.is_some(),
    (UiElement::Toggle(value), Part::ToggleText) => value.text.is_some(),
    (UiElement::RadioButton(value), Part::RadioButtonLabel) => value.label.is_some(),
    (UiElement::RadioButton(value), Part::RadioButtonText) => value.text.is_some(),
    (UiElement::DropdownField(value), Part::DropdownFieldLabel) => value.label.is_some(),
    (UiElement::Tab(value), Part::TabIcon) => value.icon.is_some(),
    (UiElement::Tab(value), Part::TabCloseButton) => value.closeable == Some(true),
    (UiElement::TextField(value), Part::TextFieldLabel) => value.label.is_some(),
    (
      UiElement::TextField(value),
      Part::TextFieldMultilineScrollView
      | Part::TextFieldVerticalScroller
      | Part::TextFieldVerticalSlider
      | Part::TextFieldVerticalLowButton
      | Part::TextFieldVerticalHighButton
      | Part::TextFieldVerticalTrack
      | Part::TextFieldVerticalDragger
      | Part::TextFieldVerticalDraggerBorder,
    ) => value.multiline == Some(true),
    (UiElement::RadioButtonGroup(value), Part::RadioButtonGroupLabel) => value.label.is_some(),
    (UiElement::ToggleButtonGroup(value), Part::ToggleButtonGroupLabel) => value.label.is_some(),
    (UiElement::Slider(value), Part::SliderLabel) => value.label.is_some(),
    (UiElement::Slider(value), Part::SliderFill) => value.fill == Some(true),
    (UiElement::Slider(value), Part::SliderTextInput) => value.show_input_field == Some(true),
    (UiElement::SliderInt(value), Part::SliderIntLabel) => value.label.is_some(),
    (UiElement::SliderInt(value), Part::SliderIntFill) => value.fill == Some(true),
    (UiElement::SliderInt(value), Part::SliderIntTextInput) => value.show_input_field == Some(true),
    (UiElement::MinMaxSlider(value), Part::MinMaxSliderLabel) => value.label.is_some(),
    _ => belongs_to(value, part),
  }
}
