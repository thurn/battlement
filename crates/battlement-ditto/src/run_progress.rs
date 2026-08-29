use std::{io::Write, path::Path};

use anyhow::Result;
use battlement_tooling::build_cache::NearestBuildMismatch;

use crate::wire::result::{BuildDisposition, RunResult, RunStatus};

pub(crate) fn build_label(disposition: BuildDisposition) -> &'static str {
  match disposition {
    BuildDisposition::Created => "created",
    BuildDisposition::Reused => "reused",
    BuildDisposition::RequiredByNoBuild => "required-by-no-build",
    BuildDisposition::Failed => "failed",
  }
}

pub(crate) fn no_build_message(expected: &str, nearest: Option<&NearestBuildMismatch>) -> String {
  let mut message =
    format!("the exact player build {expected} is not cached and --no-build was supplied");
  let Some(nearest) = nearest else {
    return message;
  };
  message.push_str(&format!("; nearest cached build {}", nearest.fingerprint));
  append_list(&mut message, "changed inputs", &nearest.changed_inputs);
  let paths = nearest
    .added_paths
    .iter()
    .map(|path| format!("added:{path}"))
    .chain(
      nearest
        .removed_paths
        .iter()
        .map(|path| format!("removed:{path}")),
    )
    .chain(
      nearest
        .changed_paths
        .iter()
        .map(|path| format!("changed:{path}")),
    )
    .collect::<Vec<_>>();
  append_list(&mut message, "changed paths", &paths);
  message
}

pub(crate) fn write_handoff(stderr: &mut dyn Write, result: &RunResult, path: &Path) -> Result<()> {
  writeln!(stderr, "DITTO_STATUS={}", status_label(result.status))?;
  writeln!(stderr, "DITTO_EXIT_CODE={}", result.exit_code)?;
  writeln!(stderr, "DITTO_RUN_ID={}", result.run_id)?;
  writeln!(stderr, "DITTO_RESULT={}", path.display())?;
  if let Some(log) = result
    .build
    .as_ref()
    .and_then(|build| build.log_path.as_ref())
  {
    writeln!(
      stderr,
      "DITTO_BUILD_LOG={}",
      path
        .parent()
        .expect("result has a parent")
        .join(log)
        .display()
    )?;
  }
  for error in &result.errors {
    writeln!(stderr, "{}: {}", error.id, error.message)?;
  }
  Ok(())
}

fn append_list(message: &mut String, label: &str, values: &[String]) {
  if values.is_empty() {
    return;
  }
  let shown = values.iter().take(8).cloned().collect::<Vec<_>>();
  message.push_str(&format!("; {label}: {}", shown.join(", ")));
  if values.len() > shown.len() {
    message.push_str(&format!(" (+{} more)", values.len() - shown.len()));
  }
}

fn status_label(status: RunStatus) -> &'static str {
  match status {
    RunStatus::Passed => "passed",
    RunStatus::Failed => "failed",
    RunStatus::InfrastructureError => "infrastructure-error",
    RunStatus::Interrupted => "interrupted",
  }
}
