//! Component-owned playback over the native audio handle.

use std::time::Duration;

use battlement::{AudioBus, AudioClipAddress, ObjectId};

use crate::{
  app_context::{self, AppHandle},
  application, hooks,
  motion_value::{AudioPlayback, AudioPlaybackOptions},
};

/// Clip identity and immutable playback routing.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioTrack {
  /// Prepared clip to play.
  pub address: AudioClipAddress,
  /// Mixer category.
  pub bus: AudioBus,
  /// Repeat the clip until its owner ends.
  pub looping: bool,
}

/// Reactive controls that do not restart an existing clip.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioSettings {
  /// Start or dispose playback.
  pub enabled: bool,
  /// Hold the playhead while retaining the clip.
  pub paused: bool,
  /// Linear gain before shared mixer controls.
  pub gain: f64,
  /// Fade between clip identities; disabling and unmounting stop immediately.
  pub crossfade: Duration,
}

#[derive(Default)]
struct OwnedPlayback {
  current: Option<AudioPlayback>,
  retiring: Option<AudioPlayback>,
  settings: Option<AudioSettings>,
}

impl Default for AudioSettings {
  fn default() -> Self {
    Self {
      enabled: true,
      paused: false,
      gain: 1.0,
      crossfade: Duration::ZERO,
    }
  }
}

/// Owns one clip until replacement, disablement or unmount, pausing in background.
/// Change `occurrence` to play the same clip again; ordinary rerenders and gain
/// changes retain its playhead. Keep occurrence identity stable across settings
/// changes. A disabled occurrence starts anew when re-enabled.
pub fn use_audio<K: hooks::Dependencies>(
  occurrence: K,
  track: Option<AudioTrack>,
  mut settings: AudioSettings,
) -> Option<AudioPlayback> {
  assert!(
    (0.0..=1.0).contains(&settings.gain),
    "audio gain is out of range"
  );
  let app = app_context::use_app();
  settings.paused |= !application::use_application_state().is_active();
  let playback = hooks::use_memo(
    || (settings.enabled && track.is_some()).then(|| AudioPlayback::new(ObjectId::new_v4())),
    (occurrence, track.clone(), settings.enabled),
  );
  let owned = hooks::use_ref(OwnedPlayback::default());
  hooks::use_effect(
    {
      let app = app.clone();
      let owned = owned.clone();
      move || {
        owned.with_mut(|owned| owned.update(&app, playback, track, settings));
      }
    },
    (playback, settings),
  );
  hooks::use_effect(move || move || owned.with_mut(|owned| owned.stop(&app)), ());
  playback
}

impl OwnedPlayback {
  fn update(
    &mut self,
    app: &AppHandle,
    playback: Option<AudioPlayback>,
    track: Option<AudioTrack>,
    settings: AudioSettings,
  ) {
    if self.current != playback {
      if let Some(retiring) = self.retiring.take() {
        app.send(retiring.stop(Duration::ZERO).nonblocking());
      }
      if let Some(current) = self.current.take() {
        let fade = if playback.is_some() && !settings.paused {
          settings.crossfade
        } else {
          Duration::ZERO
        };
        app.send(current.stop(fade).nonblocking());
        if !fade.is_zero() {
          self.retiring = Some(current);
        }
      }
      if let Some(playback) = playback {
        let track = track.expect("enabled playback has a track");
        app.send(
          playback.play_command(
            track.address,
            AudioPlaybackOptions::new()
              .bus(track.bus)
              .looping(track.looping)
              .volume(settings.gain)
              .fade_in(settings.crossfade),
          ),
        );
        if settings.paused {
          app.send(playback.pause().nonblocking());
        }
      }
      self.current = playback;
    } else if let Some(current) = self.current {
      let previous = self.settings.expect("existing playback has settings");
      if previous.gain != settings.gain {
        app.send(current.set_volume(settings.gain).nonblocking());
      }
      if previous.paused != settings.paused {
        // A retiring clip is already fading out; backgrounding ends its ownership.
        if let Some(retiring) = self.retiring.take() {
          app.send(retiring.stop(Duration::ZERO).nonblocking());
        }
        app.send(if settings.paused {
          current.pause().nonblocking()
        } else {
          current.resume().nonblocking()
        });
      }
    }
    self.settings = Some(settings);
  }

  fn stop(&mut self, app: &AppHandle) {
    for playback in [self.current.take(), self.retiring.take()]
      .into_iter()
      .flatten()
    {
      app.send(playback.stop(Duration::ZERO).nonblocking());
    }
  }
}
