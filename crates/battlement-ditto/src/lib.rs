//! Battlement Ditto command-line implementation.

pub mod baseline_manifest;
pub mod baseline_publication;
pub mod baseline_store;
pub mod baseline_update;
pub mod cli;
pub mod config;
pub mod filesystem_publication_store;
pub mod image_comparison;
pub mod macos_capture;
pub mod player_supervision;
pub mod r2_baseline_store;
pub mod r2_publication_store;
pub mod scenario_orchestration;
pub mod selection;
pub mod session_server;
pub mod suite;
pub mod wire;

mod command_execution;
mod crash_reconstruction;
mod crash_scenario;
mod execution_artifacts;
mod execution_materializer;
mod job_resolution;
mod macos_run;
mod maintenance_commands;
mod run_commands;
mod run_progress;
mod session_mutations;
mod storage_commands;

use std::{
  ffi::OsString,
  io::{self, Write},
  sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Result, ensure};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Runs Ditto with the supplied process-style arguments.
pub fn run_from<I, T>(arguments: I) -> Result<()>
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  let invocation = cli::parse_from(arguments)?;
  let code = command_execution::execute(
    invocation,
    &mut io::stdout(),
    &mut io::stderr(),
    &INTERRUPTED,
  )?;
  ensure!(code == 0, "Ditto command exited with status {code}");
  Ok(())
}

/// Runs Ditto using the current process arguments.
pub fn run() -> u8 {
  INTERRUPTED.store(false, Ordering::SeqCst);
  if let Err(error) = ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::SeqCst)) {
    let _ = writeln!(
      io::stderr(),
      "error: failed to install interrupt handler: {error}"
    );
    return 2;
  }
  process_from(std::env::args_os(), &mut io::stdout(), &mut io::stderr())
}

/// Runs Ditto with process-style arguments and explicit output streams.
pub fn process_from<I, T>(arguments: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  process_from_with_interrupt(arguments, stdout, stderr, &INTERRUPTED)
}

/// Runs Ditto with an interrupt flag owned by the embedding process.
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
  let invocation = match cli::parse_from(arguments) {
    Ok(invocation) => invocation,
    Err(error) => {
      let code = if error.use_stderr() { 2 } else { 0 };
      let _ = if error.use_stderr() {
        write!(stderr, "{error}")
      } else {
        write!(stdout, "{error}")
      };
      return code;
    }
  };
  match command_execution::execute(invocation, stdout, stderr, interrupted) {
    Ok(code) => {
      assert!(matches!(code, 0 | 1 | 2 | 130), "invalid Ditto exit code");
      code
    }
    Err(error) => {
      let _ = writeln!(stderr, "error: {error:#}");
      2
    }
  }
}
