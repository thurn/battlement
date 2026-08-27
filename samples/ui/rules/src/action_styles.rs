use battlement::{Align, Color, FlexDirection, LengthUnits, Style};

use crate::design_system::{ACCENT, BACKGROUND, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

pub(crate) fn intro() -> Style {
    Style::new()
        .color(Color::rgb(0.67, 0.78, 0.82))
        .font_size(17.0)
        .margin((2.0, 8.0, 10.0, 8.0))
        .white_space(battlement::WhiteSpace::Normal)
}

pub(crate) fn columns() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .flex_grow(1.0)
        .width(100.0_f32.pct())
}

pub(crate) fn card(accent: bool) -> Style {
    Style::new()
        .width(50.0_f32.pct())
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
        .font_size(15.0)
        .margin((0.0, 0.0, 7.0, 0.0))
}

pub(crate) fn help() -> Style {
    Style::new()
        .color(Color::rgb(0.65, 0.76, 0.79))
        .font_size(14.0)
        .margin((0.0, 0.0, 8.0, 0.0))
        .white_space(battlement::WhiteSpace::Normal)
}

pub(crate) fn scroll() -> Style {
    Style::new()
        .height(112.0)
        .background_color(BACKGROUND)
        .border_radius(10.0)
        .padding((6.0, 8.0))
        .margin((0.0, 0.0, 8.0, 0.0))
}

pub(crate) fn row() -> Style {
    Style::new()
        .color(Color::rgb(0.57, 0.68, 0.71))
        .font_size(15.0)
        .height(30.0)
        .padding((4.0, 8.0))
}

pub(crate) fn destination() -> Style {
    row()
        .color(CYAN)
        .background_color(Color::rgb(0.05, 0.23, 0.28))
        .border_radius(7.0)
}

pub(crate) fn selectable() -> Style {
    Style::new()
        .width(100.0_f32.pct())
        .height(44.0)
        .flex_shrink(0.0)
        .color(PRIMARY_TEXT)
        .background_color(Color::rgb(0.07, 0.19, 0.22))
        .font_size(19.0)
        .padding((10.0, 12.0))
        .border_color(CYAN)
        .border_width(1.0)
        .border_radius(9.0)
}

pub(crate) fn focus_probe() -> Style {
    Style::new()
        .color(ACCENT)
        .font_size(11.0)
        .height(18.0)
        .flex_shrink(0.0)
        .margin((0.0, 2.0, 3.0, 2.0))
}

pub(crate) fn selection_evidence(applied: bool) -> Style {
    Style::new()
        .width(100.0_f32.pct())
        .height(18.0)
        .flex_shrink(0.0)
        .color(if applied {
            CYAN
        } else {
            Color::rgb(0.57, 0.68, 0.71)
        })
        .font_size(12.0)
        .margin((3.0, 2.0, 0.0, 2.0))
}

pub(crate) fn toggle() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(16.0)
        .height(38.0)
        .margin((2.0, 0.0))
}

pub(crate) fn field() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(15.0)
        .height(52.0)
        .margin((2.0, 0.0))
}

pub(crate) fn field_input() -> Style {
    Style::new()
        .background_color(BACKGROUND)
        .border_color(Color::rgb(0.22, 0.48, 0.52))
        .border_width(1.0)
        .border_radius(5.0)
}

pub(crate) fn field_text() -> Style {
    Style::new().color(PRIMARY_TEXT).font_size(14.0)
}

pub(crate) fn slider() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .height(48.0)
        .margin((2.0, 0.0))
}

pub(crate) fn button() -> Style {
    Style::new()
        .background_color(Color::rgb(0.07, 0.32, 0.38))
        .color(PRIMARY_TEXT)
        .font_size(16.0)
        .height(38.0)
        .margin((7.0, 0.0, 5.0, 0.0))
}

pub(crate) fn status(complete: bool) -> Style {
    Style::new()
        .align_items(Align::Center)
        .background_color(if complete {
            Color::rgb(0.08, 0.28, 0.25)
        } else {
            BACKGROUND
        })
        .color(if complete {
            ACCENT
        } else {
            Color::rgb(0.69, 0.78, 0.8)
        })
        .font_size(13.0)
        .min_height(40.0)
        .padding((8.0, 10.0))
        .border_radius(8.0)
        .white_space(battlement::WhiteSpace::Normal)
}
