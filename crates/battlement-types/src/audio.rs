/// Shared routing category for a playing sound.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AudioBus {
  /// Background and foreground music tracks.
  Music,
  /// One-shot and sequence sound effects.
  #[default]
  Effects,
}

/// Host-owned gains applied to every active and future audio source.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioMix {
  /// Linear master gain in the inclusive range zero through one.
  pub master: f64,
  /// Linear music gain in the inclusive range zero through one.
  pub music: f64,
  /// Linear effects gain in the inclusive range zero through one.
  pub effects: f64,
  /// Whether all buses are muted without stopping playback.
  pub muted: bool,
}

impl AudioMix {
  /// Returns the shared gain for a source on this bus.
  #[must_use]
  pub fn gain(self, bus: AudioBus) -> f64 {
    if self.muted {
      return 0.0;
    }
    self.master
      * match bus {
        AudioBus::Music => self.music,
        AudioBus::Effects => self.effects,
      }
  }
}

impl Default for AudioMix {
  fn default() -> Self {
    Self {
      master: 1.0,
      music: 1.0,
      effects: 1.0,
      muted: false,
    }
  }
}
