use battlement::{
  Align, Color, EasingFunction, FlexDirection, FlexWrap, LengthUnits, Position, Style,
  TransitionList, TransitionProperty,
};

const ACTION_HOVER: Color = Color::rgb(1.0, 0.79, 0.38);
const ACTION_PRESSED: Color = Color::rgb(0.78, 0.5, 0.12);
const NAVIGATION_HOVER: Color = Color::rgb(0.07, 0.18, 0.21);
const NAVIGATION_PRESSED: Color = Color::rgb(0.035, 0.1, 0.12);
const NAVIGATION_SELECTED: Color = Color::rgb(0.04, 0.18, 0.21);
const STATE_ACTIVE_BACKGROUND: Color = Color::rgb(0.07, 0.17, 0.2);

pub(crate) const ACCENT: Color = Color::rgb(0.98, 0.72, 0.24);
pub(crate) const BACKGROUND: Color = Color::rgb(0.012, 0.025, 0.045);
pub(crate) const BODY_TEXT: Color = Color::rgb(0.86, 0.93, 0.95);
pub(crate) const CARD_BACKGROUND: Color = Color::rgb(0.055, 0.13, 0.16);
pub(crate) const CONTEXT_OVERRIDE: Color = Color::rgb(0.7, 0.58, 0.96);
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

pub(crate) fn root(compact: bool) -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .height(100.0_f32.pct())
    .background_color(BACKGROUND)
    .color(BODY_TEXT)
    .font_size(24.0)
    .flex_direction(if compact {
      FlexDirection::Column
    } else {
      FlexDirection::Row
    })
}

pub(crate) fn navigation(compact: bool) -> Style {
  let style = Style::new()
    .width(if compact {
      100.0_f32.pct()
    } else {
      340.0.into()
    })
    .flex_shrink(0)
    .background_color(NAVIGATION_BACKGROUND)
    .padding(if compact { 14.0 } else { 20.0 });
  if compact {
    style.height(136.0)
  } else {
    style.height(100.0_f32.pct())
  }
}

pub(crate) fn navigation_items(compact: bool) -> Style {
  Style::new().flex_direction(if compact {
    FlexDirection::Row
  } else {
    FlexDirection::Column
  })
}

pub(crate) fn brand(compact: bool) -> Style {
  Style::new()
    .color(CYAN)
    .font_size(if compact { 26.0 } else { 30.0 })
    .margin(if compact { (2, 8, 4, 8) } else { (8, 8, 8, 8) })
}

pub(crate) fn navigation_item(selected: bool, state: ControlState, compact: bool) -> Style {
  let background = if state == ControlState::Pressed {
    NAVIGATION_PRESSED
  } else if state == ControlState::Hovered {
    NAVIGATION_HOVER
  } else if selected {
    NAVIGATION_SELECTED
  } else {
    CARD_BACKGROUND
  };
  let focused = state == ControlState::Focused;
  let style = Style::new()
    .height(if compact { 48.0 } else { 52.0 })
    .background_color(background)
    .color(if selected { CYAN } else { PRIMARY_TEXT })
    .border_color(CYAN)
    .border_width(if focused { 2.0 } else { 0.0 })
    .border_left_width(if selected { 4.0 } else { 0.0 })
    .border_radius(4)
    .font_size(if compact { 17.0 } else { 24.0 })
    .padding(if compact { (10, 12) } else { (12, 16) })
    .margin(if compact { (4, 4) } else { (8, 0) });
  if compact { style.flex_grow(1.0) } else { style }
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

pub(crate) fn canvas(compact: bool) -> Style {
  Style::new()
    .background_color(BACKGROUND)
    .flex_grow(1.0)
    .padding(if compact { (20, 28) } else { (36, 36) })
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

pub(crate) fn state_specimen() -> Style {
  Style::new().align_self(Align::FlexStart).margin((18, 0))
}

pub(crate) fn context_specimen() -> Style {
  Style::new()
    .max_width(600.0)
    .align_self(Align::FlexStart)
    .margin((14, 0, 0, 0))
}

pub(crate) fn context_control() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .align_items(Align::Center)
}

pub(crate) fn context_action(state: ControlState) -> Style {
  self::primary_action(state)
    .width(260.0)
    .margin((8, 0, 4, 0))
}

pub(crate) fn secondary_action(state: ControlState) -> Style {
  let background = match state {
    ControlState::Hovered => NAVIGATION_HOVER,
    ControlState::Pressed => NAVIGATION_PRESSED,
    _ => CARD_BACKGROUND,
  };
  Style::new()
    .width(220.0)
    .height(52.0)
    .align_self(Align::FlexStart)
    .background_color(background)
    .color(PRIMARY_TEXT)
    .border_color(CYAN)
    .border_width(if state == ControlState::Focused {
      3.0
    } else {
      0.0
    })
    .border_radius(4)
    .font_size(24.0)
    .padding((12, 20))
    .margin((14, 0, 4, 12))
}

pub(crate) fn memo_action(state: ControlState) -> Style {
  self::secondary_action(state)
    .width(260.0)
    .margin((8, 0, 4, 0))
}

pub(crate) fn experiment_title() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .font_size(24.0)
    .color(CYAN)
    .margin((0, 0, 2, 0))
}

