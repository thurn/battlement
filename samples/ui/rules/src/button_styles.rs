use battlement::{Align, Color, FlexDirection, FlexWrap, Justify, LengthUnits, Style};

use crate::design_system::{
  ACCENT, BACKGROUND, BUTTON_BACKGROUND, BUTTON_TEXT, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND,
};

pub(crate) fn intro() -> Style {
  Style::new()
    .width(100_f32.pct())
    .color(PRIMARY_TEXT)
    .font_size(19)
    .white_space(battlement::WhiteSpace::Normal)
    .margin((0, 8, 10, 8))
}

pub(crate) fn gallery() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .justify_content(Justify::SpaceBetween)
    .width(100_f32.pct())
}

pub(crate) fn card() -> Style {
  Style::new()
    .width(23.7_f32.pct())
    .height(158)
    .padding(12)
    .margin(4)
    .background_color(SPECIMEN_BACKGROUND)
    .border_radius(12)
}

pub(crate) fn caption() -> Style {
  Style::new()
    .color(CYAN)
    .font_size(16)
    .white_space(battlement::WhiteSpace::Normal)
    .margin((0, 0, 8, 0))
}

pub(crate) fn help() -> Style {
  Style::new()
    .color(Color::rgb(0.68, 0.78, 0.81))
    .font_size(16)
    .white_space(battlement::WhiteSpace::Normal)
    .margin((7, 0, 0, 0))
}

pub(crate) fn button() -> Style {
  Style::new()
    .height(52)
    .background_color(BUTTON_BACKGROUND)
    .color(BUTTON_TEXT)
    .font_size(19)
    .padding((8, 10))
    .border_radius(9)
}

pub(crate) fn icon_button() -> Style {
  button().unity_text_align(battlement::TextAnchor::MiddleCenter)
}

pub(crate) fn navigation_button() -> Style {
  button()
    .border_color((ACCENT, ACCENT, ACCENT, ACCENT))
    .border_width(2)
}

pub(crate) fn repeat_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
}

pub(crate) fn repeat_card() -> Style {
  card().width(100_f32.pct()).height(132)
}

pub(crate) fn repeat_button() -> Style {
  button().width(70_f32.pct()).margin((0, 12, 0, 0))
}

pub(crate) fn counter() -> Style {
  Style::new()
    .flex_grow(1)
    .height(52)
    .background_color(BACKGROUND)
    .color(ACCENT)
    .font_size(27)
    .unity_text_align(battlement::TextAnchor::MiddleCenter)
    .border_radius(9)
}

pub(crate) fn status() -> Style {
  Style::new()
    .background_color(BACKGROUND)
    .color(PRIMARY_TEXT)
    .font_size(19)
    .padding((8, 12))
    .margin((8, 4, 0, 4))
    .border_radius(9)
}
