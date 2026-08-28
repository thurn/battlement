use battlement::{
  Align, Color, EasingFunction, FlexDirection, FlexWrap, LengthUnits, Position, Style, TextAnchor,
  TransitionList, TransitionProperty, WhiteSpace,
};

const ACTION_HOVER: Color = Color::rgb(1.0, 0.79, 0.38);
const ACTION_PRESSED: Color = Color::rgb(0.78, 0.5, 0.12);
const NAVIGATION_HOVER: Color = Color::rgb(0.09, 0.24, 0.28);
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
    .padding(if compact { 10.0 } else { 20.0 });
  if compact {
    style
  } else {
    style.height(100.0_f32.pct())
  }
}

pub(crate) fn navigation_items(compact: bool) -> Style {
  let style = Style::new().flex_direction(if compact {
    FlexDirection::Row
  } else {
    FlexDirection::Column
  });
  if compact {
    style.width(100.0_f32.pct()).flex_wrap(FlexWrap::Wrap)
  } else {
    style
  }
}

pub(crate) fn brand(compact: bool) -> Style {
  Style::new()
    .color(CYAN)
    .font_size(if compact { 24.0 } else { 30.0 })
    .margin(if compact { (0, 4, 2, 4) } else { (8, 8, 8, 8) })
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
  let hovered = state == ControlState::Hovered;
  let style = Style::new()
    .height(if compact { 40.0 } else { 52.0 })
    .background_color(background)
    .color(if selected { CYAN } else { PRIMARY_TEXT })
    .border_color(CYAN)
    .border_width(if focused {
      2.0
    } else if hovered {
      1.0
    } else {
      0.0
    })
    .border_left_width(if selected { 3.0 } else { 0.0 })
    .border_radius(4)
    .font_size(if compact { 15.0 } else { 24.0 })
    .padding(if compact { (8, 10) } else { (12, 16) })
    .margin(if compact { (2, 2) } else { (8, 0) });
  if compact {
    style.min_width(100.0).flex_basis(100.0).flex_grow(1.0)
  } else {
    style
  }
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
    .min_height(0.0)
    .padding(if compact { (16, 20) } else { (36, 36) })
}

pub(crate) fn eyebrow() -> Style {
  Style::new().font_size(24.0).color(ACCENT).margin(4.0)
}

pub(crate) fn title() -> Style {
  Style::new().font_size(44.0).color(PRIMARY_TEXT).margin(8.0)
}

pub(crate) fn effects_title(compact: bool) -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .font_size(if compact { 30.0 } else { 44.0 })
    .color(PRIMARY_TEXT)
    .white_space(WhiteSpace::Normal)
    .margin(8.0)
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

pub(crate) fn effects_content() -> Style {
  Style::new().width(100.0_f32.pct())
}

pub(crate) fn effects_scroller() -> Style {
  Style::new()
    .width(10.0)
    .background_color(NAVIGATION_BACKGROUND)
}

pub(crate) fn effects_scroll_button() -> Style {
  Style::new()
    .height(8.0)
    .background_color(NAVIGATION_BACKGROUND)
    .border_width(0.0)
}

pub(crate) fn effects_scroll_track() -> Style {
  Style::new().background_color(BACKGROUND)
}

pub(crate) fn effects_scroll_dragger() -> Style {
  Style::new()
    .background_color(CYAN)
    .border_width(0.0)
    .border_radius(5.0)
}

pub(crate) fn effects_specimen(compact: bool) -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .max_width(660.0)
    .align_self(Align::FlexStart)
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin(if compact { (10, 0) } else { (18, 0) })
}

pub(crate) fn effect_card(compact: bool) -> Style {
  let style = Style::new()
    .background_color(SPECIMEN_BACKGROUND)
    .padding(20.0)
    .margin((0, 14, 14, 0));
  if compact {
    style.width(100.0_f32.pct()).max_width(360.0)
  } else {
    style.width(300.0)
  }
}

pub(crate) fn effect_heading() -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .font_size(24.0)
    .color(BODY_TEXT)
    .margin((0, 0, 2, 0))
}

pub(crate) fn effect_status() -> Style {
  Style::new()
    .color(CYAN)
    .font_size(28.0)
    .margin((12, 0, 4, 0))
}

