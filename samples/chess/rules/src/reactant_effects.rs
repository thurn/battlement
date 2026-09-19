//! App-level audio, vibration, debug, and diagnostics effects.

use std::time::Duration;

use battlement::{
  Command, CommandBody, ControllerVibrationPayload, DebugUiPayload, ObjectId, object_id,
};
use battlement_cloud::diagnostics::{DiagnosticsCommand, DiagnosticsMetadata};
use reactant::{
  hooks,
  prelude::{AudioPlayback, AudioPlaybackOptions, Component, Render},
};

use crate::{
  MUSIC_TRACKS,
  reactant_input::{AppControl, LocalEffect},
};

const MUSIC_PRIMARY: ObjectId = object_id!("43000000-0000-4000-8500-000000000001");
const MUSIC_SECONDARY: ObjectId = object_id!("43000000-0000-4000-8500-000000000002");
const MUSIC_CROSSFADE: Duration = Duration::from_secs(5);

pub(crate) struct EffectsView {
  control: AppControl,
  diagnostics: bool,
}

impl EffectsView {
  pub(crate) fn new(control: AppControl, diagnostics: bool) -> Self {
    Self {
      control,
      diagnostics,
    }
  }
}

impl Component for EffectsView {
  fn render(&self) -> impl Render {
    let local = hooks::use_external_store(self.control.store());
    let app = reactant::app_context::use_app();
    let previous_music = hooks::use_ref(None::<(u64, usize)>);
    let primary = AudioPlayback::new(MUSIC_PRIMARY);
    let secondary = AudioPlayback::new(MUSIC_SECONDARY);
    hooks::use_effect(
      {
        let app = app.clone();
        let previous_music = previous_music.clone();
        move || {
          let prior = previous_music.get();
          if local.music_generation == 0 {
            if let Some((generation, _)) = prior {
              let active = if generation % 2 == 1 {
                primary
              } else {
                secondary
              };
              app.send(active.stop(Duration::ZERO));
              previous_music.replace(None);
            }
          } else if prior != Some((local.music_generation, local.music_track)) {
            let next = if local.music_generation % 2 == 1 {
              primary
            } else {
              secondary
            };
            let old = if local.music_generation % 2 == 1 {
              secondary
            } else {
              primary
            };
            app.send(
              next.play_command(
                MUSIC_TRACKS[local.music_track].clone(),
                AudioPlaybackOptions::new()
                  .volume(local.volume)
                  .looping(true)
                  .fade_in(if prior.is_some() {
                    MUSIC_CROSSFADE
                  } else {
                    Duration::ZERO
                  }),
              ),
            );
            if prior.is_some() {
              app.send(old.stop(MUSIC_CROSSFADE));
            }
            previous_music.replace(Some((local.music_generation, local.music_track)));
          } else if let Some((generation, _)) = prior {
            let active = if generation % 2 == 1 {
              primary
            } else {
              secondary
            };
            app.send(active.set_volume(local.volume));
          }
        }
      },
      (local.music_generation, local.music_track, local.volume),
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
                let (_, command) =
                  AudioPlayback::play(crate::INVALID_DROP_SOUND, AudioPlaybackOptions::new());
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

pub(crate) fn diagnostics_view(status: &'static str, origin: &'static str) -> impl Render {
  let app = reactant::app_context::use_app();
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