pub(crate) fn memo_experiment() -> Style {
  Style::new()
    .max_width(600.0)
    .align_self(Align::FlexStart)
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .align_items(Align::Center)
    .margin((14, 0, 0, 0))
}

pub(crate) fn context_counter() -> Style {
  Style::new()
    .font_size(24.0)
    .color(CYAN)
    .margin((8, 0, 4, 14))
}

pub(crate) fn context_row() -> Style {
  Style::new()
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((12, 0, 0, 0))
}

pub(crate) fn context_card(accent: Color) -> Style {
  Style::new()
    .width(240.0)
    .background_color(CARD_BACKGROUND)
    .border_left_width(4.0)
    .border_color(accent)
    .padding(18.0)
    .margin((0, 12, 0, 0))
}

pub(crate) fn context_scope() -> Style {
  Style::new().font_size(24.0).color(BODY_TEXT)
}

pub(crate) fn context_theme(color: Color) -> Style {
  Style::new()
    .font_size(24.0)
    .color(color)
    .margin((8, 0, 0, 0))
}

pub(crate) fn effects_specimen() -> Style {
  Style::new()
    .max_width(660.0)
    .align_self(Align::FlexStart)
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin((18, 0))
}

pub(crate) fn effect_card() -> Style {
  Style::new()
    .width(300.0)
    .background_color(SPECIMEN_BACKGROUND)
    .padding(20.0)
    .margin((0, 14, 14, 0))
}

pub(crate) fn effect_status(connected: bool) -> Style {
  Style::new()
    .color(if connected { CYAN } else { BODY_TEXT })
    .font_size(28.0)
    .margin((12, 0, 4, 0))
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

pub(crate) fn event_experiment() -> Style {
  Style::new()
    .align_self(Align::FlexStart)
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .align_items(Align::Center)
    .margin((18, 0))
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

pub(crate) fn state_value() -> Style {
  Style::new()
    .color(CYAN)
    .font_size(28.0)
    .margin((6, 0, 16, 0))
}

pub(crate) fn identity_row() -> Style {
  Style::new()
    .width(560.0)
    .height(116.0)
    .flex_direction(FlexDirection::Row)
    .position(Position::Relative)
}

pub(crate) fn identity_token(position: f32, active: bool) -> Style {
  Style::new()
    .width(180.0)
    .position(Position::Absolute)
    .left(position * 190.0)
    .top(0.0)
    .background_color(if active {
      STATE_ACTIVE_BACKGROUND
    } else {
      CARD_BACKGROUND
    })
    .border_left_width(4.0)
    .border_color(CYAN)
    .padding(16.0)
    .transition_property(TransitionList::new([
      TransitionProperty::Left,
      TransitionProperty::BackgroundColor,
    ]))
    .transition_duration(TransitionList::new([220.0.into(), 180.0.into()]))
    .transition_timing_function(TransitionList::new([EasingFunction::EaseOutCubic]))
}

pub(crate) fn identity_state(active: bool) -> Style {
  Style::new()
    .color(if active { CYAN } else { BODY_TEXT })
    .font_size(24.0)
    .margin((8, 0, 0, 0))
    .transition_property(TransitionList::new([TransitionProperty::Color]))
    .transition_duration(TransitionList::new([180.0.into()]))
    .transition_timing_function(TransitionList::new([EasingFunction::EaseOut]))
}
