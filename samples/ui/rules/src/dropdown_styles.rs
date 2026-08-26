use battlement::{Align, Color, FlexDirection, Justify, LengthUnits, Style, WhiteSpace};

use crate::design_system::{ACCENT, BACKGROUND, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

const MUTED: Color = Color::rgb(0.62, 0.72, 0.76);

pub(crate) fn intro() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(20)
        .white_space(WhiteSpace::Normal)
        .margin((0, 8, 16, 8))
}

pub(crate) fn gallery() -> Style {
    Style::new().flex_direction(FlexDirection::Row).height(330)
}

pub(crate) fn card() -> Style {
    Style::new()
        .width(50_f32.pct())
        .padding(22)
        .margin((0, 12, 0, 0))
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(Color::rgb(0.17, 0.36, 0.4))
        .border_width(1)
        .border_radius(14)
}

pub(crate) fn final_card() -> Style {
    card().margin(0)
}

pub(crate) fn caption() -> Style {
    Style::new().color(CYAN).font_size(20).margin((0, 0, 7, 0))
}

pub(crate) fn help() -> Style {
    Style::new()
        .color(MUTED)
        .font_size(17)
        .white_space(WhiteSpace::Normal)
        .margin((0, 0, 16, 0))
}

pub(crate) fn dropdown() -> Style {
    Style::new()
        .height(68)
        .color(PRIMARY_TEXT)
        .font_size(19)
        .padding(8)
        .background_color(Color::rgb(0.055, 0.145, 0.17))
        .border_color(CYAN)
        .border_width(1)
        .border_radius(9)
}

pub(crate) fn clear_button() -> Style {
    Style::new()
        .height(46)
        .margin((10, 0, 0, 0))
        .background_color(ACCENT)
        .color(BACKGROUND)
        .font_size(17)
        .border_radius(7)
}

pub(crate) fn selection_summary() -> Style {
    Style::new().color(CYAN).font_size(16).margin((12, 0, 0, 0))
}

pub(crate) fn inspector() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .align_items(Align::Center)
        .justify_content(Justify::SpaceBetween)
        .height(86)
        .padding(14)
        .margin((12, 0, 0, 0))
        .background_color(Color::rgb(0.04, 0.105, 0.125))
        .border_radius(12)
}

pub(crate) fn status() -> Style {
    Style::new()
        .width(46_f32.pct())
        .background_color(ACCENT)
        .color(BACKGROUND)
        .font_size(16)
        .padding(9)
        .margin((0, 10, 0, 0))
        .border_radius(7)
}

pub(crate) fn history() -> Style {
    Style::new()
        .width(54_f32.pct())
        .color(PRIMARY_TEXT)
        .font_size(16)
        .white_space(WhiteSpace::Normal)
}
