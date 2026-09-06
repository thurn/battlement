//! Controlled review surface for arcade-frame pulse contexts.

use std::time::Duration;

use crate::{
  arcade_frame_pulse::{ArcadeFramePulse, ArcadeScreen},
  portrait_viewport::PortraitViewport,
  return_button::ReturnButton,
  review_button::ReviewButton,
  screen_frame::ScreenFrame,
};
use battlement::{FlexDirection, Style};
use battlement_reactant::{hooks, prelude::*};
use trox::ls;

/// Switches frame context, logical time, and reduced-motion policy.
#[builder]
pub struct FramePulseHarness;

impl Component for FramePulseHarness {
  fn render(&self) -> impl Render {
    let (active_screen, set_active_screen) = hooks::use_state(ArcadeScreen::Main);
    let (reduced_motion, set_reduced_motion) = hooks::use_state(false);
    let (preview_open, set_preview_open) = hooks::use_state(false);
    let clock = use_controlled_motion_clock();
    View::new()
      .name("frame-pulse-harness")
      .style(Style::new().flex_grow(1).min_height(0).margin_top(32))
      .child((!preview_open).then(|| {
        Button::content(())
          .semantic_name(SemanticName::Text(ls("Show pulse preview")))
          .host_name("frame-pulse-preview")
          .style(Style::new().absolute_fill().opacity(0))
          .on_press(move || set_preview_open.set(true))
      }))
      .child(preview_open.then(|| {
        Flex::new()
          .direction(FlexDirection::Row)
          .gap(12.0)
          .style(Style::new().height(86).padding_left(8))
          .child(
            ReviewButton::new()
              .label(ls(if active_screen == ArcadeScreen::Main {
                "MAIN"
              } else {
                "SETTINGS"
              }))
              .name("frame-pulse-context")
              .on_press(set_active_screen.update_callback(|screen| match screen {
                ArcadeScreen::Main => ArcadeScreen::Settings,
                ArcadeScreen::Settings => ArcadeScreen::Main,
              })),
          )
          .child(
            ReviewButton::new()
              .label(ls("ADVANCE"))
              .name("frame-pulse-advance")
              .on_press(EventCallback::new({
                let clock = clock.clone();
                move |()| clock.advance(Duration::from_millis(4_030))
              })),
          )
          .child(
            ReviewButton::new()
              .label(ls(if reduced_motion { "REDUCED" } else { "FULL" }))
              .name("frame-pulse-motion-policy")
              .on_press(set_reduced_motion.update_callback(|value| !value)),
          )
          .child(
            ReviewButton::new()
              .label(ls("RESET"))
              .name("frame-pulse-reset")
              .on_press(EventCallback::new({
                let clock = clock.clone();
                move |()| {
                  clock.set(Duration::ZERO);
                  set_active_screen.set(ArcadeScreen::Main);
                  set_reduced_motion.set(false);
                }
              })),
          )
      }))
      .child(preview_open.then(|| {
        View::new()
          .style(Style::new().flex_grow(1).min_height(0))
          .child(
            PortraitViewport::new().child(
              ScreenFrame::new().children(
                MotionConfig::new((
                  ArcadeFramePulse::new()
                    .active_screen(active_screen)
                    .reduce_motion(reduced_motion),
                  (active_screen == ArcadeScreen::Settings)
                    .then(|| ReturnButton::new().on_press(|| {})),
                ))
                .time_source(MotionTimeSource::Controlled(clock)),
              ),
            ),
          )
      }))
  }
}
