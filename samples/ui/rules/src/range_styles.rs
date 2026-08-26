use battlement::{Color, FlexDirection, Justify, LengthUnits, Style, WhiteSpace};

use crate::design_system::{ACCENT, BACKGROUND, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

const MUTED: Color = Color::rgb(0.62, 0.72, 0.76);

pub(crate) fn intro() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(18)
        .white_space(WhiteSpace::Normal)
        .margin((0, 8, 14, 8))
}

pub(crate) fn gallery() -> Style {
    Style::new().flex_direction(FlexDirection::Row).height(430)
}

pub(crate) fn range_card() -> Style {
    Style::new()
        .width(54_f32.pct())
        .padding(24)
        .margin((0, 12, 0, 0))
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(Color::rgb(0.17, 0.36, 0.4))
        .border_width(1)
        .border_radius(14)
}

pub(crate) fn progress_card() -> Style {
    range_card().width(46_f32.pct()).margin(0)
}

pub(crate) fn caption() -> Style {
    Style::new().color(CYAN).font_size(19).margin((0, 0, 6, 0))
}

pub(crate) fn help() -> Style {
    Style::new()
        .color(MUTED)
        .font_size(15)
        .white_space(WhiteSpace::Normal)
        .margin((0, 0, 18, 0))
}

pub(crate) fn range_slider() -> Style {
    Style::new()
        .height(110)
        .margin((20, 8, 2, 8))
        .padding(12)
        .color(PRIMARY_TEXT)
}

pub(crate) fn endpoint_row() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .justify_content(Justify::SpaceBetween)
        .margin((0, 14, 24, 14))
}

pub(crate) fn endpoint() -> Style {
    Style::new().color(CYAN).font_size(15)
}

pub(crate) fn range_status() -> Style {
    Style::new()
        .background_color(ACCENT)
        .color(BACKGROUND)
        .font_size(18)
        .padding(12)
        .border_radius(8)
}

pub(crate) fn progress() -> Style {
    Style::new()
        .height(48)
        .margin((6, 2, 12, 2))
        .color(BACKGROUND)
        .font_size(15)
}
