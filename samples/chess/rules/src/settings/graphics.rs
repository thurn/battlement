//! Hydrated frame-pacing intent follows capabilities independently of display previews.

use battlement::{frame_pacing::FramePacing, host_settings::SettingAvailability};
use reactant::{app_context, hooks};

use crate::settings;

pub fn use_frame_pacing() {
  let value = settings::use_settings().desired;
  let host = reactant::use_host_settings();
  let app = app_context::use_app();
  let supported = host.frame_pacing == SettingAvailability::Available;
  let preference = FramePacing {
    maximum_frame_rate: value.framerate(&host),
    vsync: value.vsync,
  };
  hooks::use_effect(
    move || {
      if supported {
        app.send(reactant::set_frame_pacing(preference));
      }
    },
    (
      supported,
      preference,
      host.frame_rates,
      host.vsync_available,
    ),
  );
}
