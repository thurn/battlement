use battlement::{Align, Color, FlexDirection, LengthUnits, Style, WhiteSpace};

use crate::design_system::{ACCENT, BACKGROUND, CYAN, PRIMARY_TEXT, SPECIMEN_BACKGROUND};

const MUTED: Color = Color::rgb(0.62, 0.72, 0.76);

pub(crate) fn main_layout() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .height(325)
        .margin((10, 0, 12, 0))
}

pub(crate) fn edit_surface() -> Style {
    Style::new()
        .width(62_f32.pct())
        .padding(18)
        .margin((0, 14, 0, 0))
        .background_color(SPECIMEN_BACKGROUND)
        .border_color(Color::rgb(0.18, 0.34, 0.38))
        .border_width(1)
        .border_radius(14)
}

pub(crate) fn inspector() -> Style {
    Style::new()
        .width(38_f32.pct())
        .padding(18)
        .background_color(Color::rgb(0.045, 0.105, 0.125))
        .border_radius(14)
}

pub(crate) fn caption() -> Style {
    Style::new().color(CYAN).font_size(19).margin((0, 0, 7, 0))
}

pub(crate) fn lead() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(22)
        .white_space(WhiteSpace::Normal)
        .margin((0, 0, 12, 0))
}

pub(crate) fn field() -> Style {
    Style::new()
        .height(58)
        .color(BACKGROUND)
        .font_size(20)
        .padding(8)
        .margin((0, 0, 10, 0))
        .background_color(Color::rgb(0.76, 0.87, 0.88))
        .border_color(Color::rgb(0.2, 0.5, 0.55))
        .border_width(1)
        .border_radius(8)
}

pub(crate) fn emphasized_field() -> Style {
    field().border_color(ACCENT).border_width(2)
}

pub(crate) fn inspector_state() -> Style {
    Style::new()
        .background_color(ACCENT)
        .color(BACKGROUND)
        .font_size(18)
        .padding(10)
        .border_radius(8)
        .margin((0, 0, 12, 0))
}

pub(crate) fn inspector_value() -> Style {
    Style::new()
        .color(PRIMARY_TEXT)
        .font_size(19)
        .white_space(WhiteSpace::Normal)
        .margin((0, 0, 8, 0))
}

pub(crate) fn inspector_note() -> Style {
    Style::new()
        .color(MUTED)
        .font_size(17)
        .white_space(WhiteSpace::Normal)
        .margin((4, 0, 0, 0))
}

pub(crate) fn specimen_row() -> Style {
    Style::new()
        .flex_direction(FlexDirection::Row)
        .align_items(Align::Stretch)
        .height(165)
}

pub(crate) fn specimen() -> Style {
    Style::new()
        .width(33.33_f32.pct())
        .padding(12)
        .margin((0, 8, 0, 0))
        .background_color(SPECIMEN_BACKGROUND)
        .border_radius(11)
}

pub(crate) fn final_specimen() -> Style {
    specimen().margin(0)
}

pub(crate) fn specimen_title() -> Style {
    Style::new().color(CYAN).font_size(18).margin((0, 0, 6, 0))
}

pub(crate) fn specimen_note() -> Style {
    Style::new()
        .color(MUTED)
        .font_size(16)
        .white_space(WhiteSpace::Normal)
        .margin((5, 0, 0, 0))
}

pub(crate) fn compact_field() -> Style {
    field().height(56).font_size(17).margin(0)
}

pub(crate) fn multiline_field() -> Style {
    compact_field().height(82)
}
