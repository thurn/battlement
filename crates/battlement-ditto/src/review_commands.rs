use std::{
  io::Write,
  path::Path,
  process::Command as ProcessCommand,
  sync::atomic::AtomicBool,
  time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};

use crate::{
  cli::ReviewOptions,
  config::{self, model::Suite},
  maintenance_commands,
  review_acceptance::ReviewAcceptanceService,
  review_server::ReviewServer,
  wire::{
    result::{BaselineOutcome, ComparisonOutcome, ResultCommand, RunResult, ScreenshotResult},
    run_storage::RunStore,
  },
};

pub(crate) fn review(
  config_path: Option<&Path>,
  options: ReviewOptions,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  let suite = config::load(config_path)?;
  serve(&suite, options.run.as_deref(), stderr, interrupted)?;
  Ok(0)
}

pub(crate) fn serve(
  suite: &Suite,
  requested: Option<&str>,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<()> {
  let roots = maintenance_commands::cache_roots(suite)?;
  let mut store = RunStore::open(&roots.runs)?;
  let run_id = select_run(&store, suite, requested)?;
  let result = store.load_result(&run_id, unix_time()?)?;
  let directory = store.run_directory(&run_id)?;
  let acceptance =
    ReviewAcceptanceService::open(suite.clone(), roots.runs, result.clone(), directory.clone());
  let server = match acceptance {
    Ok(acceptance) => ReviewServer::bind_accepting(directory, result, acceptance)?,
    Err(error) => ReviewServer::bind_disabled(
      directory,
      result,
      format!("Baseline acceptance is read-only: {error:#}"),
    )?,
  };
  let url = server.url();
  writeln!(stderr, "DITTO_REVIEW_RUN={run_id}")?;
  writeln!(stderr, "DITTO_REVIEW_URL={url}")?;
  stderr.flush()?;
  open_browser(&url)?;
  server.serve(interrupted)
}

fn select_run(store: &RunStore, suite: &Suite, requested: Option<&str>) -> Result<String> {
  if let Some(run_id) = requested {
    store.peek_result(run_id)?;
    return Ok(run_id.to_owned());
  }
  let repository = suite.repository.canonicalize()?;
  let repository = repository.to_string_lossy();
  let mut capture = None;
  for entry in store.entries().iter().rev() {
    if entry.repository.as_deref() != Some(repository.as_ref()) {
      continue;
    }
    if entry.suite.as_deref() != Some(suite.name.as_str()) {
      continue;
    }
    let Ok(result) = store.peek_result(&entry.run_id) else {
      continue;
    };
    if has_mismatch_or_missing(&result) {
      return Ok(entry.run_id.clone());
    }
    if capture.is_none() && is_image_capture(&result) {
      capture = Some(entry.run_id.clone());
    }
  }
  capture.context("no retained image mismatch, missing baseline, or image capture is reviewable")
}

fn has_mismatch_or_missing(result: &RunResult) -> bool {
  result.scenarios.iter().any(|scenario| {
    scenario.steps.iter().any(|step| {
      matches!(
        &step.screenshot,
        Some(ScreenshotResult::Captured {
          baseline: BaselineOutcome::Missing,
          ..
        }) | Some(ScreenshotResult::Captured {
          comparison: Some(ComparisonOutcome::Mismatch { .. }),
          ..
        })
      )
    })
  })
}

fn is_image_capture(result: &RunResult) -> bool {
  result.command == ResultCommand::Capture
    && result.scenarios.iter().any(|scenario| {
      scenario
        .steps
        .iter()
        .any(|step| matches!(step.screenshot, Some(ScreenshotResult::Captured { .. })))
    })
}

fn open_browser(url: &str) -> Result<()> {
  let status = ProcessCommand::new(browser_opener())
    .arg(url)
    .status()
    .context("open the review application in a browser")?;
  if !status.success() {
    bail!("browser opener exited with status {status}");
  }
  Ok(())
}

#[cfg(target_os = "macos")]
fn browser_opener() -> &'static str {
  "open"
}

#[cfg(not(target_os = "macos"))]
fn browser_opener() -> &'static str {
  "xdg-open"
}

fn unix_time() -> Result<u64> {
  Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

#[cfg(test)]
#[path = "review_commands_tests.rs"]
mod tests;
