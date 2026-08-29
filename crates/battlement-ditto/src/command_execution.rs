use std::{io::Write, sync::atomic::AtomicBool};

use anyhow::Result;

use crate::{
  cli::{Command, Invocation},
  maintenance_commands, review_commands, run_commands, selection, storage_commands, suite,
};

pub(crate) fn execute(
  invocation: Invocation,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  match invocation.command {
    Command::List(options) => {
      writeln!(
        stdout,
        "{}",
        suite::load(
          invocation.config.as_deref(),
          selection::Options {
            profile: options.profile,
            includes: options.includes,
            excludes: options.excludes,
            allow_empty: options.allow_empty,
          },
        )?
      )?;
      Ok(0)
    }
    Command::Run(options) => run_commands::run(
      invocation.config.as_deref(),
      options,
      stdout,
      stderr,
      interrupted,
    ),
    Command::Capture(options) => run_commands::capture(
      invocation.config.as_deref(),
      options,
      stdout,
      stderr,
      interrupted,
    ),
    Command::Review(options) => {
      review_commands::review(invocation.config.as_deref(), options, stderr, interrupted)
    }
    Command::Fetch(options) => {
      storage_commands::fetch(invocation.config.as_deref(), options, stdout)
    }
    Command::Doctor(options) => {
      maintenance_commands::doctor(invocation.config.as_deref(), options, stdout)
    }
    Command::Clean(command) => {
      maintenance_commands::clean(invocation.config.as_deref(), command, stdout)
    }
    Command::Storage(command) => {
      storage_commands::storage(invocation.config.as_deref(), command, stdout)
    }
  }
}
