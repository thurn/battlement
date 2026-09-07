//! Deterministic full-screen review harness for SettingsScreen.

use battlement::{Align, FlexDirection, Style};
use battlement_reactant::{control_behavior, hooks, portal::PortalTarget, prelude::*};
use trox::{ls, tx};

use crate::{
  arcade_frame_pulse::{ArcadeFramePulse, ArcadeScreen},
  background_music::BackgroundMusicProvider,
  portrait_viewport::PortraitViewport,
  review_button::ReviewButton,
  screen_frame::ScreenFrame,
  settings_screen::SettingsScreen,
};

/// Shows the complete screen and exposes its host-owned outcomes.
#[builder]
pub struct SettingsScreenHarness {
  #[builder(required)]
  overlay: PortalTarget,
}

impl Component for SettingsScreenHarness {
  fn render(&self) -> impl Render {
    let (generation, set_generation) = hooks::use_state(0_u32);
    let (return_requests, set_return_requests) = hooks::use_state(0_u32);
    let (privacy_requests, set_privacy_requests) = hooks::use_state(0_u32);
    let clock = use_controlled_motion_clock();
    View::new()
      .name("settings-screen-harness")
      .style(Style::new().flex_grow(1).min_height(0).margin_top(24))
      .child((
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(12.0)
          .style(Style::new().height(86).align_items(Align::Center))
          .child((
            ReviewButton::new()
              .label(tx("RESET", "Reset the complete settings screen."))
              .name("settings-screen-reset")
              .on_press(
                set_generation
                  .update_callback(|value| value.wrapping_add(1))
                  .then(set_return_requests.callback().map_input(|_| 0))
                  .then(set_privacy_requests.callback().map_input(|_| 0)),
              ),
            control_behavior::static_label(ls(format!(
              "Return requests: {return_requests} · Privacy requests: {privacy_requests}"
            )))
            .name("settings-screen-status")
            .style(Style::new().height(54).font_size(24)),
          )),
        View::new()
          .style(Style::new().flex_grow(1).min_height(0))
          .child(
            PortraitViewport::new().child(
              BackgroundMusicProvider::new().children(
                ScreenFrame::new().children((
                  SettingsScreen::new()
                    .overlay(self.overlay.clone())
                    .on_return(set_return_requests.update_callback(|value| value + 1))
                    .on_open_url(
                      set_privacy_requests
                        .update_callback(|value| value + 1)
                        .map_input(|_: String| ()),
                    )
                    .key(generation),
                  MotionConfig::new(
                    ArcadeFramePulse::new()
                      .active_screen(ArcadeScreen::Settings)
                      .reduce_motion(false),
                  )
                  .time_source(MotionTimeSource::Controlled(clock)),
                )),
              ),
            ),
          ),
      ))
  }
}
