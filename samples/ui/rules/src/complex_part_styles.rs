use battlement::{
  Align, Color, Display, FlexDirection, FlexWrap, Justify, LengthUnits, Style, TextAnchor,
  WhiteSpace,
};

use crate::design_system::{ACCENT, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

const MUTED: Color = Color::rgb(0.58, 0.68, 0.72);
const INK: Color = Color::rgb(0.025, 0.075, 0.095);
const TEAL: Color = Color::rgb(0.055, 0.24, 0.28);

pub(crate) fn intro() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(16)
    .white_space(WhiteSpace::Normal)
    .margin((0, 8, 10, 8))
}

pub(crate) fn gallery() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .height(446)
}

pub(crate) fn card() -> Style {
  Style::new()
    .width(48.pct())
    .height(208)
    .padding((14, 16))
    .margin(7)
    .background_color(SPECIMEN_BACKGROUND)
    .border_color(Color::rgb(0.14, 0.35, 0.4))
    .border_width(1)
    .border_radius(12)
}

pub(crate) fn caption() -> Style {
  Style::new().color(CYAN).font_size(16).margin((0, 0, 3, 0))
}

pub(crate) fn anatomy() -> Style {
  Style::new()
    .color(ACCENT)
    .font_size(11)
    .margin((0, 0, 5, 0))
}

pub(crate) fn slider() -> Style {
  Style::new().height(64).color(PRIMARY_TEXT)
}
pub(crate) fn slider_label() -> Style {
  Style::new().width(100).color(PRIMARY_TEXT).font_size(14)
}
pub(crate) fn slider_track() -> Style {
  Style::new()
    .height(12)
    .background_color(INK)
    .border_radius(6)
}
pub(crate) fn slider_fill() -> Style {
  Style::new()
    .height(12)
    .background_color(ACCENT)
    .border_radius(6)
}
pub(crate) fn slider_dragger() -> Style {
  Style::new()
    .width(20)
    .height(20)
    .background_color(CYAN)
    .border_color(PRIMARY_TEXT)
    .border_width(2)
    .border_radius(10)
}
pub(crate) fn slider_input() -> Style {
  Style::new()
    .width(68)
    .color(PRIMARY_TEXT)
    .background_color(TEAL)
}
pub(crate) fn text_field() -> Style {
  Style::new().height(48).color(PRIMARY_TEXT).font_size(12)
}
pub(crate) fn text_input() -> Style {
  Style::new()
    .height(42)
    .background_color(INK)
    .border_color(CYAN)
    .border_width(1)
    .border_radius(6)
}
pub(crate) fn text_copy() -> Style {
  Style::new().color(PRIMARY_TEXT).font_size(12)
}
pub(crate) fn multiline_scroll() -> Style {
  Style::new().height(42).background_color(INK)
}
pub(crate) fn multiline_scroller() -> Style {
  Style::new().width(10).background_color(TEAL)
}
pub(crate) fn multiline_dragger() -> Style {
  Style::new()
    .width(7)
    .background_color(ACCENT)
    .border_radius(4)
}
pub(crate) fn scroll() -> Style {
  Style::new()
    .height(105)
    .background_color(INK)
    .border_color(CYAN)
    .border_width(1)
    .border_radius(8)
}
pub(crate) fn viewport() -> Style {
  Style::new()
    .padding(10)
    .background_color(Color::rgb(0.035, 0.11, 0.13))
}
pub(crate) fn content() -> Style {
  Style::new().height(165).color(PRIMARY_TEXT).font_size(13)
}
pub(crate) fn scroller() -> Style {
  Style::new().width(15).background_color(TEAL)
}
pub(crate) fn scroll_dragger() -> Style {
  Style::new()
    .width(9)
    .background_color(ACCENT)
    .border_radius(5)
}
pub(crate) fn tab_view() -> Style {
  Style::new()
    .height(112)
    .border_color(CYAN)
    .border_width(1)
    .border_radius(8)
}
pub(crate) fn tab_headers() -> Style {
  Style::new().height(38).background_color(INK)
}
pub(crate) fn tab_header() -> Style {
  Style::new()
    .padding((6, 11))
    .color(PRIMARY_TEXT)
    .background_color(TEAL)
}
pub(crate) fn tab_label() -> Style {
  Style::new().color(PRIMARY_TEXT).font_size(14)
}
pub(crate) fn tab_icon(active: bool) -> Style {
  Style::new()
    .display(if active { Display::Flex } else { Display::None })
    .width(16)
    .height(16)
    .margin((0, 5, 0, 0))
}
pub(crate) fn tab_underline() -> Style {
  Style::new().height(3).background_color(ACCENT)
}
pub(crate) fn tab_close() -> Style {
  Style::new()
    .width(18)
    .height(18)
    .background_color(ACCENT)
    .color(INK)
    .border_radius(9)
}
pub(crate) fn tab_content() -> Style {
  Style::new()
    .padding(12)
    .background_color(Color::rgb(0.035, 0.11, 0.13))
}
pub(crate) fn tab_copy() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(13)
    .white_space(WhiteSpace::Normal)
}
pub(crate) fn conditional_title() -> Style {
  Style::new()
    .color(ACCENT)
    .font_size(11)
    .margin((0, 0, 2, 0))
}
pub(crate) fn options() -> Style {
  Style::new().height(80).color(PRIMARY_TEXT).font_size(14)
}
pub(crate) fn all_options() -> Style {
  Style::new()
    .height(27)
    .padding((3, 8))
    .margin((1, 0))
    .background_color(TEAL)
    .border_radius(6)
}
pub(crate) fn all_options_state(active: bool) -> Style {
  Style::new().background_color(if active {
    Color::rgb(0.075, 0.29, 0.33)
  } else {
    TEAL
  })
}
pub(crate) fn highlighted_option() -> Style {
  Style::new()
    .background_color(Color::rgb(0.35, 0.22, 0.06))
    .border_color(ACCENT)
    .border_width(1)
}
pub(crate) fn highlighted_text() -> Style {
  Style::new().color(ACCENT).font_size(14)
}
pub(crate) fn toggle_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
    .justify_content(Justify::SpaceBetween)
    .margin((4, 0, 0, 0))
}
pub(crate) fn toggle_button(active: bool) -> Style {
  Style::new()
    .height(34)
    .padding((0, 13))
    .background_color(if active { ACCENT } else { TEAL })
    .color(if active { INK } else { PRIMARY_TEXT })
    .border_color(CYAN)
    .border_width(1)
    .border_radius(7)
}
pub(crate) fn state(active: bool) -> Style {
  Style::new()
    .color(if active { ACCENT } else { MUTED })
    .font_size(12)
    .unity_text_align(TextAnchor::MiddleRight)
}
