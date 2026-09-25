//! Input availability comes from host observations, never browser identification.
use battlement::{
  GridTrack,
  host_settings::{HostPlatform, HostSettings},
};

#[derive(Clone, Copy)]
pub struct InputDevices {
  pub keyboard: bool,
  pub controller: bool,
}

impl InputDevices {
  pub fn from_host(host: &HostSettings) -> Self {
    Self {
      keyboard: matches!(host.platform, HostPlatform::MacOs | HostPlatform::Windows)
        || host.keyboard_connected,
      controller: host.controller_count > 0,
    }
  }

  pub fn available(self) -> bool {
    self.keyboard || self.controller
  }

  pub fn count(self) -> u32 {
    u32::from(self.keyboard) + u32::from(self.controller)
  }

  pub fn columns(self, scale: f32) -> Vec<GridTrack> {
    if scale > 1.0 {
      vec![GridTrack::fr(1.0); self.count() as usize]
    } else if self.count() == 2 {
      vec![
        GridTrack::px(310.0),
        GridTrack::px(310.0),
        GridTrack::fr(1.0),
      ]
    } else {
      vec![GridTrack::px(310.0), GridTrack::fr(1.0)]
    }
  }
}
