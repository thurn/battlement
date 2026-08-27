use battlement::{
  Align, Color, FlexDirection, Justify, LengthUnits, Style, TextAnchor, WhiteSpace,
};

use crate::design_system::{ACCENT, BACKGROUND, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

const MUTED: Color = Color::rgb(0.6, 0.7, 0.74);
const TEAL: Color = Color::rgb(0.06, 0.3, 0.35);

pub(crate) fn intro() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(18)
    .white_space(WhiteSpace::Normal)
    .margin((0, 8, 14, 8))
}

pub(crate) fn gallery() -> Style {
  Style::new().flex_direction(FlexDirection::Row).height(432)
}

pub(crate) fn card(width: f32) -> Style {
  Style::new()
    .width(width.pct())
    .padding(22)
    .margin((0, 10))
    .background_color(SPECIMEN_BACKGROUND)
    .border_color(Color::rgb(0.14, 0.35, 0.4))
    .border_width(1)
    .border_radius(14)
}

pub(crate) fn caption() -> Style {
  Style::new().color(CYAN).font_size(18).margin((0, 0, 5, 0))
}

pub(crate) fn help() -> Style {
  Style::new()
    .color(MUTED)
    .font_size(14)
    .white_space(WhiteSpace::Normal)
    .margin((0, 0, 14, 0))
}

pub(crate) fn specimen_row() -> Style {
  Style::new()
    .padding((11, 12))
    .margin((4, 0))
    .background_color(Color::rgb(0.025, 0.075, 0.095))
    .border_radius(8)
}

pub(crate) fn anatomy_label() -> Style {
  Style::new()
    .color(ACCENT)
    .font_size(12)
    .margin((0, 0, 5, 0))
}

pub(crate) fn button() -> Style {
  Style::new()
    .width(48)
    .height(48)
    .padding(10)
    .background_color(TEAL)
    .color(PRIMARY_TEXT)
    .border_color(CYAN)
    .border_width(1)
    .border_radius(9)
}

pub(crate) fn button_icon() -> Style {
  Style::new()
    .width(26)
    .height(26)
    .unity_background_image_tint_color(ACCENT)
}

pub(crate) fn button_line() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
}

pub(crate) fn action_label() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(15)
    .margin((0, 0, 0, 12))
}

pub(crate) fn toggle() -> Style {
  Style::new().height(58).color(PRIMARY_TEXT)
}

pub(crate) fn toggle_input() -> Style {
  Style::new()
    .padding(8)
    .background_color(TEAL)
    .border_color(CYAN)
    .border_width(1)
    .border_radius(8)
}

pub(crate) fn toggle_checkmark() -> Style {
  Style::new()
    .width(22)
    .height(22)
    .background_color(ACCENT)
    .border_radius(5)
}

pub(crate) fn control_text() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(16)
    .margin((0, 0, 0, 10))
}

pub(crate) fn dropdown() -> Style {
  Style::new().height(62).color(PRIMARY_TEXT)
}

pub(crate) fn dropdown_input() -> Style {
  Style::new()
    .height(46)
    .padding((0, 12))
    .background_color(TEAL)
    .border_color(CYAN)
    .border_width(1)
    .border_radius(8)
}

pub(crate) fn dropdown_arrow() -> Style {
  Style::new()
    .width(22)
    .height(22)
    .background_color(ACCENT)
    .border_radius(11)
}

pub(crate) fn progress() -> Style {
  Style::new().height(52).color(PRIMARY_TEXT).font_size(14)
}

pub(crate) fn progress_container() -> Style {
  Style::new()
    .height(42)
    .padding(4)
    .background_color(BACKGROUND)
    .border_color(CYAN)
    .border_width(1)
    .border_radius(9)
}

pub(crate) fn progress_background() -> Style {
  Style::new()
    .background_color(Color::rgb(0.04, 0.12, 0.15))
    .border_radius(6)
}

pub(crate) fn progress_fill() -> Style {
  Style::new().background_color(ACCENT).border_radius(6)
}

pub(crate) fn progress_title() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(14)
    .unity_text_align(TextAnchor::MiddleCenter)
}

pub(crate) fn legend() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .justify_content(Justify::SpaceBetween)
    .padding((9, 12))
    .margin((12, 10, 0, 10))
    .background_color(Color::rgb(0.05, 0.16, 0.18))
    .border_radius(7)
    .color(CYAN)
    .font_size(13)
}
