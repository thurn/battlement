use battlement_tooling::discovery::HostDiscovery;

use crate::{
  macos_run,
  selection::Selection,
  wire::{
    common::{ErrorCode, ErrorSource},
    result::{
      ErrorOccurrence, PhaseName, PhaseResult, PhaseStatus, ResultCommand, RunResult, RunStatus,
    },
  },
};

pub(crate) fn comparison(
  discovery: &HostDiscovery,
  selection: &Selection,
  command: ResultCommand,
  result: &mut RunResult,
) -> bool {
  if command != ResultCommand::Run || !macos_run::selection_has_screenshots(selection) {
    return true;
  }
  if let Err(error) = macos_run::required_tool(&discovery.odiff) {
    let id = format!("E{:04}", result.errors.len() + 1);
    result.errors.push(ErrorOccurrence {
      id: id.clone(),
      code: ErrorCode::ImageComparisonFailed,
      source: ErrorSource::Ditto,
      message: format!(
        "Comparison preflight: {error:#}. Set DITTO_ODIFF_PATH to an executable ODiff binary."
      ),
      job_id: None,
      player_session_id: None,
      scenario_id: None,
      step_index: None,
      log_sequence: None,
    });
    result.phases.push(PhaseResult {
      name: PhaseName::Discovery,
      status: PhaseStatus::Failed,
      duration_ms: 0,
      expired_deadline: None,
      log_path: None,
      error_ids: vec![id],
    });
    result.status = RunStatus::InfrastructureError;
    result.exit_code = 2;
    return false;
  }
  true
}
