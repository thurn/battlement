use std::{
  io::{BufRead, Write},
  path::PathBuf,
  sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
  },
  thread,
  time::{Duration, Instant},
};

use anyhow::{Context, Result};

use crate::{
  comparison_refresh,
  config::{self, FragmentInput, model::Suite},
  run_commands::{self, CompletedCycle, ExecuteOptions},
  storage_commands,
  watch::{ChangeSet, CyclePath, FileObserver, PendingState},
  wire::result::{BuildDisposition, RunResult},
};

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const DEBOUNCE: Duration = Duration::from_millis(150);

pub(crate) fn execute(
  mut suite: Suite,
  options: ExecuteOptions,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  let mut cycle = 1;
  let mut runtime = crate::macos_run::WatchRuntime::default();
  let first = run_commands::execute_watch_cycle(
    suite.clone(),
    &options,
    cycle,
    stdout,
    stderr,
    interrupted,
    &mut runtime,
  )?;
  let mut current = first.result.clone();
  let mut latest_images = current.clone();
  let live = crate::review_commands::start_live(
    &suite,
    first.result,
    first.directory,
    options.review,
    stderr,
  )?;
  let scenario_files = scenario_files(&suite, &options)?;
  let lock = storage_commands::lock_path(&suite);
  let mut observer = FileObserver::new(&suite.repository, scenario_files, lock, DEBOUNCE)?;
  let retries = retry_requests();
  let mut coalescer = PendingState::default();
  let mut blocked = ChangeSet::default();

  while !interrupted.load(Ordering::Acquire) {
    if let Some(accepted) = accepted_cycle(&live, &current)? {
      cycle = accepted.cycle;
      let path = run_result_path(&suite, &accepted.run_id)?;
      run_commands::emit(&accepted, &path, &options, stdout, stderr)?;
      current = accepted;
    }
    if retries.try_recv().is_ok()
      && let Some(retry) = coalescer.retry()
    {
      blocked.merge(retry);
    }
    let Some(mut changes) = observer.poll(Instant::now())? else {
      thread::sleep(POLL_INTERVAL);
      continue;
    };
    changes.merge(std::mem::take(&mut blocked));
    let Some(path) = changes.path() else {
      continue;
    };
    if coalescer.source_is_broken() && path == CyclePath::Execution {
      blocked.merge(changes);
      writeln!(stderr, "DITTO_WATCH=waiting-for-build-input-or-r")?;
      continue;
    }
    let path = coalescer.begin(changes)?;
    cycle = cycle.checked_add(1).context("watch cycle overflow")?;
    let completed = match path {
      CyclePath::ComparisonOnly => {
        let completed = comparison_cycle(&suite, &latest_images, cycle, runtime.odiff(), stderr)?;
        run_commands::emit(
          &completed.result,
          &completed.result_path,
          &options,
          stdout,
          stderr,
        )?;
        completed
      }
      CyclePath::Execution | CyclePath::ReplacementBuild => {
        suite = reload(&options)?;
        run_commands::execute_watch_cycle(
          suite.clone(),
          &options,
          cycle,
          stdout,
          stderr,
          interrupted,
          &mut runtime,
        )?
      }
    };
    let replacement_succeeded =
      (path == CyclePath::ReplacementBuild).then(|| replacement_build_succeeded(&completed.result));
    coalescer.finish(replacement_succeeded);
    live.publish(completed.directory, completed.result.clone())?;
    current = completed.result;
    if has_actual_images(&current) {
      latest_images = current.clone();
    }
  }
  writeln!(stderr, "DITTO_WATCH=stopped")?;
  Ok(130)
}

fn comparison_cycle(
  suite: &Suite,
  source: &RunResult,
  cycle: u32,
  odiff: std::sync::Arc<crate::image_comparison::OdiffPool>,
  stderr: &mut dyn Write,
) -> Result<CompletedCycle> {
  writeln!(stderr, "DITTO_WATCH=comparison-only")?;
  let refreshed = comparison_refresh::refresh(suite, source, cycle, odiff, stderr)?;
  Ok(CompletedCycle {
    result: refreshed.result,
    result_path: refreshed.result_path,
    directory: refreshed.directory,
  })
}

fn accepted_cycle(
  live: &crate::review_commands::LiveReview,
  current: &RunResult,
) -> Result<Option<RunResult>> {
  let reviewed = live.current_result()?;
  Ok((reviewed.run_id != current.run_id).then_some(reviewed))
}

fn reload(options: &ExecuteOptions) -> Result<Suite> {
  let base = config::load(Some(&options.base_source))?;
  match &options.fragment_source {
    Some(fragment) => config::load_fragment(&base, FragmentInput::File(fragment.clone()), true),
    None => Ok(base),
  }
}

fn scenario_files(suite: &Suite, options: &ExecuteOptions) -> Result<Vec<PathBuf>> {
  let mut paths = vec![options.base_source.clone()];
  if let Some(fragment) = &options.fragment_source {
    paths.push(fragment.clone());
  }
  anyhow::ensure!(
    suite.repository.is_dir(),
    "watch repository is not a directory"
  );
  Ok(paths)
}

fn run_result_path(suite: &Suite, run_id: &str) -> Result<PathBuf> {
  Ok(
    crate::maintenance_commands::cache_roots(suite)?
      .runs
      .join(run_id)
      .join("result.json"),
  )
}

fn replacement_build_succeeded(result: &RunResult) -> bool {
  !matches!(
    result.build.as_ref().map(|build| build.disposition),
    Some(BuildDisposition::Failed | BuildDisposition::RequiredByNoBuild)
  )
}

fn has_actual_images(result: &RunResult) -> bool {
  result.scenarios.iter().any(|scenario| {
    scenario.steps.iter().any(|step| {
      matches!(
        step.screenshot,
        Some(crate::wire::result::ScreenshotResult::Captured { .. })
      )
    })
  })
}

fn retry_requests() -> Receiver<()> {
  let (sender, receiver) = mpsc::channel();
  thread::spawn(move || {
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
      if line.trim() == "r" && sender.send(()).is_err() {
        break;
      }
    }
  });
  receiver
}
