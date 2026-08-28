use battlement::{Align, Color, FlexDirection, FlexWrap, LengthUnits, Style};

const ACTION_HOVER: Color = Color::rgb(1.0, 0.79, 0.38);
const ACTION_PRESSED: Color = Color::rgb(0.78, 0.5, 0.12);
const NAVIGATION_HOVER: Color = Color::rgb(0.07, 0.18, 0.21);
const NAVIGATION_PRESSED: Color = Color::rgb(0.035, 0.1, 0.12);
const NAVIGATION_SELECTED: Color = Color::rgb(0.04, 0.18, 0.21);

pub(crate) const ACCENT: Color = Color::rgb(0.98, 0.72, 0.24);
pub(crate) const BACKGROUND: Color = Color::rgb(0.012, 0.025, 0.045);
pub(crate) const BODY_TEXT: Color = Color::rgb(0.86, 0.93, 0.95);
pub(crate) const CARD_BACKGROUND: Color = Color::rgb(0.055, 0.13, 0.16);
pub(crate) const CYAN: Color = Color::rgb(0.32, 0.92, 0.96);
pub(crate) const NAVIGATION_BACKGROUND: Color = Color::rgb(0.025, 0.065, 0.085);
pub(crate) const PRIMARY_TEXT: Color = Color::rgb(0.94, 0.98, 0.99);
pub(crate) const SPECIMEN_BACKGROUND: Color = Color::rgb(0.035, 0.09, 0.115);

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ControlState {
  Resting,
  Hovered,
  Pressed,
  Focused,
}

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
    .width(340.0)
    .height(100.0_f32.pct())
    .flex_shrink(0)
    .background_color(NAVIGATION_BACKGROUND)
    .padding(20.0)
}

pub(crate) fn brand() -> Style {
  Style::new().color(CYAN).font_size(30.0).margin(8.0)
}

pub(crate) fn navigation_item(selected: bool, state: ControlState) -> Style {
  let background = match state {
    ControlState::Pressed => NAVIGATION_PRESSED,
    ControlState::Hovered => NAVIGATION_HOVER,
    _ if selected => NAVIGATION_SELECTED,
    _ => CARD_BACKGROUND,
  };
  let focused = state == ControlState::Focused;
  Style::new()
    .height(52.0)
    .background_color(background)
    .color(if selected { CYAN } else { PRIMARY_TEXT })
    .border_color(CYAN)
    .border_width(if focused { 2.0 } else { 0.0 })
    .border_left_width(if selected { 4.0 } else { 0.0 })
    .border_radius(4)
    .font_size(24.0)
    .padding((12, 16))
    .margin((8, 0))
}

pub(crate) fn primary_action(state: ControlState) -> Style {
  let background = match state {
    ControlState::Hovered => ACTION_HOVER,
    ControlState::Pressed => ACTION_PRESSED,
    _ => ACCENT,
  };
  let focused = state == ControlState::Focused;
  Style::new()
    .width(220.0)
    .height(52.0)
    .align_self(Align::FlexStart)
    .background_color(background)
    .color(BACKGROUND)
    .border_color(CYAN)
    .border_width(if focused { 3.0 } else { 0.0 })
    .border_radius(4)
    .font_size(24.0)
    .padding((12, 20))
    .margin((14, 0, 4, 0))
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
    .width(100.0_f32.pct())
    .max_width(840.0)
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
    .flex_wrap(FlexWrap::Wrap)
    .margin((12, 0))
}

pub(crate) fn badge() -> Style {
  Style::new()
    .background_color(CARD_BACKGROUND)
    .padding(16.0)
    .margin((0, 8, 8, 0))
}

pub(crate) fn badge_text() -> Style {
  Style::new().font_size(24.0).color(BODY_TEXT)
}

pub(crate) fn event_route() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((14, 0, 4, 0))
}

pub(crate) fn event_step(active: bool) -> Style {
  Style::new()
    .background_color(if active { CYAN } else { CARD_BACKGROUND })
    .color(if active { BACKGROUND } else { BODY_TEXT })
    .font_size(24.0)
    .padding((10, 14))
    .margin((0, 6, 0, 0))
}

pub(crate) fn event_arrow() -> Style {
  Style::new()
    .color(ACCENT)
    .font_size(24.0)
    .padding((10, 4))
    .margin((0, 6, 0, 0))
}

pub(crate) fn event_ready() -> Style {
  Style::new()
    .align_self(Align::FlexStart)
    .background_color(CARD_BACKGROUND)
    .color(BODY_TEXT)
    .font_size(24.0)
    .padding((10, 14))
    .margin((14, 0, 4, 0))
}
