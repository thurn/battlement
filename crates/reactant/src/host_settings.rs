use battlement::{Command, CommandBody, frame_pacing::FramePacing, host_settings::HostSettings};
use reactant_core::{app_context::HostEnvironment, hooks};

/// Reads current host capabilities without owning or resetting user preferences.
pub fn use_host_settings() -> HostSettings {
  hooks::use_required_context::<HostEnvironment>().settings
}

/// Requests a supported pacing policy; observe use_host_settings for applied values and outcome.
pub fn set_frame_pacing(preference: FramePacing) -> Command {
  assert!(
    (1..=1000).contains(&preference.maximum_frame_rate),
    "invalid frame-rate ceiling"
  );
  Command::new_v4(CommandBody::ApplicationSetFramePacing(preference))
}
