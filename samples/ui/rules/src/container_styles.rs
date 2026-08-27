use battlement::{Color, FlexDirection, FlexWrap, Justify, LengthUnits, Style, WhiteSpace};

use crate::design_system::{
  ACCENT, BACKGROUND, BODY_TEXT, BUTTON_BACKGROUND, BUTTON_TEXT, CYAN, PRIMARY_TEXT,
  SPECIMEN_BACKGROUND,
};

pub(crate) fn intro() -> Style {
  Style::new()
    .width(100_f32.pct())
    .color(PRIMARY_TEXT)
    .font_size(18)
    .white_space(WhiteSpace::Normal)
    .margin((0, 8, 8, 8))
}

pub(crate) fn gallery() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .justify_content(Justify::SpaceBetween)
    .width(100_f32.pct())
}

pub(crate) fn specimen() -> Style {
  Style::new()
    .width(49_f32.pct())
    .height(205)
    .padding(12)
    .margin(4)
    .background_color(SPECIMEN_BACKGROUND)
    .border_radius(12)
}

pub(crate) fn caption() -> Style {
  Style::new().color(CYAN).font_size(16).margin((0, 0, 7, 0))
}

pub(crate) fn group() -> Style {
  Style::new()
    .height(122)
    .padding(12)
    .background_color(Color::rgb(0.045, 0.14, 0.17))
    .color(ACCENT)
    .font_size(18)
    .border_color((
      Color::rgb(0.2, 0.42, 0.46),
      Color::rgb(0.2, 0.42, 0.46),
      Color::rgb(0.2, 0.42, 0.46),
      Color::rgb(0.2, 0.42, 0.46),
    ))
    .border_width(1)
    .border_radius(9)
}

pub(crate) fn empty_group() -> Style {
  group().background_color(Color::rgb(0.025, 0.075, 0.095))
}

pub(crate) fn group_content() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(16)
    .margin((4, 0, 0, 0))
    .white_space(WhiteSpace::Normal)
}

pub(crate) fn action() -> Style {
  Style::new()
    .height(34)
    .background_color(BUTTON_BACKGROUND)
    .color(BUTTON_TEXT)
    .font_size(16)
    .margin((7, 0, 0, 0))
    .border_radius(7)
}

pub(crate) fn popup() -> Style {
  Style::new()
    .height(122)
    .padding(12)
    .background_color(Color::rgb(0.12, 0.08, 0.19))
    .color(ACCENT)
    .font_size(18)
    .white_space(WhiteSpace::Normal)
    .border_color((
      Color::rgb(0.42, 0.3, 0.58),
      Color::rgb(0.42, 0.3, 0.58),
      Color::rgb(0.42, 0.3, 0.58),
      Color::rgb(0.42, 0.3, 0.58),
    ))
    .border_width(1)
    .border_radius(9)
}

pub(crate) fn popup_content() -> Style {
  Style::new()
    .background_color(BACKGROUND)
    .color(PRIMARY_TEXT)
    .font_size(15)
    .padding((3, 7))
    .margin((4, 0, 0, 0))
    .border_radius(5)
}

pub(crate) fn help() -> Style {
  Style::new()
    .color(BODY_TEXT)
    .font_size(14)
    .white_space(WhiteSpace::Normal)
    .margin((6, 0, 0, 0))
}
