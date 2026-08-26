use battlement::{Color, FlexDirection, LengthUnits, Style, WhiteSpace};

use crate::design_system::{
    ACCENT, BACKGROUND, BODY_TEXT, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND,
};

pub(crate) fn layout() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .width(100_f32.pct())
        .height(535)
        .margin((12, 0, 0, 0))
}

pub(crate) fn scroll_specimen() -> Style {
    Style::new()
        .width(62_f32.pct())
        .height(520)
        .padding(16)
        .margin((0, 14, 0, 0))
        .background_color(SPECIMEN_BACKGROUND)
        .border_radius(14)
}

pub(crate) fn control_specimen() -> Style {
    Style::new()
        .width(36_f32.pct())
        .height(520)
        .padding(20)
        .background_color(Color::rgb(0.055, 0.115, 0.145))
        .border_radius(14)
}

pub(crate) fn caption() -> Style {
    Style::new().color(CYAN).font_size(24).margin((0, 0, 10, 0))
}

pub(crate) fn primary_scroll() -> Style {
    Style::new()
        .width(100_f32.pct())
        .height(390)
        .background_color(BACKGROUND)
        .border_color((CYAN, CYAN, CYAN, CYAN))
        .border_width(2)
        .border_radius(10)
}

pub(crate) fn map() -> Style {
    Style::new()
        .width(790)
        .height(590)
        .padding(24)
        .background_color(Color::rgb(0.035, 0.15, 0.16))
}

pub(crate) fn map_title() -> Style {
    Style::new()
        .color(ACCENT)
        .font_size(30)
        .margin((0, 0, 18, 0))
}

pub(crate) fn map_note() -> Style {
    Style::new()
        .width(360)
        .color(PRIMARY_TEXT)
        .font_size(24)
        .margin((28, 0, 0, 120))
}

pub(crate) fn gallery() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .width(900)
        .height(145)
        .margin((140, 0, 0, 0))
}

pub(crate) fn card() -> Style {
    Style::new()
        .width(205)
        .height(125)
        .margin((6, 10))
        .padding(12)
        .background_color(Color::rgb(0.09, 0.25, 0.28))
        .border_radius(9)
}

pub(crate) fn card_title() -> Style {
    Style::new().color(ACCENT).font_size(24)
}

pub(crate) fn card_status() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(24)
        .margin((12, 0, 0, 0))
}

pub(crate) fn status() -> Style {
    Style::new().color(CYAN).font_size(24).margin((10, 0, 0, 0))
}

pub(crate) fn control_heading() -> Style {
    Style::new()
        .width(100_f32.pct())
        .color(PRIMARY_TEXT)
        .font_size(28)
        .white_space(WhiteSpace::Normal)
        .margin((18, 0, 54, 0))
}

pub(crate) fn scroller() -> Style {
    Style::new()
        .width(100_f32.pct())
        .height(74)
        .margin((12, 0, 34, 0))
}

pub(crate) fn value() -> Style {
    Style::new()
        .color(BACKGROUND)
        .background_color(ACCENT)
        .font_size(32)
        .padding(18)
        .border_radius(10)
}

pub(crate) fn control_note() -> Style {
    Style::new()
        .width(100_f32.pct())
        .color(BODY_TEXT)
        .font_size(24)
        .white_space(WhiteSpace::Normal)
        .margin((28, 0, 0, 0))
}
