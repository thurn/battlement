use battlement::{Align, Color, FlexDirection, Style, WhiteSpace};

use crate::design_system::{ACCENT, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

const MUTED: Color = Color::rgb(0.62, 0.72, 0.76);
const PANEL: Color = Color::rgb(0.025, 0.07, 0.09);

pub(crate) fn layout() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .align_items(Align::Stretch)
        .height(490)
        .margin((12, 0, 0, 0))
}

pub(crate) fn workspace() -> Style {
    Style::new()
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(Color::rgb(0.22, 0.31, 0.34))
        .border_width(1)
        .border_radius(15)
        .padding(18)
        .flex_grow(1)
        .margin((0, 14, 0, 0))
}

pub(crate) fn inspector() -> Style {
    Style::new()
        .width(250)
        .flex_shrink(0)
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(Color::rgb(0.22, 0.31, 0.34))
        .border_width(1)
        .border_radius(15)
        .padding(16)
}

pub(crate) fn caption() -> Style {
    Style::new()
        .color(CYAN)
        .font_size(21)
        .white_space(WhiteSpace::Normal)
        .margin((0, 0, 14, 0))
}

pub(crate) fn tab_view() -> Style {
    Style::new()
        .height(390)
        .background_color(PANEL)
        .border_color(CYAN)
        .border_width(1)
        .border_radius(8)
}

pub(crate) fn content() -> Style {
    Style::new().height(320).background_color(PANEL).padding(30)
}

pub(crate) fn content_title() -> Style {
    Style::new()
        .color(ACCENT)
        .font_size(32)
        .margin((0, 0, 18, 0))
}

pub(crate) fn content_detail() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(27)
        .margin((0, 0, 28, 0))
}

pub(crate) fn content_note() -> Style {
    Style::new()
        .color(MUTED)
        .font_size(21)
        .white_space(WhiteSpace::Normal)
}

pub(crate) fn inspector_title() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(24)
        .white_space(WhiteSpace::Normal)
        .margin((8, 0, 24, 0))
}

pub(crate) fn status() -> Style {
    Style::new()
        .background_color(ACCENT)
        .color(Color::rgb(0.02, 0.04, 0.05))
        .font_size(18)
        .white_space(WhiteSpace::Normal)
        .padding(14)
        .border_radius(10)
        .margin((0, 0, 28, 0))
}

pub(crate) fn help() -> Style {
    Style::new()
        .color(MUTED)
        .font_size(18)
        .white_space(WhiteSpace::Normal)
}
