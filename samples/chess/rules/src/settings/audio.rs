//! Application-wide audio mix and transient music-only mute.

use battlement::AudioMix;
use reactant::{application, audio::AudioMixProvider, hooks, prelude::*};

use crate::settings;

/// Music-indicator intent, independent of persisted volume preferences.
#[derive(Clone, PartialEq)]
pub struct AudioContext {
  pub music_muted: bool,
  pub set_music_muted: StateSetter<bool>,
}

/// Applies hydrated settings to every active and future sound.
pub struct AudioSettings {
  pub children: Children,
}

impl Component for AudioSettings {
  fn render(&self) -> impl Render {
    let settings = settings::use_settings().desired;
    let application = application::use_application_state();
    let (music_muted, set_music_muted) = hooks::use_state(false);
    let mix = AudioMix {
      master: f64::from(settings.master_volume) / 100.0,
      music: if music_muted {
        0.0
      } else {
        f64::from(settings.music_volume) / 100.0
      },
      effects: f64::from(settings.effects_volume) / 100.0,
      muted: settings.mute_in_background && !application.is_active(),
    };
    ContextProvider::new()
      .context(AudioContext {
        music_muted,
        set_music_muted,
      })
      .child(AudioMixProvider {
        mix,
        children: self.children.clone(),
      })
  }
}
