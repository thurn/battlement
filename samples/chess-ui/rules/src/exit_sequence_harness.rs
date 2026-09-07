//! Resettable source-sized specimen for the synchronized exit collapse.

use std::time::Duration;

use crate::{
  action_button::{ActionButton, ActionLabel},
  arcade_exit_sequence::ArcadeExitStage,
  portrait_viewport::PortraitViewport,
  review_button::ReviewButton,
  screen_header::{HeaderVariant, ScreenHeader},
};
use battlement::{Align, Color, FlexDirection, Gradient, Position, Style};
use battlement_reactant::{hooks, paint::PaintStyle, prelude::*};
use trox::{ls, tx};

/// Exercises intact, timed collapse, black, reduced-motion, and reset states.
#[builder]
pub struct ExitSequenceHarness;

impl Component for ExitSequenceHarness {
  fn render(&self) -> impl Render {
    let (active, set_active) = hooks::use_state(false);
    let (reduce_motion, set_reduce_motion) = hooks::use_state(false);
    let (reset_generation, set_reset_generation) = hooks::use_state(0_u32);
    let clock = use_controlled_motion_clock();
    View::new()
      .name("exit-sequence-harness")
      .style(Style::new().flex_grow(1).min_height(0).margin_top(24))
      .child((
        View::new().style(Style::new().height(196)).child((
          Flex::new()
            .direction(FlexDirection::Row)
            .gap(12.0)
            .style(Style::new().height(98).align_items(Align::Center))
            .child((
              ReviewButton::new()
                .label(tx("EXIT", "Start the arcade exit sequence."))
                .name("exit-sequence-start")
                .disabled(active)
                .on_press(EventCallback::new({
                  let clock = clock.clone();
                  let set_active = set_active.clone();
                  move |()| {
                    clock.set(Duration::ZERO);
                    set_active.set(true);
                  }
                })),
              ReviewButton::new()
                .label(tx("COLLAPSE", "Show the source collapse checkpoint."))
                .name("exit-sequence-midpoint")
                .disabled(!active)
                .on_press(EventCallback::new({
                  let clock = clock.clone();
                  move |()| clock.set(Duration::from_millis(453))
                })),
              ReviewButton::new()
                .label(tx("BLACK", "Complete the arcade exit sequence."))
                .name("exit-sequence-complete")
                .disabled(!active)
                .on_press(EventCallback::new({
                  let clock = clock.clone();
                  move |()| clock.set(Duration::from_millis(620))
                })),
            )),
          Flex::new()
            .direction(FlexDirection::Row)
            .gap(12.0)
            .style(Style::new().height(98).align_items(Align::Center))
            .child((
              ReviewButton::new()
                .label(ls(if reduce_motion { "REDUCED" } else { "FULL" }))
                .name("exit-sequence-motion-policy")
                .disabled(active)
                .on_press(set_reduce_motion.update_callback(|value| !value)),
              ReviewButton::new()
                .label(tx("RESET", "Reset the arcade exit specimen."))
                .name("exit-sequence-reset")
                .on_press(EventCallback::new({
                  let clock = clock.clone();
                  move |()| {
                    clock.set(Duration::ZERO);
                    set_active.set(false);
                    set_reduce_motion.set(false);
                    set_reset_generation.update(|value| value.wrapping_add(1));
                  }
                })),
            )),
        )),
        View::new()
          .style(Style::new().flex_grow(1).min_height(0))
          .child(
            PortraitViewport::new().child(
              MotionConfig::new(
                ArcadeExitStage::new()
                  .active(active)
                  .reduce_motion(reduce_motion)
                  .children(ExitSpecimen::new())
                  .key(reset_generation),
              )
              .time_source(MotionTimeSource::Controlled(clock)),
            ),
          ),
      ))
  }
}

#[builder]
struct ExitSpecimen;

impl Component for ExitSpecimen {
  fn render(&self) -> impl Render {
    Region::new(ls("Arcade exit content"))
      .host_name("arcade-exit-content")
      .style(Style::new().position(Position::Absolute).inset(0))
      .child(
        View::new()
          .style(Style::new().absolute_fill().center_content())
          .paint(
            PaintStyle::new().background(
              Gradient::radial([0.5, 0.42], [0.82, 0.68])
                .stop(0.0, Color::hex(0x123b86))
                .stop(0.42, Color::hex(0x06152c))
                .stop(1.0, Color::BLACK),
            ),
          )
          .child((
            ScreenHeader::new().variant(HeaderVariant::Game),
            View::new()
              .style(
                Style::new()
                  .position(Position::Absolute)
                  .top(520)
                  .left(103)
                  .width(760)
                  .height(140),
              )
              .child(
                ActionButton::new()
                  .children(Text::new(ls("PLAY")))
                  .artwork(ActionLabel::Play),
              ),
          )),
      )
  }
}
