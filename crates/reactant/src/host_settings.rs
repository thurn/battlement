use battlement::host_settings::HostSettings;
use reactant_core::{app_context::HostEnvironment, hooks};

/// Reads current host capabilities without owning or resetting user preferences.
pub fn use_host_settings() -> HostSettings {
  hooks::use_required_context::<HostEnvironment>().settings
}
