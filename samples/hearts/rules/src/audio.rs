use std::time::Duration;

use battlement::{AudioBus, AudioClipAddress, AudioMix};
use reactant::{
  audio::{self, AudioSettings, AudioTrack},
  hooks,
  prelude::*,
};
use trox::ls;

use crate::{
  assets,
  domain::{Event, Phase},
  settings::Preferences,
};

pub(crate) struct GameAudio {
  pub phase: Phase,
  pub selection: u64,
}

pub(crate) struct SoundControls {
  pub mix: AudioMix,
  pub set_mix: hooks::StateSetter<AudioMix>,
}

impl Component for GameAudio {
  fn render(&self) -> impl Render {
    let result = matches!(self.phase, Phase::HandOver | Phase::MatchOver { .. });
    audio::use_audio(
      (),
      Some(AudioTrack {
        address: if result {
          assets::hearts::audio::RESULTS_MUSIC
        } else {
          assets::hearts::audio::MUSIC
        },
        bus: AudioBus::Music,
        looping: true,
      }),
      AudioSettings {
        crossfade: Duration::from_millis(900),
        ..AudioSettings::default()
      },
    );
    audio::use_audio(
      self.selection,
      (self.selection != 0).then_some(AudioTrack {
        address: assets::hearts::audio::SELECT,
        bus: AudioBus::Effects,
        looping: false,
      }),
      AudioSettings {
        gain: 0.45,
        ..AudioSettings::default()
      },
    );
  }
}

/// Existing NotJam cues substitute for literal card and woodland recordings.
pub(crate) fn cue(event: Event) -> Option<(AudioClipAddress, f64)> {
  let (address, gain) = match event {
    Event::Deal => (assets::hearts::audio::DEAL, 0.4),
    Event::PassSubmitted { .. } => return None,
    Event::PassExchanged => (assets::hearts::audio::PASS, 0.25),
    Event::CardPlayed { .. } => (assets::hearts::audio::LAND, 0.35),
    Event::TrickCollected { .. } => (assets::hearts::audio::COLLECT, 0.45),
    Event::HandScored { .. } => (assets::hearts::audio::RESULT, 0.35),
    Event::MatchEnded { .. } => return None,
  };
  Some((address, gain))
}

impl Component for SoundControls {
  fn render(&self) -> impl Render {
    let font = if hooks::use_context::<Preferences>().larger_text {
      24.0
    } else {
      18.0
    };
    let music = self.mix.music > 0.0;
    let effects = self.mix.effects > 0.0;
    View::new()
      .style(
        Style::new()
          .position(Position::Absolute)
          .right(18.px())
          .top(70.px())
          .flex_direction(FlexDirection::Row),
      )
      .child((
        Button::new(ls(if music { "Music on" } else { "Music off" }))
          .style(self::button_style(font))
          .on_press(self.set_mix.update_callback(|mix| AudioMix {
            music: if mix.music > 0.0 { 0.0 } else { 0.22 },
            ..mix
          })),
        Button::new(ls(if effects { "Effects on" } else { "Effects off" }))
          .style(self::button_style(font))
          .on_press(self.set_mix.update_callback(|mix| AudioMix {
            effects: if mix.effects > 0.0 { 0.0 } else { 0.65 },
            ..mix
          })),
      ))
  }
}

fn button_style(font: f32) -> Style {
  Style::new()
    .width(104.px())
    .margin_left(6.px())
    .height(40.px())
    .font_size(font.px())
    .color(Color::rgb(0.08, 0.14, 0.06))
    .background_color(Color::rgb(0.96, 0.92, 0.77))
    .border_radius(6.px())
}
