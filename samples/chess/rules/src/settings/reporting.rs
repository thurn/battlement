//! Saved reporting intent and truthful local-control feedback.

use battlement::{Command, host_settings::SettingAvailability};
use battlement_cloud::diagnostics::DiagnosticsCommand;
use reactant::{app_context, hooks};
use trox::{LocalizedString, tx};

use crate::settings;
use battlement::host_settings::HostSettings;

/// Applies hydrated intent before game consumers start their effects.
pub fn use_reporting() {
  let enabled = settings::use_settings().desired.upload_crash_reports;
  let supported = reactant::use_host_settings().diagnostics == SettingAvailability::Available;
  let app = app_context::use_app();
  hooks::use_effect(
    move || {
      if supported {
        app.send(Command::diagnostics(DiagnosticsCommand::SetReporting(
          enabled,
        )));
      }
    },
    (supported, enabled),
  );
}

/// Separates the stored request from local readback and service configuration.
pub fn feedback(host: &HostSettings, enabled: bool) -> LocalizedString {
  if host.diagnostics == SettingAvailability::Failed || host.diagnostics_error.is_some() {
    return tx(
      "Some reporting controls could not be applied.",
      "Local reporting control failure.",
    );
  }
  if !host.diagnostics_configured {
    return tx(
      "Reporting is not configured in this build.",
      "Missing Unity reporting configuration.",
    );
  }
  if host.capture_exceptions != Some(enabled) {
    return tx(
      "Local capture has not confirmed your reporting choice.",
      "Reporting request differs from capture readback.",
    );
  }
  if host.performance_reporting != Some(enabled) {
    return tx(
      "Only some local reporting controls match your choice.",
      "Partial local reporting control.",
    );
  }
  tx(
    "Local controls match your choice. Upload behavior is not verified.",
    "Local reporting readback is not an upload guarantee.",
  )
}
