use battlement::{
  Align, Color, Display, FlexDirection, Justify, LengthUnits, Style, TextAnchor, WhiteSpace,
};

use crate::design_system::{ACCENT, CYAN, PRIMARY_TEXT};

const CONTRACT_SURFACE: Color = Color::rgb(0.025, 0.085, 0.105);
const MUTED_TEXT: Color = Color::rgb(0.68, 0.79, 0.82);

pub(crate) fn intro() -> Style {
  Style::new()
    .color(MUTED_TEXT)
    .font_size(16)
    .margin((0, 0, 18, 0))
}

pub(crate) fn page_title() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(38)
    .margin((0, 0, 4, 0))
}

pub(crate) fn composition() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
    .flex_grow(1)
}

pub(crate) fn preview_column() -> Style {
  Style::new()
    .width(512)
    .height(384)
    .flex_shrink(0)
    .border_color(Color::rgb(0.12, 0.50, 0.55))
    .border_width(1)
    .border_radius(10)
}

pub(crate) fn monitor_image() -> Style {
  Style::new().width(512).height(384).border_radius(10)
}

pub(crate) fn contracts() -> Style {
  Style::new()
    .flex_grow(1)
    .padding((0, 0, 0, 24))
    .justify_content(Justify::Center)
}

pub(crate) fn contract_heading() -> Style {
  Style::new()
    .color(MUTED_TEXT)
    .font_size(11)
    .margin((0, 0, 6, 0))
}

pub(crate) fn mode() -> Style {
  Style::new()
    .height(52)
    .justify_content(Justify::Center)
    .background_color(CONTRACT_SURFACE)
    .border_color(ACCENT)
    .border_left_width(3)
    .border_right_width(0)
    .border_top_width(0)
    .border_bottom_width(0)
    .padding((0, 14))
    .margin((0, 0, 8, 0))
}

pub(crate) fn mode_name() -> Style {
  Style::new().color(PRIMARY_TEXT).font_size(15)
}

pub(crate) fn details_button(focused: bool) -> Style {
  Style::new()
    .height(44)
    .background_color(Color::rgb(0.035, 0.12, 0.14))
    .color(CYAN)
    .border_color(if focused {
      ACCENT
    } else {
      Color::rgb(0.12, 0.40, 0.44)
    })
    .border_width(if focused { 3 } else { 1 })
    .border_radius(6)
    .font_size(13)
    .unity_text_align(TextAnchor::MiddleCenter)
    .margin((4, 0, 0, 0))
}

pub(crate) fn details(expanded: bool) -> Style {
  Style::new()
    .display(if expanded {
      Display::Flex
    } else {
      Display::None
    })
    .background_color(CONTRACT_SURFACE)
    .padding(12)
    .margin((8, 0, 0, 0))
}

pub(crate) fn detail() -> Style {
  Style::new()
    .color(MUTED_TEXT)
    .font_size(12)
    .white_space(WhiteSpace::Normal)
    .margin((2, 0))
}

pub(crate) fn detail_heading() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(11)
    .margin((8, 0, 2, 0))
}

pub(crate) fn target_root() -> Style {
  Style::new()
    .width(100.pct())
    .height(100.pct())
    .align_items(Align::Center)
    .justify_content(Justify::Center)
    .background_color(Color::rgb(0.012, 0.055, 0.07))
}

pub(crate) fn target_title() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(30)
    .margin((0, 0, 8, 0))
}

pub(crate) fn target_status() -> Style {
  Style::new().color(CYAN).font_size(18)
}
