//! Shared looping music state and native playback controls.

use std::time::Duration;

use battlement::{AudioClipAddress, ObjectId, object_id};
use battlement_reactant::{application, context::ContextProvider, hooks, prelude::*};

/// Address of the source application's looping background track.
pub const BACKGROUND_MUSIC: AudioClipAddress =
  AudioClipAddress::from_static("chess-ui/audio/drag-and-dread");

const PLAYBACK_ID: ObjectId = object_id!("31000000-0000-4000-8000-000000000001");

/// Host playback availability exposed by the review fixture.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum PlaybackAvailability {
  #[default]
  Available,
  Unavailable,
}

/// Current lifecycle state for the shared background track.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackgroundMusicStatus {
  Stopped,
  Playing,
  Unavailable,
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
  /// Latest host audio playhead sample.
  pub playhead: Duration,
  app: AppHandle,
  audio: AudioPlayback,
  availability: PlaybackAvailability,
  set_master_volume: StateSetter<u32>,
  set_music_volume: StateSetter<u32>,
  set_mute_in_background: StateSetter<bool>,
  set_sound_muted: StateSetter<bool>,
  set_playing: StateSetter<bool>,
  set_playhead: StateSetter<Duration>,
  playback_active: hooks::Ref<bool>,
}

impl BackgroundMusicContext {
  /// Requests looping playback when the host can supply the track.
  pub fn start_music(&self) {
    if self.availability == PlaybackAvailability::Unavailable {
      self.set_playing.set(false);
      self.set_playhead.set(Duration::ZERO);
      return;
    }
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

  /// Stops playback and restores the source defaults.
  pub fn reset(&self) {
    if self.playback_active.replace(false) {
      self.app.send(self.audio.stop(Duration::ZERO));
    }
    self.set_master_volume.set(80);
    self.set_music_volume.set(65);
    self.set_mute_in_background.set(false);
    self.set_sound_muted.set(false);
    self.set_playing.set(false);
    self.set_playhead.set(Duration::ZERO);
  }

  /// Returns the native audio clock shared by music-synchronized visuals.
  pub fn motion_time_source(&self) -> MotionTimeSource {
    MotionTimeSource::Audio(self.audio)
  }

  pub(crate) fn seek_for_review(&self, position: Duration) {
    if self.playback_active.get() {
      self.app.send(self.audio.seek(position));
      self.set_playhead.set(position);
    }
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
}

impl Component for BackgroundMusicProvider {
  fn render(&self) -> impl Render {
    let context = use_background_music_provider();
    ContextProvider::new()
      .context(context)
      .child(self.children.render())
  }
}

/// Reads the nearest shared background-music state and actions.
pub fn use_background_music() -> BackgroundMusicContext {
  hooks::use_required_context::<BackgroundMusicContext>()
}

pub(crate) fn availability_provider(
  availability: PlaybackAvailability,
  child: impl Render,
) -> impl Render {
  ContextProvider::new().context(availability).child(child)
}

fn use_background_music_provider() -> BackgroundMusicContext {
  let app = use_app();
  let application = application::use_application_state();
  let availability = hooks::use_context::<PlaybackAvailability>();
  let (master_volume, set_master_volume) = hooks::use_state(80_u32);
  let (music_volume, set_music_volume) = hooks::use_state(65_u32);
  let (mute_in_background, set_mute_in_background) = hooks::use_state(false);
  let (sound_muted, set_sound_muted) = hooks::use_state(false);
  let (playing, set_playing) = hooks::use_state(false);
  let (playhead, set_playhead) = hooks::use_state(Duration::ZERO);
  let playback_active = hooks::use_ref(false);
  let audio = AudioPlayback::new(PLAYBACK_ID);
  let audio_time = use_motion_time(MotionTimeSource::Audio(audio));
  use_motion_value_event(audio_time, MotionValueEvent::AnimationFrame, {
    let set_playhead = set_playhead.clone();
    move |sample| set_playhead.set(sample)
  });

  let visible = !application.paused;
  let effective_volume = f64::from(master_volume) * f64::from(music_volume) / 10_000.0;
  let muted = sound_muted || (mute_in_background && !visible);
  let status = match (availability, playing) {
    (PlaybackAvailability::Unavailable, _) => BackgroundMusicStatus::Unavailable,
    (PlaybackAvailability::Available, true) => BackgroundMusicStatus::Playing,
    (PlaybackAvailability::Available, false) => BackgroundMusicStatus::Stopped,
  };
  let output_volume = if muted { 0.0 } else { effective_volume };

  hooks::use_effect(
    {
      let app = app.clone();
      move || {
        if playing {
          app.send(audio.set_volume(output_volume));
        }
      }
    },
    (playing, output_volume),
  );
  hooks::use_effect(
    {
      let app = app.clone();
      let set_playing = set_playing.clone();
      let set_playhead = set_playhead.clone();
      let playback_active = playback_active.clone();
      move || {
        if availability == PlaybackAvailability::Unavailable && playing {
          app.send(audio.stop(Duration::ZERO));
          playback_active.replace(false);
          set_playing.set(false);
          set_playhead.set(Duration::ZERO);
        }
      }
    },
    availability,
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
    playhead,
    app,
    audio,
    availability,
    set_master_volume,
    set_music_volume,
    set_mute_in_background,
    set_sound_muted,
    set_playing,
    set_playhead,
    playback_active,
  }
}
