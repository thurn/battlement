//! Resettable source-sized route-transition specimen.

use std::time::Duration;

use crate::{
  arcade_frame_pulse::ArcadeScreen, arcade_menu_transition::ArcadeMenuTransition, frame_styles,
  portrait_viewport::PortraitViewport, review_button::ReviewButton, screen_frame::ScreenFrame,
  setting_row::DISPLAY_FONT,
};
use battlement::{Align, Color, FlexDirection, Gradient, Position, Style, TextAnchor};
use battlement_reactant::{hooks, paint::PaintStyle, prelude::*};
use trox::{ls, tx};

/// Controls direction, interruption, time, motion policy, and reset for Page 33.
#[builder]
pub struct MenuTransitionHarness;

impl Component for MenuTransitionHarness {
  fn render(&self) -> impl Render {
    let (screen, set_screen) = hooks::use_state(ArcadeScreen::Main);
    let (play_transition, set_play_transition) = hooks::use_state(false);
    let (reduce_motion, set_reduce_motion) = hooks::use_state(false);
    let (reset_generation, set_reset_generation) = hooks::use_state(0_u32);
    let clock = use_controlled_motion_clock();
    self::harness(
      screen,
      set_screen,
      play_transition,
      set_play_transition,
      reduce_motion,
      set_reduce_motion,
      reset_generation,
      set_reset_generation,
      clock,
    )
  }
}

#[allow(clippy::too_many_arguments)]
fn harness(
  screen: ArcadeScreen,
  set_screen: StateSetter<ArcadeScreen>,
  play_transition: bool,
  set_play_transition: StateSetter<bool>,
  reduce_motion: bool,
  set_reduce_motion: StateSetter<bool>,
  reset_generation: u32,
  set_reset_generation: StateSetter<u32>,
  clock: ControlledMotionClock,
) -> View {
  View::new()
    .name("menu-transition-harness")
    .style(Style::new().flex_grow(1).min_height(0).margin_top(24))
    .child((
      View::new().style(Style::new().height(196)).child((
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(12.0)
          .style(Style::new().height(98).align_items(Align::Center))
          .child((
            self::route_button(
              "MAIN",
              ArcadeScreen::Main,
              screen,
              &set_screen,
              &set_play_transition,
              &clock,
            ),
            self::route_button(
              "SETTINGS",
              ArcadeScreen::Settings,
              screen,
              &set_screen,
              &set_play_transition,
              &clock,
            ),
            ReviewButton::new()
              .label(tx(
                "MIDPOINT",
                "Advance the route transition to its visible midpoint.",
              ))
              .name("menu-transition-midpoint")
              .on_press(EventCallback::new({
                let clock = clock.clone();
                move |()| clock.set(Duration::from_millis(230))
              })),
          )),
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(12.0)
          .style(Style::new().height(98).align_items(Align::Center))
          .child((
            ReviewButton::new()
              .label(ls(if reduce_motion { "REDUCED" } else { "FULL" }))
              .name("menu-transition-motion-policy")
              .on_press(set_reduce_motion.update_callback(|value| !value)),
            ReviewButton::new()
              .label(tx("RESET", "Reset the menu transition specimen."))
              .name("menu-transition-reset")
              .on_press(EventCallback::new({
                let clock = clock.clone();
                move |()| {
                  clock.set(Duration::ZERO);
                  set_screen.set(ArcadeScreen::Main);
                  set_play_transition.set(false);
                  set_reduce_motion.set(false);
                  set_reset_generation.update(|generation| generation.wrapping_add(1));
                }
              })),
          )),
      )),
      View::new()
        .style(Style::new().flex_grow(1).min_height(0))
        .child(
          PortraitViewport::new().child(
            ScreenFrame::new().children(
              MotionConfig::new(
                ArcadeMenuTransition::new()
                  .screen_key(screen)
                  .children(MenuSpecimen::new().screen(screen))
                  .play_transition(play_transition)
                  .reduce_motion(reduce_motion)
                  .key(reset_generation),
              )
              .time_source(MotionTimeSource::Controlled(clock)),
            ),
          ),
        ),
    ))
}

fn route_button(
  label: &'static str,
  target: ArcadeScreen,
  current: ArcadeScreen,
  set_screen: &StateSetter<ArcadeScreen>,
  set_play_transition: &StateSetter<bool>,
  clock: &ControlledMotionClock,
) -> ReviewButton {
  ReviewButton::new()
    .label(ls(label))
    .name(format!(
      "menu-transition-route-{}",
      label.to_ascii_lowercase()
    ))
    .disabled(target == current)
    .on_press(EventCallback::new({
      let set_screen = set_screen.clone();
      let set_play_transition = set_play_transition.clone();
      let clock = clock.clone();
      move |()| {
        clock.set(Duration::ZERO);
        set_play_transition.set(true);
        set_screen.set(target);
      }
    }))
}

#[builder]
struct MenuSpecimen {
  #[builder(required)]
  screen: ArcadeScreen,
}

impl Component for MenuSpecimen {
  fn render(&self) -> impl Render {
    self::specimen(self.screen)
  }
}

fn specimen(screen: ArcadeScreen) -> impl Render {
  let (label, semantic_name, color, edge) = match screen {
    ArcadeScreen::Main => ("MAIN", "Main transition specimen", 0x5eefff, 0x076bff),
    ArcadeScreen::Settings => (
      "SETTINGS",
      "Settings transition specimen",
      0xff69d5,
      0x7a35ff,
    ),
  };
  Region::new(ls(semantic_name))
    .host_name(format!(
      "menu-transition-specimen-{}",
      label.to_ascii_lowercase()
    ))
    .style(Style::new().position(Position::Absolute).inset(0))
    .child(
      View::new()
        .style(
          Style::new()
            .position(Position::Absolute)
            .top(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
            .right(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
            .bottom(frame_styles::OUTER_BOTTOM + frame_styles::BORDER_THICKNESS)
            .left(frame_styles::OUTER_INSET + frame_styles::BORDER_THICKNESS)
            .center_content(),
        )
        .paint(
          PaintStyle::new().background(
            Gradient::radial([0.5, 0.45], [0.85, 0.68])
              .stop(0.0, Color::hex(edge).with_alpha(0.28))
              .stop(0.48, Color::hex(0x041126))
              .stop(1.0, Color::BLACK),
          ),
        )
        .child(
          Text::new(ls(label)).style(
            Style::new()
              .color(Color::hex(color))
              .unity_font_definition(DISPLAY_FONT)
              .font_size(132)
              .letter_spacing(8)
              .unity_text_align(TextAnchor::MiddleCenter),
          ),
        ),
    )
}
