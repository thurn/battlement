use serde::{Deserialize, Serialize};

use crate::{Style, UiElement};

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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct PartStyle {
    pub part: Part,
    pub style: Style,
}

pub(crate) fn append(parts: &mut Option<Vec<PartStyle>>, part: Part, style: Style) {
    parts
        .get_or_insert_with(Vec::new)
        .push(PartStyle { part, style });
}

pub(crate) fn merge(target: &mut Option<Vec<PartStyle>>, update: &Option<Vec<PartStyle>>) {
    let Some(update) = update else {
        return;
    };
    let target = target.get_or_insert_with(Vec::new);
    for replacement in update {
        if let Some(current) = target
            .iter_mut()
            .find(|value| value.part == replacement.part)
        {
            current.style = current.style.clone().merge(replacement.style.clone());
        } else {
            target.push(replacement.clone());
        }
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
    )
}

pub(crate) fn exists_in_complete_state(value: &UiElement, part: Part) -> bool {
    match (value, part) {
        (UiElement::Button(value), Part::ButtonIcon) => value.icon.is_some(),
        (UiElement::GroupBox(value), Part::GroupBoxTitle) => {
            value.text.as_deref().is_some_and(|text| !text.is_empty())
        }
        (UiElement::Toggle(value), Part::ToggleLabel) => value.label.is_some(),
        (UiElement::Toggle(value), Part::ToggleText) => value.text.is_some(),
        (UiElement::RadioButton(value), Part::RadioButtonLabel) => value.label.is_some(),
        (UiElement::RadioButton(value), Part::RadioButtonText) => value.text.is_some(),
        (UiElement::DropdownField(value), Part::DropdownFieldLabel) => value.label.is_some(),
        _ => belongs_to(value, part),
    }
}

macro_rules! part_style_builders {
    ($($method:ident => $part:ident),+ $(,)?) => {$(
        #[doc = concat!("Applies sparse inline declarations to the native `", stringify!($part), "` part.")]
        #[must_use]
        pub fn $method(mut self, value: Style) -> Self {
            parts::append(
                &mut self.parts,
                Part::$part,
                value,
            );
            self
        }
    )+};
}

pub(crate) use part_style_builders;
