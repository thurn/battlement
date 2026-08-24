use battlement::{Color, FlexDirection, Style};

const AMBER: Color = Color::rgb(0.95, 0.68, 0.22);
const BACKGROUND: Color = Color::rgb(0.012, 0.025, 0.045);
const BODY_TEXT: Color = Color::rgb(0.78, 0.88, 0.92);
const CYAN: Color = Color::rgb(0.18, 0.9, 0.95);
const MUTED_TEXT: Color = Color::rgb(0.42, 0.58, 0.64);
const NAVIGATION_BACKGROUND: Color = Color::rgb(0.025, 0.065, 0.085);
const PRIMARY_TEXT: Color = Color::rgb(0.86, 0.95, 0.97);
const SPECIMEN_BACKGROUND: Color = Color::rgb(0.035, 0.09, 0.115);
const INSPECTOR_BACKGROUND: Color = Color::rgb(0.018, 0.045, 0.06);

pub(crate) fn root() -> Style {
    Style::new()
        .background_color(BACKGROUND)
        .color(BODY_TEXT)
        .flex_direction(FlexDirection::Row)
        .padding(18.0)
}

pub(crate) fn navigation() -> Style {
    Style::new()
        .width(250.0)
        .background_color(NAVIGATION_BACKGROUND)
        .padding(22.0)
}

pub(crate) fn inspector() -> Style {
    Style::new()
        .width(310.0)
        .background_color(INSPECTOR_BACKGROUND)
        .padding(22.0)
}

pub(crate) fn brand() -> Style {
    Style::new().color(CYAN).font_size(24.0).margin(8.0)
}

pub(crate) fn navigation_item(active: bool) -> Style {
    Style::new()
        .color(if active { AMBER } else { MUTED_TEXT })
        .font_size(15.0)
        .margin(9.0)
}

pub(crate) fn canvas() -> Style {
    Style::new()
        .background_color(BACKGROUND)
        .flex_grow(1.0)
        .padding(28.0)
}

pub(crate) fn eyebrow() -> Style {
    Style::new().font_size(14.0).color(AMBER)
}

pub(crate) fn title() -> Style {
    Style::new().font_size(38.0).color(PRIMARY_TEXT)
}

pub(crate) fn specimen() -> Style {
    Style::new()
        .background_color(SPECIMEN_BACKGROUND)
        .padding(24.0)
        .margin(18.0)
}

pub(crate) fn specimen_title() -> Style {
    Style::new().font_size(22.0).color(CYAN)
}

pub(crate) fn inspector_heading() -> Style {
    Style::new().font_size(20.0)
}

pub(crate) fn inspector_identity() -> Style {
    Style::new().font_size(12.0)
}
