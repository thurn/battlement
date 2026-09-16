use std::path::PathBuf;

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Generate typed constants from the project's Addressables entries.
  Generate(Selection),
  /// Check that the typed constants are current.
  Check(Selection),
}

#[derive(Debug, ClapArgs)]
struct Selection {
  /// Explicit Unity project directory.
  #[arg(long)]
  project: PathBuf,
  /// Generated module file. Relative paths resolve from the current directory.
  #[arg(long)]
  output: Option<PathBuf>,
}

pub(crate) fn run(args: Args) -> Result<()> {
  let (selection, check) = match args.command {
    Command::Generate(selection) => (selection, false),
    Command::Check(selection) => (selection, true),
  };
  battlement_tooling::addressables::run(
    Some(&selection.project),
    selection.output.as_deref(),
    check,
  )
}
