//! Battlement Ditto command-line implementation.

pub mod config;
pub mod selection;
pub mod suite;

use std::{ffi::OsString, path::PathBuf};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

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
  List(ListArgs),
}

#[derive(Debug, Args)]
struct ListArgs {
  /// Scenario glob selectors.
  filters: Vec<String>,
  /// Additional scenario glob selector.
  #[arg(long = "scenario")]
  scenarios: Vec<String>,
  /// Scenario glob removed after inclusion.
  #[arg(long)]
  exclude: Vec<String>,
  /// Profile to resolve instead of the suite default.
  #[arg(long)]
  profile: Option<String>,
  /// Permit a selection containing no scenarios.
  #[arg(long)]
  allow_empty: bool,
}

/// Runs Ditto with the supplied process-style arguments.
pub fn run_from<I, T>(arguments: I) -> Result<()>
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  let cli = Cli::try_parse_from(arguments)?;
  match cli.command {
    Command::List(arguments) => println!(
      "{}",
      suite::load(
        cli.config.as_deref(),
        selection::Options {
          profile: arguments.profile,
          includes: arguments
            .filters
            .into_iter()
            .chain(arguments.scenarios)
            .collect(),
          excludes: arguments.exclude,
          allow_empty: arguments.allow_empty,
        },
      )?
    ),
  }
  Ok(())
}

/// Runs Ditto using the current process arguments.
pub fn run() -> Result<()> {
  run_from(std::env::args_os())
}
