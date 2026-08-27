use battlement::{Align, Color, FlexDirection, FlexWrap, Justify, LengthUnits, Style, TextAnchor};

const PANEL: Color = Color::rgb(0.025, 0.09, 0.11);
const INK: Color = Color::rgb(0.92, 0.97, 0.98);
const MUTED: Color = Color::rgb(0.52, 0.66, 0.69);
const CYAN: Color = Color::rgb(0.20, 0.91, 0.98);
const AMBER: Color = Color::rgb(1.0, 0.69, 0.22);

pub(crate) fn intro() -> Style {
  Style::new()
    .color(INK)
    .font_size(15)
    .flex_shrink(0)
    .white_space(battlement::WhiteSpace::Normal)
    .margin((8, 0, 12, 0))
}

pub(crate) fn columns() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .align_items(Align::Stretch)
    .flex_grow(1)
}

pub(crate) fn card(left: bool) -> Style {
  Style::new()
    .width(50.pct())
    .padding(16)
    .margin(if left { (0, 8, 0, 0) } else { (0, 0, 0, 8) })
    .background_color(PANEL)
    .border_color(Color::rgb(0.08, 0.36, 0.40))
    .border_width(1)
    .border_radius(12)
}

pub(crate) fn caption() -> Style {
  Style::new().color(CYAN).font_size(14).margin((0, 0, 12, 0))
}

pub(crate) fn grid() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .justify_content(Justify::Center)
    .align_items(Align::Center)
    .flex_grow(1)
}

pub(crate) fn target(focused: bool) -> Style {
  Style::new()
    .width(44.pct())
    .height(92)
    .margin(8)
    .background_color(if focused {
      Color::rgb(0.10, 0.28, 0.32)
    } else {
      Color::rgb(0.04, 0.14, 0.17)
    })
    .color(INK)
    .border_color(if focused {
      AMBER
    } else {
      Color::rgb(0.10, 0.42, 0.46)
    })
    .border_width(if focused { 4 } else { 1 })
    .border_radius(10)
    .font_size(17)
    .unity_text_align(TextAnchor::MiddleCenter)
}

pub(crate) fn focus_status() -> Style {
  Style::new()
    .padding((9, 11))
    .background_color(Color::rgb(0.06, 0.18, 0.20))
    .color(AMBER)
    .border_radius(7)
    .font_size(13)
    .white_space(battlement::WhiteSpace::Normal)
    .margin((0, 0, 10, 0))
}

pub(crate) fn inspector() -> Style {
  Style::new()
    .flex_grow(1)
    .padding(14)
    .background_color(Color::rgb(0.015, 0.055, 0.065))
    .color(INK)
    .font_size(15)
    .white_space(battlement::WhiteSpace::Normal)
}

pub(crate) fn hint() -> Style {
  Style::new()
    .color(MUTED)
    .font_size(12)
    .white_space(battlement::WhiteSpace::Normal)
    .margin((12, 0, 0, 0))
}
