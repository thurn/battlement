use std::{io::Write, path::Path, sync::atomic::AtomicBool};

use anyhow::{Context, Result, ensure};

use crate::{
  cli::{ProfileOptions, SelectionOptions},
  config,
  config::model::{PerformancePass, ScenarioPerformance, Target},
  run_commands::{self, ExecuteOptions},
  selection::{self, Disposition},
  wire::result::ResultCommand,
};

const IDLE_FRAMES: u32 = 120;
pub(crate) const MEASURED_ITERATIONS: u32 = 5;

pub(crate) fn profile(
  config_path: Option<&Path>,
  options: ProfileOptions,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  let mut suite = config::load(config_path)?;
  let performance = suite
    .performance
    .context("ditto profile requires [performance] with target_fps")?;
  let selected = selection::resolve(
    &suite,
    &selection::Options {
      profile: options.selection.profile.clone(),
      includes: options.selection.includes.clone(),
      excludes: options.selection.excludes.clone(),
      allow_empty: false,
    },
  )?;
  ensure!(
    selected.profile.target() == Target::Macos,
    "profile currently supports macOS only"
  );
  ensure!(
    selected.scenarios.len() == 1,
    "ditto profile requires exactly one scenario"
  );
  let selected_scenario = &selected.scenarios[0];
  ensure!(
    selected_scenario.disposition == Disposition::Runnable,
    "selected performance scenario is not runnable"
  );
  ensure!(
    selected_scenario
      .scenario
      .steps
      .iter()
      .any(|step| step.measure),
    "profile scenario requires at least one measured pointer_action step"
  );
  let base_name = selected_scenario.scenario.name.clone();
  let mut scenarios = Vec::new();
  for pass in [PerformancePass::Score, PerformancePass::Detail] {
    scenarios.push(attempt(
      &selected_scenario.scenario,
      pass,
      true,
      0,
      performance.target_fps,
      &base_name,
    ));
    for iteration in 1..=MEASURED_ITERATIONS {
      scenarios.push(attempt(
        &selected_scenario.scenario,
        pass,
        false,
        iteration,
        performance.target_fps,
        &base_name,
      ));
    }
  }
  suite.scenarios = scenarios;
  let base_source = suite.source.clone();
  run_commands::execute(
    suite,
    ExecuteOptions {
      selection: SelectionOptions {
        includes: Vec::new(),
        excludes: Vec::new(),
        profile: options.selection.profile,
        allow_empty: false,
      },
      command: ResultCommand::Profile,
      update: false,
      bail_after: None,
      no_build: options.no_build,
      filtered: true,
      json: options.json,
      output: options.output,
      review: false,
      watch: false,
      base_source,
      fragment_source: None,
      native_execution: None,
    },
    stdout,
    stderr,
    interrupted,
  )
}

fn attempt(
  source: &crate::config::model::Scenario,
  pass: PerformancePass,
  warmup: bool,
  iteration: u32,
  target_fps: u32,
  base_name: &str,
) -> crate::config::model::Scenario {
  let mut scenario = source.clone();
  let pass_name = match pass {
    PerformancePass::Score => "score",
    PerformancePass::Detail => "detail",
  };
  let iteration_name = if warmup {
    "warmup".to_owned()
  } else {
    format!("iteration-{iteration}")
  };
  scenario.name = format!("{base_name} [{pass_name} {iteration_name}]");
  scenario.performance = Some(ScenarioPerformance {
    pass,
    warmup,
    iteration,
    target_fps,
    idle_frames: IDLE_FRAMES,
  });
  scenario
}
