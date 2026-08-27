use battlement::{Align, Color, FlexDirection, Justify, LengthUnits, Style, WhiteSpace};

use crate::design_system::{ACCENT, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

pub(crate) fn page_title() -> Style {
    Style::new().color(PRIMARY_TEXT).font_size(38.0).margin(8.0)
}

pub(crate) fn intro() -> Style {
    Style::new()
        .width(100.0_f32.pct())
        .color(Color::rgb(0.67, 0.78, 0.82))
        .font_size(24.0)
        .white_space(WhiteSpace::Normal)
        .margin((2.0, 8.0, 10.0, 8.0))
}

pub(crate) fn columns() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Column)
        .width(54.0_f32.pct())
        .height(390.0)
}

pub(crate) fn card() -> Style {
    Style::new()
        .width(100.0_f32.pct())
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(Color::rgb(0.18, 0.32, 0.35))
        .border_width(1.0)
        .border_radius(14.0)
        .padding(10.0)
        .margin(4.0)
}

pub(crate) fn caption() -> Style {
    Style::new()
        .color(ACCENT)
        .font_size(24.0)
        .white_space(WhiteSpace::Normal)
        .margin((0.0, 0.0, 7.0, 0.0))
}

pub(crate) fn detail() -> Style {
    Style::new()
        .color(Color::rgb(0.66, 0.78, 0.82))
        .font_size(24.0)
        .white_space(WhiteSpace::Normal)
}

pub(crate) fn line() -> Style {
    Style::new()
        .height(42.0)
        .flex_direction(FlexDirection::Row)
        .align_items(Align::Center)
        .border_bottom_color(Color::rgb(0.12, 0.24, 0.27))
        .border_bottom_width(1.0)
}

pub(crate) fn line_title() -> Style {
    Style::new()
        .width(40.0_f32.pct())
        .color(CYAN)
        .font_size(24.0)
        .white_space(WhiteSpace::Normal)
}

pub(crate) fn line_detail() -> Style {
    Style::new()
        .width(60.0_f32.pct())
        .color(Color::rgb(0.74, 0.84, 0.87))
        .font_size(24.0)
        .white_space(WhiteSpace::Normal)
}

pub(crate) fn monitor() -> Style {
    Style::new()
        .height(96.0)
        .background_color(Color::rgb(0.015, 0.035, 0.045))
        .border_color(CYAN)
        .border_width(2.0)
        .border_radius(10.0)
        .padding(6.0)
}

pub(crate) fn monitor_image() -> Style {
    Style::new().width(100.0_f32.pct()).height(100.0_f32.pct())
}

pub(crate) fn stage_footer() -> Style {
    Style::new()
        .flex_grow(1.0)
        .width(100.0_f32.pct())
        .background_color(Color::rgb(0.012, 0.025, 0.045))
        .border_top_color(Color::rgb(0.12, 0.25, 0.29))
        .border_top_width(1.0)
        .color(Color::rgb(0.46, 0.64, 0.68))
        .font_size(24.0)
        .padding((18.0, 12.0))
        .margin((0.0, -36.0, -36.0, -36.0))
}

pub(crate) fn world_root() -> Style {
    Style::new()
        .width(720.0)
        .height(430.0)
        .align_items(Align::Center)
        .justify_content(Justify::Center)
}

pub(crate) fn world_panel() -> Style {
    Style::new()
        .width(92.0_f32.pct())
        .height(86.0_f32.pct())
        .background_color(Color::rgba(0.02, 0.09, 0.12, 0.96))
        .border_color(CYAN)
        .border_width(5.0)
        .border_radius(24.0)
        .padding(22.0)
}

pub(crate) fn world_title() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(31.0)
        .margin((2.0, 0.0, 8.0, 0.0))
}

pub(crate) fn world_button() -> Style {
    Style::new()
        .background_color(Color::rgb(0.04, 0.38, 0.45))
        .border_color(CYAN)
        .border_width(3.0)
        .border_radius(12.0)
        .color(PRIMARY_TEXT)
        .font_size(24.0)
        .padding(16.0)
        .margin((18.0, 0.0, 10.0, 0.0))
}

pub(crate) fn world_status(active: bool) -> Style {
    Style::new()
        .background_color(if active {
            Color::rgb(0.04, 0.32, 0.18)
        } else {
            Color::rgb(0.04, 0.15, 0.18)
        })
        .color(if active { PRIMARY_TEXT } else { CYAN })
        .font_size(24.0)
        .padding(10.0)
        .border_radius(8.0)
}
