//! Isolated, resettable attract-mode review surface.

use std::time::Duration;

use crate::{
  arcade_attract_mode::ArcadeAttractMode, portrait_viewport::PortraitViewport,
  review_button::ReviewButton, screen_frame::ScreenFrame,
};
use battlement::{Color, FlexDirection, Style};
use battlement_reactant::{control_behavior, hooks, prelude::*};
use trox::ls;

/// Exercises seeded motion, reset, and the reduced-motion alternative.
#[builder]
pub struct AttractModeHarness;

impl Component for AttractModeHarness {
  fn render(&self) -> impl Render {
    let (reduced_motion, set_reduced_motion) = hooks::use_state(false);
    let clock = use_controlled_motion_clock();
    View::new()
      .name("arcade-attract-harness")
      .style(Style::new().flex_grow(1).min_height(0).margin_top(32))
      .child(
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(12.0)
          .style(Style::new().height(86).padding_left(8))
          .child(
            ReviewButton::new()
              .label(ls(if reduced_motion {
                "REDUCED"
              } else {
                "FULL MOTION"
              }))
              .name("attract-motion-policy")
              .on_press(set_reduced_motion.update_callback(|value| !value)),
          )
          .child(
            ReviewButton::new()
              .label(ls("ADVANCE"))
              .name("attract-advance")
              .on_press(EventCallback::new({
                let clock = clock.clone();
                move |()| clock.advance(Duration::from_millis(1_500))
              })),
          )
          .child(
            ReviewButton::new()
              .label(ls("RESET"))
              .name("attract-reset")
              .on_press(EventCallback::new({
                let clock = clock.clone();
                move |()| clock.set(Duration::ZERO)
              })),
          )
          .child(
            control_behavior::static_label(ls(if reduced_motion { "REDUCED" } else { "FULL" }))
              .name("attract-status")
              .style(Style::new().font_size(28).color(Color::hex(0xbddcf7))),
          ),
      )
      .child(
        View::new()
          .style(Style::new().flex_grow(1).min_height(0))
          .child(
            PortraitViewport::new().child(
              ScreenFrame::new().children(
                MotionConfig::new(ArcadeAttractMode::new().reduce_motion(reduced_motion))
                  .time_source(MotionTimeSource::Controlled(clock)),
              ),
            ),
          ),
      )
  }
}
