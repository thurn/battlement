//! Deterministic gallery harness for the complete main menu.

use battlement::{Align, FlexDirection, Style};
use battlement_reactant::{control_behavior, hooks, prelude::*};
use trox::{ls, tx};

use crate::{
  main_menu::MainMenu, portrait_viewport::PortraitViewport, review_button::ReviewButton,
};

/// Exercises navigation, sound, exit completion, reduced motion, and reset.
#[builder]
pub struct MainMenuHarness;

impl Component for MainMenuHarness {
  fn render(&self) -> impl Render {
    MainMenuHarnessContent::new()
  }
}

#[builder]
struct MainMenuHarnessContent;

impl Component for MainMenuHarnessContent {
  fn render(&self) -> impl Render {
    let (generation, set_generation) = hooks::use_state(0_u32);
    let (settings_requests, set_settings_requests) = hooks::use_state(0_u32);
    let (reduce_motion, set_reduce_motion) = hooks::use_state(false);
    let music = crate::background_music::use_background_music();
    View::new()
      .name("main-menu-harness")
      .style(Style::new().flex_grow(1).min_height(0).margin_top(24))
      .child((
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(12.0)
          .style(Style::new().height(86).align_items(Align::Center))
          .child((
            ReviewButton::new()
              .label(ls(if reduce_motion { "REDUCED" } else { "FULL" }))
              .name("main-menu-motion-policy")
              .on_press(set_reduce_motion.update_callback(|value| !value)),
            ReviewButton::new()
              .label(tx("RESET", "Reset the complete main menu."))
              .name("main-menu-reset")
              .on_press(
                EventCallback::new({
                  let music = music.clone();
                  move |()| music.set_sound_muted(false)
                })
                .then(set_generation.update_callback(|value| value.wrapping_add(1)))
                .then(set_settings_requests.callback().map_input(|_| 0))
                .then(set_reduce_motion.callback().map_input(|_| false)),
              ),
            control_behavior::static_label(ls(format!("Settings requests: {settings_requests}")))
              .name("main-menu-status")
              .style(Style::new().height(54).font_size(24)),
          )),
        View::new()
          .style(Style::new().flex_grow(1).min_height(0))
          .child(
            PortraitViewport::new().child(
              MainMenu::new()
                .reduce_motion(reduce_motion)
                .on_settings(set_settings_requests.update_callback(|value| value + 1))
                .key(generation),
            ),
          ),
      ))
  }
}
