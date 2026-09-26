//! Validated local chess preferences, separate from saved game progress.

use battlement::{
  PhysicalKey,
  host_settings::{
    DisplayConfiguration, DisplayMode, DisplayResolution, HostPlatform, HostSettings,
  },
};
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use serde_json::Value;

use crate::settings::bindings::{self, Bindings, ControllerBinding};

pub const FILE_NAME: &str = "chess-settings.json";

/// Stable language identity; labels belong to presentation.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
  #[default]
  English,
  French,
}

/// Discrete readable-text scale chosen by the player.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextSize {
  #[default]
  #[serde(rename = "100")]
  Percent100,
  #[serde(rename = "150")]
  Percent150,
  #[serde(rename = "200")]
  Percent200,
}

/// Desired local preferences. Unspecified host-dependent values use current capabilities.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ChessSettings {
  pub language: Language,
  pub text_size: TextSize,
  pub reduce_motion: bool,
  pub increase_move_duration: bool,
  pub upload_crash_reports: bool,
  pub resolution: Option<DisplayResolution>,
  pub max_framerate: Option<u32>,
  pub display_mode: DisplayMode,
  pub screenshake: bool,
  pub vsync: bool,
  pub master_volume: u32,
  pub music_volume: u32,
  pub effects_volume: u32,
  pub mute_in_background: bool,
  pub keyboard: Bindings<PhysicalKey>,
  pub controller: Bindings<ControllerBinding>,
}

/// One validated preference mutation, independent of the active route.
#[derive(Clone, Copy)]
pub enum SettingsChange {
  Language(Language),
  TextSize(TextSize),
  ReduceMotion(bool),
  IncreaseMoveDuration(bool),
  UploadCrashReports(bool),
  Display(DisplayConfiguration),
  MaxFramerate(u32),
  Screenshake(bool),
  Vsync(bool),
  MasterVolume(u32),
  AdjustMasterVolume(i32),
  MusicVolume(u32),
  EffectsVolume(u32),
  MuteInBackground(bool),
  Keyboard(Bindings<PhysicalKey>),
  Controller(Bindings<ControllerBinding>),
}

impl Default for ChessSettings {
  fn default() -> Self {
    Self {
      language: Language::English,
      text_size: TextSize::Percent100,
      reduce_motion: false,
      increase_move_duration: true,
      upload_crash_reports: true,
      resolution: None,
      max_framerate: None,
      display_mode: DisplayMode::Borderless,
      screenshake: true,
      vsync: true,
      master_volume: 80,
      music_volume: 65,
      effects_volume: 75,
      mute_in_background: false,
      keyboard: Bindings::<PhysicalKey>::default(),
      controller: Bindings::<ControllerBinding>::default(),
    }
  }
}

impl ChessSettings {
  /// Chooses the documented initial cap without overwriting a saved preference.
  pub fn framerate(self, host: &HostSettings) -> u32 {
    self.max_framerate.unwrap_or_else(|| {
      if host.platform == HostPlatform::Ios {
        host
          .frame_rates
          .iter()
          .copied()
          .filter(|rate| *rate <= 60)
          .max()
          .unwrap_or(60)
      } else {
        144
      }
    })
  }

  pub fn resolution(self, host: &HostSettings) -> Option<DisplayResolution> {
    self
      .resolution
      .or_else(|| host.applied_display.map(|display| display.resolution))
  }

  fn validated(mut self) -> Self {
    let defaults = Self::default();
    self.resolution = self
      .resolution
      .filter(|resolution| self::valid_resolution(*resolution));
    self.max_framerate = self
      .max_framerate
      .filter(|rate| self::valid_framerate(*rate));
    if self.master_volume > 100 {
      self.master_volume = defaults.master_volume;
    }
    if self.music_volume > 100 {
      self.music_volume = defaults.music_volume;
    }
    if self.effects_volume > 100 {
      self.effects_volume = defaults.effects_volume;
    }
    if !bindings::valid_keyboard(self.keyboard) {
      self.keyboard = defaults.keyboard;
    }
    if !bindings::valid_controller(self.controller) {
      self.controller = defaults.controller;
    }
    self
  }
}

impl SettingsChange {
  pub fn apply(self, mut value: ChessSettings) -> ChessSettings {
    match self {
      Self::Language(next) => value.language = next,
      Self::TextSize(next) => value.text_size = next,
      Self::ReduceMotion(next) => value.reduce_motion = next,
      Self::IncreaseMoveDuration(next) => value.increase_move_duration = next,
      Self::UploadCrashReports(next) => value.upload_crash_reports = next,
      Self::Display(next) if self::valid_resolution(next.resolution) => {
        value.resolution = Some(next.resolution);
        value.display_mode = next.mode;
      }
      Self::MaxFramerate(next) if self::valid_framerate(next) => value.max_framerate = Some(next),
      Self::Screenshake(next) => value.screenshake = next,
      Self::Vsync(next) => value.vsync = next,
      Self::MasterVolume(next) if next <= 100 => value.master_volume = next,
      Self::AdjustMasterVolume(delta) => {
        value.master_volume = value.master_volume.saturating_add_signed(delta).min(100);
      }
      Self::MusicVolume(next) if next <= 100 => value.music_volume = next,
      Self::EffectsVolume(next) if next <= 100 => value.effects_volume = next,
      Self::MuteInBackground(next) => value.mute_in_background = next,
      Self::Keyboard(next) if bindings::valid_keyboard(next) => value.keyboard = next,
      Self::Controller(next) if bindings::valid_controller(next) => value.controller = next,
      _ => {}
    }
    value
  }
}

impl<'de> Deserialize<'de> for ChessSettings {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let value = Value::deserialize(deserializer)?;
    let defaults = Self::default();
    Ok(
      Self {
        language: self::field(&value, "language", defaults.language),
        text_size: self::field(&value, "text_size", defaults.text_size),
        reduce_motion: self::field(&value, "reduce_motion", defaults.reduce_motion),
        increase_move_duration: self::field(
          &value,
          "increase_move_duration",
          defaults.increase_move_duration,
        ),
        upload_crash_reports: self::field(
          &value,
          "upload_crash_reports",
          defaults.upload_crash_reports,
        ),
        resolution: self::field(&value, "resolution", defaults.resolution),
        max_framerate: self::field(&value, "max_framerate", defaults.max_framerate),
        display_mode: self::field(&value, "display_mode", defaults.display_mode),
        screenshake: self::field(&value, "screenshake", defaults.screenshake),
        vsync: self::field(&value, "vsync", defaults.vsync),
        master_volume: self::field(&value, "master_volume", defaults.master_volume),
        music_volume: self::field(&value, "music_volume", defaults.music_volume),
        effects_volume: self::field(&value, "effects_volume", defaults.effects_volume),
        mute_in_background: self::field(&value, "mute_in_background", defaults.mute_in_background),
        keyboard: self::field(&value, "keyboard", defaults.keyboard),
        controller: self::field(&value, "controller", defaults.controller),
      }
      .validated(),
    )
  }
}

fn field<T: DeserializeOwned>(value: &Value, name: &str, default: T) -> T {
  value
    .get(name)
    .and_then(|value| serde_json::from_value(value.clone()).ok())
    .unwrap_or(default)
}

fn valid_resolution(value: DisplayResolution) -> bool {
  value.width > 0 && value.height > 0 && value.refresh_denominator > 0
}

fn valid_framerate(value: u32) -> bool {
  [30, 60, 120, 144, 240].contains(&value)
}
