use battlement::{Align, Color, FlexDirection, LengthUnits, Style};

use crate::design_system::{ACCENT, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

pub(crate) fn intro() -> Style {
    Style::new()
        .color(Color::rgb(0.67, 0.78, 0.82))
        .font_size(17.0)
        .margin((2.0, 8.0, 10.0, 8.0))
        .white_space(battlement::WhiteSpace::Normal)
}

pub(crate) fn page_title() -> Style {
    Style::new().color(PRIMARY_TEXT).font_size(40.0).margin(8.0)
}

pub(crate) fn columns() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .flex_grow(1.0)
        .width(100.0_f32.pct())
}

pub(crate) fn card(width: f32, accent: bool) -> Style {
    Style::new()
        .width(width.pct())
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(if accent {
            CYAN
        } else {
            Color::rgb(0.18, 0.32, 0.35)
        })
        .border_width(1.0)
        .border_radius(16.0)
        .padding(14.0)
        .margin(7.0)
}

pub(crate) fn caption() -> Style {
    Style::new()
        .color(ACCENT)
        .font_size(14.0)
        .margin((0.0, 0.0, 7.0, 0.0))
}

pub(crate) fn mode_name(active: bool) -> Style {
    Style::new()
        .color(if active { CYAN } else { PRIMARY_TEXT })
        .font_size(18.0)
        .margin((4.0, 0.0))
}

pub(crate) fn detail() -> Style {
    Style::new()
        .color(Color::rgb(0.62, 0.73, 0.77))
        .font_size(13.0)
        .white_space(battlement::WhiteSpace::Normal)
        .margin((2.0, 0.0))
}

pub(crate) fn monitor() -> Style {
    Style::new()
        .height(248.0)
        .background_color(Color::rgb(0.015, 0.035, 0.045))
        .border_color(CYAN)
        .border_width(3.0)
        .border_radius(12.0)
        .padding(8.0)
}

pub(crate) fn monitor_image() -> Style {
    Style::new().width(100.0_f32.pct()).height(100.0_f32.pct())
}

pub(crate) fn target_root() -> Style {
    Style::new()
        .width(100.0_f32.pct())
        .height(100.0_f32.pct())
        .align_items(Align::Center)
        .justify_content(battlement::Justify::Center)
        .background_color(Color::rgb(0.015, 0.055, 0.07))
}

pub(crate) fn target_panel() -> Style {
    Style::new()
        .width(82.0_f32.pct())
        .height(68.0_f32.pct())
        .align_items(Align::Center)
        .justify_content(battlement::Justify::Center)
        .background_color(Color::rgb(0.04, 0.16, 0.2))
        .border_color(CYAN)
        .border_width(6.0)
        .border_radius(28.0)
}

pub(crate) fn target_title() -> Style {
    Style::new().color(PRIMARY_TEXT).font_size(20.0).margin(6.0)
}

pub(crate) fn target_status() -> Style {
    Style::new().color(CYAN).font_size(16.0).margin(4.0)
}
