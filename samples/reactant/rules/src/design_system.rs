use battlement::{Color, FlexDirection, LengthUnits, Style};

pub(crate) const ACCENT: Color = Color::rgb(0.98, 0.72, 0.24);
pub(crate) const BACKGROUND: Color = Color::rgb(0.012, 0.025, 0.045);
pub(crate) const BODY_TEXT: Color = Color::rgb(0.86, 0.93, 0.95);
pub(crate) const CARD_BACKGROUND: Color = Color::rgb(0.055, 0.13, 0.16);
pub(crate) const CYAN: Color = Color::rgb(0.32, 0.92, 0.96);
pub(crate) const NAVIGATION_BACKGROUND: Color = Color::rgb(0.025, 0.065, 0.085);
pub(crate) const PRIMARY_TEXT: Color = Color::rgb(0.94, 0.98, 0.99);
pub(crate) const SPECIMEN_BACKGROUND: Color = Color::rgb(0.035, 0.09, 0.115);

pub(crate) fn root() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .height(100.0_f32.pct())
    .background_color(BACKGROUND)
    .color(BODY_TEXT)
    .font_size(24.0)
    .flex_direction(FlexDirection::Row)
}

pub(crate) fn navigation() -> Style {
  Style::new()
    .width(300.0)
    .height(100.0_f32.pct())
    .flex_shrink(0)
    .background_color(NAVIGATION_BACKGROUND)
    .padding(20.0)
}

pub(crate) fn brand() -> Style {
  Style::new().color(CYAN).font_size(30.0).margin(8.0)
}

pub(crate) fn navigation_item() -> Style {
  Style::new()
    .background_color(ACCENT)
    .color(BACKGROUND)
    .border_width(0)
    .border_radius(4)
    .font_size(24.0)
    .padding((12, 16))
    .margin((8, 0))
}

pub(crate) fn canvas() -> Style {
  Style::new()
    .background_color(BACKGROUND)
    .flex_grow(1.0)
    .padding(36.0)
}

pub(crate) fn eyebrow() -> Style {
  Style::new().font_size(24.0).color(ACCENT).margin(4.0)
}

pub(crate) fn title() -> Style {
  Style::new().font_size(44.0).color(PRIMARY_TEXT).margin(8.0)
}

pub(crate) fn specimen() -> Style {
  Style::new()
    .background_color(SPECIMEN_BACKGROUND)
    .padding(28.0)
    .margin((18, 0))
}

pub(crate) fn specimen_title() -> Style {
  Style::new().font_size(28.0).color(CYAN).margin(6.0)
}

pub(crate) fn badge_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .margin((12, 0))
}

pub(crate) fn badge() -> Style {
  Style::new()
    .background_color(CARD_BACKGROUND)
    .padding(16.0)
    .margin((0, 8))
}

pub(crate) fn badge_text() -> Style {
  Style::new().font_size(24.0).color(BODY_TEXT)
}
