use battlement::{Color, FlexDirection, Style};

const ACCENT: Color = Color::rgb(0.98, 0.72, 0.24);
const BACKGROUND: Color = Color::rgb(0.012, 0.025, 0.045);
const BODY_TEXT: Color = Color::rgb(0.86, 0.93, 0.95);
const BUTTON_BACKGROUND: Color = Color::rgb(0.08, 0.31, 0.38);
const BUTTON_TEXT: Color = Color::rgb(0.96, 0.99, 1.0);
const CYAN: Color = Color::rgb(0.32, 0.92, 0.96);
const NAVIGATION_BACKGROUND: Color = Color::rgb(0.025, 0.065, 0.085);
const NAVIGATION_ITEM_BACKGROUND: Color = Color::rgb(0.045, 0.12, 0.15);
const PRIMARY_TEXT: Color = Color::rgb(0.94, 0.98, 0.99);
const SPECIMEN_BACKGROUND: Color = Color::rgb(0.035, 0.09, 0.115);
const SUCCESS_BACKGROUND: Color = Color::rgb(0.04, 0.32, 0.18);

pub(crate) fn root() -> Style {
    Style::new()
        .background_color(BACKGROUND)
        .color(BODY_TEXT)
        .font_size(22.0)
        .flex_direction(FlexDirection::Row)
        .padding(20.0)
}

pub(crate) fn navigation() -> Style {
    Style::new()
        .width(280.0)
        .background_color(NAVIGATION_BACKGROUND)
        .padding(24.0)
}

pub(crate) fn brand() -> Style {
    Style::new().color(CYAN).font_size(30.0).margin(10.0)
}

pub(crate) fn navigation_item(active: bool) -> Style {
    Style::new()
        .background_color(if active {
            ACCENT
        } else {
            NAVIGATION_ITEM_BACKGROUND
        })
        .color(if active { BACKGROUND } else { PRIMARY_TEXT })
        .font_size(20.0)
        .padding(14.0)
        .margin(8.0)
}

pub(crate) fn canvas() -> Style {
    Style::new()
        .background_color(BACKGROUND)
        .flex_grow(1.0)
        .padding(36.0)
}

pub(crate) fn eyebrow() -> Style {
    Style::new().font_size(18.0).color(ACCENT).margin(4.0)
}

pub(crate) fn title() -> Style {
    Style::new().font_size(44.0).color(PRIMARY_TEXT).margin(8.0)
}

pub(crate) fn specimen() -> Style {
    Style::new()
        .background_color(SPECIMEN_BACKGROUND)
        .padding(28.0)
        .margin(18.0)
}

pub(crate) fn specimen_title() -> Style {
    Style::new().font_size(28.0).color(CYAN).margin(6.0)
}

pub(crate) fn component_value() -> Style {
    Style::new().font_size(24.0).color(PRIMARY_TEXT).margin(6.0)
}

pub(crate) fn command_button() -> Style {
    Style::new()
        .background_color(BUTTON_BACKGROUND)
        .color(BUTTON_TEXT)
        .padding(18.0)
        .margin(12.0)
        .font_size(22.0)
}

pub(crate) fn success() -> Style {
    Style::new()
        .background_color(SUCCESS_BACKGROUND)
        .padding(22.0)
        .margin(12.0)
}

pub(crate) fn success_text() -> Style {
    Style::new().font_size(26.0).color(PRIMARY_TEXT)
}
