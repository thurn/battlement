//! Requested frame pacing is independent of display preview transactions.

use crate::host_settings::{HostSettings, SettingAvailability};

/// Saved pacing intent; supported caps are selected from the current host snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FramePacing {
  /// Positive FPS ceiling, not an achieved frame-rate promise.
  pub maximum_frame_rate: u32,
  /// Desktop refresh synchronization; ignored on hosts without VSync control.
  pub vsync: bool,
}

impl FramePacing {
  /// Applies deterministic policy to a fake host observation, preserving requested intent.
  pub fn apply_to(self, host: &mut HostSettings) -> bool {
    if host.frame_pacing != SettingAvailability::Available || host.frame_rates.is_empty() {
      return false;
    }
    let rate = host
      .frame_rates
      .iter()
      .copied()
      .filter(|rate| *rate <= self.maximum_frame_rate)
      .max()
      .unwrap_or_else(|| *host.frame_rates.iter().min().expect("nonempty rates"));
    host.applied_vsync = host.vsync_available && self.vsync;
    host.applied_frame_rate = if host.applied_vsync { -1 } else { rate as i32 };
    true
  }
}
