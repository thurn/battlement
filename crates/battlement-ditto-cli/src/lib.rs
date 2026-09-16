//! Legacy Ditto process adapter retained until the `rt` command owns preparation.

use std::{
  ffi::OsString,
  io::{self, Write},
  sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
  },
};

use anyhow::Result;
use battlement_ditto::{config::model::Suite, preparation::PlayerPreparation};
use battlement_reactant_assets::{AssetCommand, CommandOptions, FeatureSelection};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

struct LegacyReactantPreparation;

impl PlayerPreparation for LegacyReactantPreparation {
  fn prepare(&self, suite: &Suite) -> Result<()> {
    if !suite
      .player
      .unity_project
      .join("Packages/manifest.json")
      .is_file()
    {
      return Ok(());
    }
    battlement_reactant_assets::run_quiet(
      AssetCommand::Generate,
      &CommandOptions {
        project: Some(suite.player.unity_project.clone()),
        manifest_path: Some(suite.player.rust_manifest.clone()),
        feature_selection: FeatureSelection::default(),
        browser: None,
        work_report: None,
      },
    )
  }
}

/// Runs the legacy Ditto process with its temporary Reactant preparation rule.
pub fn run() -> u8 {
  INTERRUPTED.store(false, Ordering::SeqCst);
  if let Err(error) = ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::SeqCst)) {
    let _ = writeln!(
      io::stderr(),
      "error: failed to install interrupt handler: {error}"
    );
    return 2;
  }
  process_from_with_interrupt(
    std::iter::once(OsString::from("ditto")).chain(std::env::args_os().skip(1)),
    &mut io::stdout(),
    &mut io::stderr(),
    &INTERRUPTED,
  )
}

/// Runs legacy process-style arguments with an embedding process's interrupt flag.
pub fn process_from_with_interrupt<I, T>(
  arguments: I,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> u8
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  battlement_ditto::process_from_with_preparation(
    arguments,
    stdout,
    stderr,
    interrupted,
    Arc::new(LegacyReactantPreparation),
  )
}
