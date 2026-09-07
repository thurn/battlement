//! Deterministic gallery harness for the complete main menu.

use battlement::{Align, FlexDirection, Style};
use battlement_reactant::{control_behavior, hooks, prelude::*};
use trox::{ls, tx};

use crate::{
  arcade_frame_pulse::ArcadeScreen,
  arcade_route_transition::{ArcadeRouteTransition, use_arcade_navigation},
  background_music,
  font_scale::FontScaleProvider,
  main_menu::MainMenu,
  portrait_viewport::PortraitViewport,
  review_button::ReviewButton,
  screen_frame::ExitAwareScreenFrame,
};

/// Exercises navigation, sound, exit completion, reduced motion, and reset.
#[builder]
pub struct MainMenuHarness;

impl Component for MainMenuHarness {
  fn render(&self) -> impl Render {
    let (generation, set_generation) = hooks::use_state(0_u32);
    FontScaleProvider::new().children(
      ArcadeRouteTransition::new()
        .children(MainMenuHarnessContent::new().reset_generation(set_generation))
        .key(generation),
    )
  }
}

#[builder]
struct MainMenuHarnessContent {
  #[builder(required)]
  reset_generation: StateSetter<u32>,
}

impl Component for MainMenuHarnessContent {
  fn render(&self) -> impl Render {
    let navigation = use_arcade_navigation();
    let music = background_music::use_background_music();
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
              .label(ls(if navigation.reduce_motion {
                "REDUCED"
              } else {
                "FULL"
              }))
              .name("main-menu-motion-policy")
              .on_press(EventCallback::new({
                let navigation = navigation.clone();
                move |()| navigation.set_reduce_motion(!navigation.reduce_motion)
              })),
            ReviewButton::new()
              .label(tx("RESET", "Reset the complete main menu."))
              .name("main-menu-reset")
              .on_press(EventCallback::new({
                let music = music.clone();
                let reset_generation = self.reset_generation.clone();
                move |()| {
                  music.set_sound_muted(false);
                  reset_generation.update(|value| value.wrapping_add(1));
                }
              })),
            control_behavior::static_label(ls(format!(
              "Settings requests: {}",
              u32::from(navigation.has_navigated)
            )))
            .name("main-menu-status")
            .style(Style::new().height(54).font_size(24)),
          )),
        View::new()
          .style(Style::new().flex_grow(1).min_height(0))
          .child(
            PortraitViewport::new().child(
              ExitAwareScreenFrame::new()
                .frame_pulse(ArcadeScreen::Main)
                .reduce_motion(navigation.reduce_motion)
                .children(MainMenu::new()),
            ),
          ),
      ))
  }
}
