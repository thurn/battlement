//! Battlement Ditto command-line implementation.

mod suite;

use std::{ffi::OsString, path::PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};

pub use suite::{Display, ListedProfile, ListedScenario, ListedSuite, Target};

#[derive(Debug, Parser)]
#[command(name = "ditto", version, about)]
struct Cli {
  /// Suite file. By default Ditto searches upward for ditto.toml.
  #[arg(long, global = true)]
  config: Option<PathBuf>,
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// List profiles, scenarios, and screenshot checkpoints.
  List,
}

/// Runs Ditto with the supplied process-style arguments.
pub fn run_from<I, T>(arguments: I) -> Result<()>
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  let cli = Cli::try_parse_from(arguments)?;
  match cli.command {
    Command::List => println!("{}", suite::load(cli.config.as_deref())?),
  }
  Ok(())
}

/// Runs Ditto using the current process arguments.
pub fn run() -> Result<()> {
  run_from(std::env::args_os())
}
