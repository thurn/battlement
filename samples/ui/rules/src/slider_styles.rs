use battlement::{Align, Color, FlexDirection, Justify, LengthUnits, Style, WhiteSpace};

use crate::design_system::{ACCENT, BACKGROUND, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

const MUTED: Color = Color::rgb(0.62, 0.72, 0.76);

pub(crate) fn intro() -> Style {
  Style::new()
    .color(PRIMARY_TEXT)
    .font_size(19)
    .white_space(WhiteSpace::Normal)
    .margin((0, 8, 14, 8))
}

pub(crate) fn gallery() -> Style {
  Style::new().flex_direction(FlexDirection::Row).height(342)
}

pub(crate) fn card() -> Style {
  Style::new()
    .width(58_f32.pct())
    .padding(22)
    .margin((0, 12, 0, 0))
    .background_color(SPECIMEN_BACKGROUND)
    .border_color(Color::rgb(0.17, 0.36, 0.4))
    .border_width(1)
    .border_radius(14)
}

pub(crate) fn final_card() -> Style {
  card().width(42_f32.pct()).margin(0)
}

pub(crate) fn caption() -> Style {
  Style::new().color(CYAN).font_size(20).margin((0, 0, 6, 0))
}

pub(crate) fn help() -> Style {
  Style::new()
    .color(MUTED)
    .font_size(16)
    .white_space(WhiteSpace::Normal)
    .margin((0, 0, 18, 0))
}

pub(crate) fn horizontal_slider() -> Style {
  Style::new()
    .height(88)
    .margin((18, 6, 18, 6))
    .padding(8)
    .color(PRIMARY_TEXT)
    .font_size(18)
}

pub(crate) fn vertical_row() -> Style {
  Style::new()
    .height(220)
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
    .justify_content(Justify::Center)
}

pub(crate) fn vertical_slider() -> Style {
  Style::new()
    .width(150)
    .height(210)
    .padding(6)
    .color(PRIMARY_TEXT)
    .font_size(17)
}

pub(crate) fn scale() -> Style {
  Style::new()
    .height(180)
    .width(90)
    .margin((8, 0, 0, 8))
    .justify_content(Justify::SpaceBetween)
}

pub(crate) fn scale_label() -> Style {
  Style::new().color(MUTED).font_size(14)
}

pub(crate) fn final_value() -> Style {
  Style::new()
    .color(ACCENT)
    .font_size(20)
    .margin((8, 6, 0, 6))
}

pub(crate) fn inspector() -> Style {
  Style::new()
    .height(84)
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Center)
    .padding(13)
    .margin((12, 0, 0, 0))
    .background_color(Color::rgb(0.04, 0.105, 0.125))
    .border_radius(12)
}

pub(crate) fn live_status() -> Style {
  Style::new()
    .width(48_f32.pct())
    .color(CYAN)
    .font_size(16)
    .margin((0, 12, 0, 0))
}

pub(crate) fn commit_status() -> Style {
  Style::new()
    .width(52_f32.pct())
    .background_color(ACCENT)
    .color(BACKGROUND)
    .font_size(16)
    .padding(9)
    .border_radius(7)
}
