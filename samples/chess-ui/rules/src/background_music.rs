//! Shared looping music state and native playback controls.

use std::time::Duration;

use battlement::{AudioClipAddress, ObjectId, object_id};
use battlement_reactant::{application, context::ContextProvider, hooks, prelude::*};

use crate::music_heartbeat::{self, Heartbeat};

/// Address of the source application's looping background track.
pub const BACKGROUND_MUSIC: AudioClipAddress =
  AudioClipAddress::from_static("chess-ui/audio/drag-and-dread");

const PLAYBACK_ID: ObjectId = object_id!("31000000-0000-4000-8000-000000000001");

/// Current lifecycle state for the shared background track.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackgroundMusicStatus {
  Stopped,
  Playing,
}

/// Values and actions exposed by [`use_background_music`].
#[derive(Clone, PartialEq)]
pub struct BackgroundMusicContext {
  /// Controlled master-volume percentage.
  pub master_volume: u32,
  /// Controlled music-volume percentage.
  pub music_volume: u32,
  /// Whether hidden applications should silence background music.
  pub mute_in_background: bool,
  /// Whether the shared sound toggle has muted music.
  pub sound_muted: bool,
  /// Whether the host currently reports the application as visible.
  pub visible: bool,
  /// Effective unmuted volume after combining both sliders.
  pub effective_volume: f64,
  /// Whether either sound policy currently silences output.
  pub muted: bool,
  /// Current playback lifecycle state.
  pub status: BackgroundMusicStatus,
  /// Shared presentation graph sampled directly from the audio playhead.
  pub heartbeat: Heartbeat,
  app: AppHandle,
  audio: AudioPlayback,
  set_master_volume: StateSetter<u32>,
  set_music_volume: StateSetter<u32>,
  set_mute_in_background: StateSetter<bool>,
  set_sound_muted: StateSetter<bool>,
  set_playing: StateSetter<bool>,
  playback_active: hooks::Ref<bool>,
}

impl BackgroundMusicContext {
  /// Requests looping playback when the host can supply the track.
  pub fn start_music(&self) {
    if self.playback_active.get() {
      self.set_playing.set(true);
      return;
    }
    self.app.send(
      self.audio.play_command(
        BACKGROUND_MUSIC,
        AudioPlaybackOptions::new()
          .volume(self.output_volume())
          .looping(true),
      ),
    );
    self.playback_active.replace(true);
    self.set_playing.set(true);
  }

  /// Replaces the controlled master-volume percentage.
  pub fn set_master_volume(&self, volume: u32) {
    self.set_master_volume.set(volume.min(100));
  }

  /// Replaces the controlled music-volume percentage.
  pub fn set_music_volume(&self, volume: u32) {
    self.set_music_volume.set(volume.min(100));
  }

  /// Changes the hidden-application mute policy.
  pub fn set_mute_in_background(&self, muted: bool) {
    self.set_mute_in_background.set(muted);
  }

  /// Changes the explicit sound mute used by the playback indicator.
  pub fn set_sound_muted(&self, muted: bool) {
    self.set_sound_muted.set(muted);
  }

  fn output_volume(&self) -> f64 {
    if self.muted {
      0.0
    } else {
      self.effective_volume
    }
  }
}

/// Provides one source-shaped background-music context to its descendants.
#[builder]
pub struct BackgroundMusicProvider {
  #[builder(required, into)]
  children: Children,
  /// Exposes playing state before a menu-owned startup effect commits.
  autoplay: bool,
}

impl Component for BackgroundMusicProvider {
  fn render(&self) -> impl Render {
    let context = use_background_music_provider(self.autoplay);
    ContextProvider::new()
      .context(context)
      .child(self.children.render())
  }
}

/// Reads the nearest shared background-music state and actions.
pub fn use_background_music() -> BackgroundMusicContext {
  hooks::use_required_context::<BackgroundMusicContext>()
}

fn use_background_music_provider(autoplay: bool) -> BackgroundMusicContext {
  let app = use_app();
  let application = application::use_application_state();
  let (master_volume, set_master_volume) = hooks::use_state(80_u32);
  let (music_volume, set_music_volume) = hooks::use_state(65_u32);
  let (mute_in_background, set_mute_in_background) = hooks::use_state(false);
  let (sound_muted, set_sound_muted) = hooks::use_state(false);
  let (playing, set_playing) = hooks::use_state(autoplay);
  let playback_active = hooks::use_ref(false);
  let audio = AudioPlayback::new(PLAYBACK_ID);
  let audio_time = use_motion_time(MotionTimeSource::Audio(audio));
  let heartbeat = music_heartbeat::use_heartbeat(audio_time);

  let visible = !application.paused;
  let effective_volume = f64::from(master_volume) * f64::from(music_volume) / 10_000.0;
  let muted = sound_muted || (mute_in_background && !visible);
  let status = if playing {
    BackgroundMusicStatus::Playing
  } else {
    BackgroundMusicStatus::Stopped
  };
  let output_volume = if muted { 0.0 } else { effective_volume };

  hooks::use_effect(
    {
      let app = app.clone();
      let playback_active = playback_active.clone();
      move || {
        if playback_active.get() {
          app.send(audio.set_volume(output_volume));
        }
      }
    },
    (playing, output_volume),
  );
  hooks::use_effect(
    {
      let app = app.clone();
      let playback_active = playback_active.clone();
      move || {
        move || {
          if playback_active.replace(false) {
            app.send(audio.stop(Duration::ZERO));
          }
        }
      }
    },
    (),
  );

  BackgroundMusicContext {
    master_volume,
    music_volume,
    mute_in_background,
    sound_muted,
    visible,
    effective_volume,
    muted,
    status,
    heartbeat,
    app,
    audio,
    set_master_volume,
    set_music_volume,
    set_mute_in_background,
    set_sound_muted,
    set_playing,
    playback_active,
  }
}
