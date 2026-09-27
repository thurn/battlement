//! App-level audio, vibration, debug, and diagnostics effects.

use std::time::Duration;

use battlement::{
  AudioBus, AudioClipAddress, Command, CommandBody, ControllerVibrationPayload, DebugUiPayload,
};
use battlement_cloud::diagnostics::{DiagnosticsCommand, DiagnosticsMetadata};
use reactant::{
  app_context,
  audio::{self, AudioSettings, AudioTrack},
  hooks,
  prelude::{AudioPlayback, AudioPlaybackOptions, Component, Render},
};

use crate::{
  assets::music,
  chess_ui_state::{AppScreen, ChessUiController, LocalEffect},
};

const MUSIC_CROSSFADE: Duration = Duration::from_secs(5);
pub(super) const MUSIC_TRACKS: [AudioClipAddress; 4] = [
  music::CRITICAL,
  music::SWITCH_WITH_ME,
  music::BREAKBEAT_CHIPS,
  music::DRAG_AND_DREAD,
];

/// Component that interprets app-local effect state at the host boundary.
///
/// Keeping effects in a leaf component separates idempotent rendering from
/// imperative audio, vibration, debug UI, and diagnostics commands.
pub struct GameEffects {
  control: ChessUiController,
  diagnostics: bool,
}

/// Publishes game metadata only when the optional diagnostics host module exists.
///
/// This helper returns renderable effect work, letting callers colocate metadata
/// with the component that derives it while Reactant schedules the host command.
pub fn diagnostics_view(status: &'static str, origin: &'static str) -> impl Render {
  let app = app_context::use_app();
  hooks::use_effect(
    move || {
      for (key, value) in [("chess.game_status", status), ("chess.game_origin", origin)] {
        app.send(Command::diagnostics(DiagnosticsCommand::SetMetadata(
          DiagnosticsMetadata {
            key: key.to_owned(),
            value: Some(value.to_owned()),
          },
        )));
      }
    },
    (status, origin),
  );
}

pub(super) fn music_track_count() -> usize {
  MUSIC_TRACKS.len()
}

impl GameEffects {
  /// Creates the effect bridge for one app-local controller.
  pub fn new(control: ChessUiController, diagnostics: bool) -> Self {
    Self {
      control,
      diagnostics,
    }
  }
}

impl Component for GameEffects {
  /// Declares effects keyed by the smallest state that should retrigger them.
  ///
  /// Music depends on generation, track, and screen, while one-shot feedback
  /// depends on a monotonically increasing serial. Explicit dependencies prevent
  /// ordinary rerenders from replaying host commands.
  fn render(&self) -> impl Render {
    let local = self.control.snapshot();
    let app = app_context::use_app();
    audio::use_audio(
      local.music_generation,
      Some(AudioTrack {
        address: MUSIC_TRACKS[local.music_track].clone(),
        bus: AudioBus::Music,
        looping: true,
      }),
      AudioSettings {
        enabled: local.music_generation != 0 && local.screen != AppScreen::Menu,
        crossfade: MUSIC_CROSSFADE,
        pause_in_background: false,
        ..AudioSettings::default()
      },
    );
    hooks::use_effect(
      {
        let app = app.clone();
        move || {
          if let Some(effect) = local.effect.clone() {
            match effect {
              LocalEffect::Sound(address) => {
                let (_, command) = AudioPlayback::play(address, AudioPlaybackOptions::new());
                app.send(command);
              }
              LocalEffect::Invalid => {
                let (_, command) = AudioPlayback::play(
                  crate::audio::INVALID_DROP_SOUND,
                  AudioPlaybackOptions::new(),
                );
                app.send(command);
                app.send(Command::new_v4(CommandBody::ControllerVibrate(
                  ControllerVibrationPayload {
                    low_frequency: 0.2,
                    high_frequency: 0.25,
                    duration_ms: 90,
                  },
                )));
              }
              LocalEffect::ShowDebug(surface) => {
                app.send(Command::new_v4(CommandBody::DebugUi(DebugUiPayload {
                  surface,
                  visible: true,
                })));
              }
            }
          }
        }
      },
      local.effect_serial,
    );
    if self.diagnostics {
      hooks::use_effect(
        {
          let app = app.clone();
          move || {
            for (key, value) in [
              ("sample.name", "chess"),
              ("sample.rules_version", env!("CARGO_PKG_VERSION")),
              ("chess.opponent", "computer"),
            ] {
              app.send(Command::diagnostics(DiagnosticsCommand::SetMetadata(
                DiagnosticsMetadata {
                  key: key.to_owned(),
                  value: Some(value.to_owned()),
                },
              )));
            }
          }
        },
        (),
      );
    }
  }
}