pub(crate) fn effect_action(state: ControlState, forward: bool) -> Style {
  if forward {
    self::primary_action(state).width(260.0)
  } else {
    self::secondary_action(state)
      .width(260.0)
      .margin((14, 0, 4, 0))
  }
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

pub(crate) fn event_route(compact: bool) -> Style {
  Style::new()
    .flex_direction(if compact {
      FlexDirection::Column
    } else {
      FlexDirection::Row
    })
    .margin((8, 0, 0, 0))
}

pub(crate) fn event_specimen(compact: bool) -> Style {
  Style::new()
    .width(100.0_f32.pct())
    .max_width(920.0)
    .align_self(Align::FlexStart)
    .flex_direction(FlexDirection::Row)
    .flex_wrap(FlexWrap::Wrap)
    .margin(if compact { (10, 0) } else { (18, 0) })
}

pub(crate) fn event_source_card(compact: bool) -> Style {
  let style = Style::new()
    .background_color(SPECIMEN_BACKGROUND)
    .padding(20.0)
    .margin((0, 14, 14, 0));
  if compact {
    style.width(100.0_f32.pct()).max_width(340.0)
  } else {
    style.width(440.0)
  }
}

pub(crate) fn portal_card(compact: bool) -> Style {
  let style = Style::new()
    .background_color(STATE_ACTIVE_BACKGROUND)
    .border_color(CYAN)
    .border_width(1.0)
    .border_top_width(4.0)
    .border_radius(4)
    .min_height(140.0)
    .padding(20.0);
  if compact {
    style
      .width(100.0_f32.pct())
      .max_width(326.0)
      .margin((0, 0, 14, 14))
  } else {
    style.width(300.0).margin((10, 0, 14, 0))
  }
}

pub(crate) fn portal_connector(compact: bool) -> Style {
  Style::new()
    .width(if compact { 340.0 } else { 50.0 })
    .height(if compact { 36.0 } else { 140.0 })
    .align_self(Align::FlexStart)
    .color(CYAN)
    .font_size(24.0)
    .unity_text_align(TextAnchor::MiddleCenter)
    .white_space(WhiteSpace::Normal)
    .margin(0.0)
}

pub(crate) fn event_action(state: ControlState, forward: bool) -> Style {
  if forward {
    self::primary_action(state).width(260.0)
  } else {
    self::secondary_action(state)
      .width(260.0)
      .background_color(SPECIMEN_BACKGROUND)
      .border_width(if state == ControlState::Focused {
        3.0
      } else {
        1.0
      })
      .margin((14, 0, 4, 0))
  }
}

pub(crate) fn event_status_frame(compact: bool) -> Style {
  Style::new()
    .height(if compact { 190.0 } else { 58.0 })
    .position(Position::Relative)
}

pub(crate) fn event_step(compact: bool, active: bool, order: u32) -> Style {
  Style::new()
    .width(if compact { 220.0 } else { 112.0 })
    .height(42.0)
    .background_color(if active { CYAN } else { CARD_BACKGROUND })
    .color(if active { BACKGROUND } else { BODY_TEXT })
    .font_size(24.0)
    .unity_text_align(TextAnchor::MiddleCenter)
    .padding((8, 0))
    .transition_property(TransitionList::new([
      TransitionProperty::BackgroundColor,
      TransitionProperty::Color,
    ]))
    .transition_delay(TransitionList::new([
      (order as f32 * 90.0).into(),
      (order as f32 * 90.0).into(),
    ]))
    .transition_duration(TransitionList::new([160.0.into()]))
    .transition_timing_function(TransitionList::new([EasingFunction::EaseOut]))
}

pub(crate) fn event_arrow(compact: bool) -> Style {
  Style::new()
    .width(if compact { 220.0 } else { 25.0 })
    .height(if compact { 28.0 } else { 42.0 })
    .color(ACCENT)
    .font_size(24.0)
    .unity_text_align(TextAnchor::MiddleCenter)
}

pub(crate) fn event_ready() -> Style {
  Style::new()
    .align_self(Align::FlexStart)
    .background_color(CARD_BACKGROUND)
    .color(BODY_TEXT)
    .font_size(24.0)
    .padding((10, 14))
    .margin((8, 0, 0, 0))
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
