use battlement::AudioMix;
use reactant::GameVersion;
use serde::{Deserialize, Serialize};

use crate::{domain::HeartsState, settings::Preferences};

/// Durable accepted match; deserialization rejects inconsistent game history.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedMatch")]
pub struct SavedMatch {
  pub session: u64,
  pub revision: u64,
  pub state: HeartsState,
}

#[derive(Deserialize)]
struct UncheckedMatch {
  session: u64,
  revision: u64,
  state: HeartsState,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedSettings")]
pub(crate) struct SavedSettings {
  pub preferences: Preferences,
  pub music: f64,
  pub effects: f64,
  pub muted: bool,
}

#[derive(Deserialize)]
struct UncheckedSettings {
  preferences: Preferences,
  music: f64,
  effects: f64,
  muted: bool,
}

impl SavedMatch {
  pub fn capture(version: GameVersion, state: &HeartsState) -> Self {
    Self {
      session: version.session,
      revision: version.revision,
      state: state.clone(),
    }
  }
}

impl TryFrom<UncheckedMatch> for SavedMatch {
  type Error = &'static str;

  fn try_from(value: UncheckedMatch) -> Result<Self, Self::Error> {
    value.state.validate_saved()?;
    Ok(Self {
      session: value.session,
      revision: value.revision,
      state: value.state,
    })
  }
}

impl SavedSettings {
  pub fn capture(preferences: Preferences, mix: AudioMix) -> Self {
    Self {
      preferences,
      music: mix.music,
      effects: mix.effects,
      muted: mix.muted,
    }
  }

  pub fn mix(self) -> AudioMix {
    AudioMix {
      music: self.music,
      effects: self.effects,
      muted: self.muted,
      ..AudioMix::default()
    }
  }
}

impl Default for SavedSettings {
  fn default() -> Self {
    Self {
      preferences: Preferences::default(),
      music: 0.22,
      effects: 0.65,
      muted: false,
    }
  }
}

impl TryFrom<UncheckedSettings> for SavedSettings {
  type Error = &'static str;

  fn try_from(value: UncheckedSettings) -> Result<Self, Self::Error> {
    if ![value.music, value.effects]
      .iter()
      .all(|level| level.is_finite() && (0.0..=1.0).contains(level))
    {
      return Err("saved volume is outside its range");
    }
    Ok(Self {
      preferences: value.preferences,
      music: value.music,
      effects: value.effects,
      muted: value.muted,
    })
  }
}
