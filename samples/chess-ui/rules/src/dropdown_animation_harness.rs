//! Resettable review surface for selector motion and interruption.

use trox::{ls, tx};

use crate::{review_button::ReviewButton, select_control::SelectControl};
use battlement::{Color, FlexDirection, Style};
use battlement_reactant::{
  control_behavior, hooks,
  motion_config::{MotionConfig, ReducedMotion},
  portal::PortalTarget,
  prelude::*,
};

/// Exercises dropdown presence with an explicit reduced-motion policy.
#[builder]
pub struct DropdownAnimationHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for DropdownAnimationHarness {
  fn render(&self) -> impl Render {
    let (value, set_value) = hooks::use_state(String::from("Borderless"));
    let (reduced_motion, set_reduced_motion) = hooks::use_state(false);
    let (reset_generation, set_reset_generation) = hooks::use_state(0_u32);
    MotionConfig::new(
      View::new()
        .name("dropdown-animation-specimen")
        .style(
          Style::new()
            .width(839)
            .height(650)
            .margin_top(48)
            .background_color(Color::rgb(0.01, 0.035, 0.08)),
        )
        .child(
          Flex::new()
            .direction(FlexDirection::Row)
            .gap(16.0)
            .style(Style::new().height(110).padding_left(20))
            .child(
              ReviewButton::new()
                .label(ls(if reduced_motion {
                  "REDUCED MOTION"
                } else {
                  "FULL MOTION"
                }))
                .name("dropdown-motion-policy")
                .on_press(set_reduced_motion.update_callback(|value| !value)),
            )
            .child(
              ReviewButton::new()
                .label(ls("RESET"))
                .name("dropdown-animation-reset")
                .on_press(EventCallback::new({
                  let set_value = set_value.clone();
                  move |()| {
                    set_value.set(String::from("Borderless"));
                    set_reduced_motion.set(false);
                    set_reset_generation.update(|generation| generation.wrapping_add(1));
                  }
                })),
            ),
        )
        .child(
          View::new()
            .key(reset_generation)
            .style(Style::new().width(839).height(480).margin_top(24))
            .child(
              SelectControl::new()
                .label(control_behavior::name_source_text(tx(
                  "Display Mode",
                  "Display mode selector interface label.",
                )))
                .value(value)
                .options(vec![
                  String::from("Borderless"),
                  String::from("Fullscreen"),
                  String::from("Windowed"),
                ])
                .overlay(self.overlay.clone())
                .on_change(set_value)
                .first(true),
            ),
        ),
    )
    .reduced_motion(if reduced_motion {
      ReducedMotion::Always
    } else {
      ReducedMotion::Never
    })
  }
}
