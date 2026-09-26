use battlement::{
  Command, CommandBody, CommandId, display::DisplayCommand, frame_pacing::FramePacing,
  host_settings::DisplayConfiguration, host_settings::HostSettings,
};
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

/// Starts a host-timed display preview; keep its command ID for confirmation or cancellation.
pub fn preview_display(configuration: DisplayConfiguration) -> Command {
  let operation = DisplayCommand::Preview(configuration);
  assert!(operation.is_valid(), "invalid display configuration");
  Command::new_v4(CommandBody::ApplicationDisplay(operation))
}

/// Keeps the matching preview only after host readback and durable confirmation succeed.
pub fn confirm_display(preview_id: CommandId) -> Command {
  Command::new_v4(CommandBody::ApplicationDisplay(DisplayCommand::Confirm(
    preview_id,
  )))
}

/// Restores the prior display when this preview still owns the host transaction.
pub fn cancel_display(preview_id: CommandId) -> Command {
  Command::new_v4(CommandBody::ApplicationDisplay(DisplayCommand::Cancel(
    preview_id,
  )))
}
