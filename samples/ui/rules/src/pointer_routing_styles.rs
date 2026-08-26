use battlement::{
    Align, Color, FlexDirection, Justify, LengthUnits, Style, TextAnchor, WhiteSpace,
};

use crate::design_system::{ACCENT, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

const INK: Color = Color::rgb(0.018, 0.055, 0.075);
const PANEL: Color = Color::rgb(0.045, 0.14, 0.17);
const MUTED: Color = Color::rgb(0.55, 0.66, 0.7);

pub(crate) fn intro() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(15)
        .white_space(WhiteSpace::Normal)
        .margin((0, 8, 10, 8))
}

pub(crate) fn columns() -> Style {
    Style::new().flex_direction(FlexDirection::Row).height(430)
}

pub(crate) fn route_card() -> Style {
    Style::new()
        .width(54.0_f32.pct())
        .padding(14)
        .margin((0, 8, 0, 0))
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(Color::rgb(0.12, 0.34, 0.39))
        .border_width(1)
        .border_radius(12)
}

pub(crate) fn inspector_card() -> Style {
    Style::new()
        .width(46.0_f32.pct())
        .padding(14)
        .margin((0, 0, 0, 8))
        .background_color(INK)
        .border_color(Color::rgb(0.12, 0.34, 0.39))
        .border_width(1)
        .border_radius(12)
}

pub(crate) fn caption() -> Style {
    Style::new().color(CYAN).font_size(16).margin((0, 0, 8, 0))
}

pub(crate) fn root(active: bool) -> Style {
    routed_box(active, 288.0, Color::rgb(0.04, 0.12, 0.15)).padding(18)
}

pub(crate) fn panel(active: bool) -> Style {
    routed_box(active, 202.0, PANEL).padding(18).margin((10, 4))
}

pub(crate) fn target(active: bool) -> Style {
    Style::new()
        .height(102)
        .margin((12, 8))
        .background_color(if active {
            ACCENT
        } else {
            Color::rgb(0.08, 0.31, 0.38)
        })
        .color(if active { INK } else { PRIMARY_TEXT })
        .border_color(if active {
            Color::rgb(1.0, 1.0, 1.0)
        } else {
            CYAN
        })
        .border_width(if active { 3 } else { 1 })
        .border_radius(10)
        .font_size(17)
}

pub(crate) fn node_label() -> Style {
    Style::new().color(MUTED).font_size(11)
}

pub(crate) fn route_strip() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .align_items(Align::Center)
        .justify_content(Justify::Center)
        .height(52)
        .margin((8, 0, 0, 0))
}

pub(crate) fn route_step(active: bool) -> Style {
    Style::new()
        .padding((6, 8))
        .margin(2)
        .background_color(if active { ACCENT } else { PANEL })
        .color(if active { INK } else { MUTED })
        .border_radius(6)
        .font_size(10)
        .unity_text_align(TextAnchor::MiddleCenter)
}

pub(crate) fn capture(active: bool) -> Style {
    Style::new()
        .padding((8, 10))
        .margin((0, 0, 10, 0))
        .background_color(if active {
            Color::rgb(0.04, 0.32, 0.18)
        } else {
            PANEL
        })
        .color(if active {
            Color::rgb(0.72, 1.0, 0.82)
        } else {
            MUTED
        })
        .border_radius(7)
        .font_size(13)
}

pub(crate) fn payload() -> Style {
    Style::new()
        .flex_grow(1)
        .padding(12)
        .background_color(Color::rgb(0.025, 0.075, 0.095))
        .color(PRIMARY_TEXT)
        .font_size(13)
        .white_space(WhiteSpace::Normal)
}

pub(crate) fn hint() -> Style {
    Style::new()
        .color(MUTED)
        .font_size(11)
        .white_space(WhiteSpace::Normal)
        .margin((10, 0, 0, 0))
}

fn routed_box(active: bool, height: f32, background: Color) -> Style {
    Style::new()
        .height(height)
        .background_color(background)
        .border_color(if active {
            ACCENT
        } else {
            Color::rgb(0.12, 0.34, 0.39)
        })
        .border_width(if active { 3 } else { 1 })
        .border_radius(10)
}
