//! Controlled review surface for the audio-ledger heartbeat and indicator.

use std::time::Duration;

use crate::{
  action_button::{ActionButton, ActionLabel},
  background_music::{self, BackgroundMusicContext, BackgroundMusicProvider, PlaybackAvailability},
  music_playback_indicator::MusicPlaybackIndicator,
  portrait_viewport::PortraitViewport,
  review_button::ReviewButton,
  screen_frame::ScreenFrame,
};
use battlement::{Color, FlexDirection, Position, Style};
use battlement_reactant::{control_behavior, hooks, prelude::*};
use trox::ls;

/// Exercises sound toggling, zero-volume recovery, availability, and motion policy.
#[builder]
pub struct MusicHeartbeatHarness;

impl Component for MusicHeartbeatHarness {
  fn render(&self) -> impl Render {
    let (reduced_motion, set_reduced_motion) = hooks::use_state(false);
    let (availability, set_availability) = hooks::use_state(PlaybackAvailability::Available);
    background_music::availability_provider(
      availability,
      BackgroundMusicProvider::new().children(
        MusicHeartbeatSpecimen::new()
          .reduced_motion(reduced_motion)
          .availability(availability)
          .set_reduced_motion(set_reduced_motion)
          .set_availability(set_availability),
      ),
    )
  }
}

#[builder]
struct MusicHeartbeatSpecimen {
  #[builder(required)]
  reduced_motion: bool,
  #[builder(required)]
  availability: PlaybackAvailability,
  #[builder(required)]
  set_reduced_motion: StateSetter<bool>,
  #[builder(required)]
  set_availability: StateSetter<PlaybackAvailability>,
}

impl Component for MusicHeartbeatSpecimen {
  fn render(&self) -> impl Render {
    let music = background_music::use_background_music();
    View::new()
      .name("music-heartbeat-harness")
      .style(Style::new().flex_grow(1).min_height(0).margin_top(20))
      .child((
        self::actions(self, &music),
        View::new()
          .style(Style::new().flex_grow(1).min_height(0).margin_top(12))
          .child(
            PortraitViewport::new()
              .child(ScreenFrame::new().children(self::source_crop(self.reduced_motion))),
          ),
      ))
  }
}

fn actions(specimen: &MusicHeartbeatSpecimen, music: &BackgroundMusicContext) -> Flex {
  Flex::new()
    .direction(FlexDirection::Column)
    .gap(8.0)
    .child((
      Flex::new().direction(FlexDirection::Row).gap(10.0).child((
        ReviewButton::new()
          .label(ls("PULSE"))
          .name("music-heartbeat-pulse")
          .on_press({
            let music = music.clone();
            move || music.seek_for_review(Duration::from_secs_f64(1.04))
          }),
        ReviewButton::new()
          .label(ls("ZERO VOLUME"))
          .name("music-heartbeat-zero-volume")
          .on_press({
            let music = music.clone();
            move || {
              music.set_master_volume(0);
              music.set_music_volume(0);
            }
          }),
        ReviewButton::new()
          .label(ls(if specimen.reduced_motion {
            "REDUCED"
          } else {
            "FULL"
          }))
          .name("music-heartbeat-motion-policy")
          .on_press(specimen.set_reduced_motion.update_callback(|value| !value)),
      )),
      Flex::new().direction(FlexDirection::Row).gap(10.0).child((
        ReviewButton::new()
          .label(ls(match specimen.availability {
            PlaybackAvailability::Available => "AVAILABLE",
            PlaybackAvailability::Unavailable => "UNAVAILABLE",
          }))
          .name("music-heartbeat-availability")
          .on_press(
            specimen
              .set_availability
              .update_callback(|availability| match availability {
                PlaybackAvailability::Available => PlaybackAvailability::Unavailable,
                PlaybackAvailability::Unavailable => PlaybackAvailability::Available,
              }),
          ),
        ReviewButton::new()
          .label(ls("RESET"))
          .name("music-heartbeat-reset")
          .on_press({
            let music = music.clone();
            let set_reduced_motion = specimen.set_reduced_motion.clone();
            let set_availability = specimen.set_availability.clone();
            move || {
              music.reset();
              set_reduced_motion.set(false);
              set_availability.set(PlaybackAvailability::Available);
            }
          }),
      )),
    ))
}

fn source_crop(reduced_motion: bool) -> View {
  View::new()
    .name("music-heartbeat-source-crop")
    .style(
      Style::new()
        .position(Position::Relative)
        .width(982)
        .height(1_404)
        .background_color(Color::rgb8(2, 6, 19)),
    )
    .child((
      View::new()
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(121)
            .top(360)
            .width(740)
            .height(160),
        )
        .child(
          crate::music_heartbeat::MusicHeartbeat::new()
            .reduced_motion(reduced_motion)
            .children(
              ActionButton::new()
                .artwork(ActionLabel::Play)
                .children(control_behavior::name_source_text(ls("PLAY")))
                .on_press(|| {}),
            ),
        ),
      View::new()
        .style(
          Style::new()
            .position(Position::Absolute)
            .left(80)
            .right(80)
            .bottom(218)
            .height(114),
        )
        .child(MusicPlaybackIndicator::new().reduced_motion(reduced_motion)),
    ))
}
