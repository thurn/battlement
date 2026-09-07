//! Controlled review surface for shared background music.

use battlement::{Align, Color, FlexDirection, FontStyle, Style};
use battlement_reactant::{application, control_behavior, hooks, prelude::*};
use trox::{ls, tx};

use crate::{
  background_music::{
    self, BackgroundMusicContext, BackgroundMusicProvider, BackgroundMusicStatus,
    PlaybackAvailability,
  },
  review_button::ReviewButton,
  toggle_control::ToggleControl,
  volume_control::VolumeControl,
};

/// Exercises native playback, visibility muting, availability, and reset.
#[builder]
pub struct BackgroundMusicHarness;

impl Component for BackgroundMusicHarness {
  fn render(&self) -> impl Render {
    let (hidden, set_hidden) = hooks::use_state(false);
    let (availability, set_availability) = hooks::use_state(PlaybackAvailability::Available);
    background_music::availability_provider(
      availability,
      application::provider(battlement::application::ApplicationState {
        focused: true,
        paused: hidden,
      })
      .child(
        BackgroundMusicProvider::new().children(
          BackgroundMusicSpecimen::new()
            .hidden(hidden)
            .availability(availability)
            .set_hidden(set_hidden)
            .set_availability(set_availability),
        ),
      ),
    )
  }
}

#[builder]
struct BackgroundMusicSpecimen {
  #[builder(required)]
  hidden: bool,
  #[builder(required)]
  availability: PlaybackAvailability,
  #[builder(required)]
  set_hidden: StateSetter<bool>,
  #[builder(required)]
  set_availability: StateSetter<PlaybackAvailability>,
}

impl Component for BackgroundMusicSpecimen {
  fn render(&self) -> impl Render {
    let music = background_music::use_background_music();
    View::new()
      .name("background-music-harness")
      .style(Style::new().width(896).margin_top(24))
      .child((
        self::actions(self, &music),
        self::status(&music),
        View::new()
          .name("background-music-controls")
          .style(
            Style::new()
              .width(839)
              .margin_top(20)
              .background_color(Color::rgb(0.01, 0.035, 0.08)),
          )
          .child((
            VolumeControl::new()
              .label(tx("Master Volume", "Volume control interface label."))
              .value(music.master_volume)
              .on_change({
                let music = music.clone();
                move |value| music.set_master_volume(value)
              })
              .first(true),
            VolumeControl::new()
              .label(tx("Music Volume", "Volume control interface label."))
              .value(music.music_volume)
              .on_change({
                let music = music.clone();
                move |value| music.set_music_volume(value)
              }),
            ToggleControl::new()
              .label(control_behavior::name_source_text(tx(
                "Mute in Background",
                "Background music setting label.",
              )))
              .checked(music.mute_in_background)
              .on_change({
                let music = music.clone();
                move |muted| music.set_mute_in_background(muted)
              }),
          )),
      ))
  }
}

fn actions(specimen: &BackgroundMusicSpecimen, music: &BackgroundMusicContext) -> Flex {
  Flex::new()
    .direction(FlexDirection::Column)
    .gap(8.0)
    .style(Style::new().align_items(Align::FlexStart))
    .child((
      Flex::new().direction(FlexDirection::Row).gap(12.0).child((
        ReviewButton::new()
          .label(ls("START"))
          .name("background-music-start")
          .on_press({
            let music = music.clone();
            move || music.start_music()
          }),
        ReviewButton::new()
          .label(ls("RESET"))
          .name("background-music-reset")
          .on_press({
            let music = music.clone();
            let set_hidden = specimen.set_hidden.clone();
            let set_availability = specimen.set_availability.clone();
            move || {
              music.reset();
              set_hidden.set(false);
              set_availability.set(PlaybackAvailability::Available);
            }
          }),
      )),
      Flex::new().direction(FlexDirection::Row).gap(12.0).child((
        ReviewButton::new()
          .label(ls(if specimen.hidden { "HIDDEN" } else { "VISIBLE" }))
          .name("background-music-visibility")
          .on_press(specimen.set_hidden.update_callback(|hidden| !hidden)),
        ReviewButton::new()
          .label(ls(match specimen.availability {
            PlaybackAvailability::Available => "AVAILABLE",
            PlaybackAvailability::Unavailable => "UNAVAILABLE",
          }))
          .name("background-music-availability")
          .on_press(
            specimen
              .set_availability
              .update_callback(|availability| match availability {
                PlaybackAvailability::Available => PlaybackAvailability::Unavailable,
                PlaybackAvailability::Unavailable => PlaybackAvailability::Available,
              }),
          ),
      )),
    ))
}

fn status(music: &BackgroundMusicContext) -> View {
  let playback = match music.status {
    BackgroundMusicStatus::Stopped => "stopped",
    BackgroundMusicStatus::Playing => "playing",
    BackgroundMusicStatus::Unavailable => "unavailable",
  };
  let output = if music.muted { "muted" } else { "audible" };
  View::new()
    .name("background-music-status")
    .style(
      Style::new()
        .margin_top(16)
        .padding(18)
        .border_width(1)
        .border_color(Color::rgb8(41, 75, 96))
        .border_radius(8)
        .background_color(Color::rgb8(7, 20, 36)),
    )
    .child(
      Flex::new()
        .direction(FlexDirection::Column)
        .gap(5.0)
        .child((
          self::status_label(
            "background-music-playback-status",
            format!("Playback: {playback}"),
          ),
          self::status_label(
            "background-music-playhead-status",
            format!("Playhead: {:.1} s", music.playhead.as_secs_f64()),
          ),
          self::status_label(
            "background-music-visibility-status",
            format!(
              "Visibility: {}",
              if music.visible { "visible" } else { "hidden" }
            ),
          ),
          self::status_label(
            "background-music-volume-status",
            format!("Effective: {:.0}%", music.effective_volume * 100.0),
          ),
          self::status_label("background-music-mute-status", format!("Output: {output}")),
        )),
    )
}

fn status_label(name: &'static str, value: String) -> Label {
  control_behavior::static_label(ls(value)).name(name).style(
    Style::new()
      .font_size(24)
      .unity_font_style_and_weight(FontStyle::Bold)
      .color(Color::rgb8(182, 221, 236)),
  )
}
