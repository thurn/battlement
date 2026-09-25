//! Host-observed settings capabilities and applied state.

use crate::CommandId;

/// Platform classes with supported settings policies.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum HostPlatform {
  #[default]
  /// Host does not expose this capability.
  Unavailable,
  /// Native macOS host.
  MacOs,
  /// Native Windows host.
  Windows,
  /// Browser host.
  Web,
  /// Native iOS host.
  Ios,
}

/// Whether a host can provide a setting, independently of user intent.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SettingAvailability {
  #[default]
  /// Host does not expose this capability.
  Unavailable,
  /// Host exposes this capability.
  Available,
  /// Host supports this capability but its observation failed.
  Failed,
}

/// Stable display mode identity, independent of presentation labels.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DisplayMode {
  #[default]
  /// Resizable desktop window.
  Windowed,
  /// Borderless window filling the display.
  Borderless,
  /// Exclusive desktop fullscreen.
  Fullscreen,
}

/// Pixel dimensions and exact refresh ratio reported by the host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayResolution {
  /// Pixel width.
  pub width: u32,
  /// Pixel height.
  pub height: u32,
  /// Refresh frequency numerator; zero means unreported.
  pub refresh_numerator: u32,
  /// Nonzero refresh frequency denominator.
  pub refresh_denominator: u32,
}

/// A coupled display mode and resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayConfiguration {
  /// Window presentation mode.
  pub mode: DisplayMode,
  /// Pixel dimensions and refresh rate.
  pub resolution: DisplayResolution,
}

/// Result identity for an explicitly requested host setting operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostSettingsResult {
  /// Command whose outcome is being reported.
  pub request_id: CommandId,
  /// Absence means the requested operation succeeded.
  pub error: Option<String>,
}

/// Latest host capabilities and observed values; never a preference store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostSettings {
  /// Host policy family.
  pub platform: HostPlatform,
  /// Availability of desktop display configuration.
  pub display: SettingAvailability,
  /// Selectable window modes.
  pub display_modes: Vec<DisplayMode>,
  /// Selectable dimensions and refresh ratios.
  pub resolutions: Vec<DisplayResolution>,
  /// Currently observed display configuration, when queryable.
  pub applied_display: Option<DisplayConfiguration>,
  /// Availability of software frame pacing.
  pub frame_pacing: SettingAvailability,
  /// Supported software caps, ordered from low to high.
  pub frame_rates: Vec<u32>,
  /// Current host cap; -1 means the platform default.
  pub applied_frame_rate: i32,
  /// Whether the host exposes desktop VSync control.
  pub vsync_available: bool,
  /// Whether desktop VSync is currently enabled.
  pub applied_vsync: bool,
  /// Whether the host has evidence of usable keyboard input.
  pub keyboard_connected: bool,
  /// Number of enabled attached game controllers.
  pub controller_count: u32,
  /// Availability of the selected diagnostics module on this platform.
  pub diagnostics: SettingAvailability,
  /// Whether the host has the vendor project configuration.
  pub diagnostics_configured: bool,
  /// Observation failures are separate from unavailable capabilities.
  pub observation_error: Option<String>,
  /// Latest explicit setting operation outcome, if any.
  pub last_result: Option<HostSettingsResult>,
}

impl Default for HostSettings {
  fn default() -> Self {
    Self {
      platform: HostPlatform::Unavailable,
      display: SettingAvailability::Unavailable,
      display_modes: Vec::new(),
      resolutions: Vec::new(),
      applied_display: None,
      frame_pacing: SettingAvailability::Unavailable,
      frame_rates: Vec::new(),
      applied_frame_rate: -1,
      vsync_available: false,
      applied_vsync: false,
      keyboard_connected: false,
      controller_count: 0,
      diagnostics: SettingAvailability::Unavailable,
      diagnostics_configured: false,
      observation_error: None,
      last_result: None,
    }
  }
}
