//! Shared parsing for direct and Cargo-delegated Ditto commands.

use std::{ffi::OsString, path::PathBuf};

use clap::{Args, Parser, Subcommand};

/// One parsed Ditto invocation, independent of its process entry point.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Invocation {
  pub config: Option<PathBuf>,
  pub command: Command,
}

/// A complete publicly available command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
  Build(BuildOptions),
  Run(RunOptions),
  Capture(CaptureOptions),
  Review(ReviewOptions),
  Fetch(FetchOptions),
  List(SelectionOptions),
  Doctor(DoctorOptions),
  Clean(CleanCommand),
  Storage(StorageCommand),
}

/// Options for preparing an immutable player without executing scenarios.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildOptions {
  pub profile: Option<String>,
  pub json: bool,
  pub output: Option<PathBuf>,
}

/// Scenario and profile selectors shared by execution and inspection commands.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectionOptions {
  pub includes: Vec<String>,
  pub excludes: Vec<String>,
  pub profile: Option<String>,
  pub allow_empty: bool,
}

/// Options for a baseline-comparing run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunOptions {
  pub selection: SelectionOptions,
  pub update: bool,
  pub bail_after: Option<u32>,
  pub no_build: bool,
  pub json: bool,
  pub output: Option<PathBuf>,
  pub review: bool,
  pub watch: bool,
}

/// Options for a baseline-neutral capture.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureOptions {
  pub selection: SelectionOptions,
  pub fragment: Option<PathBuf>,
  pub bail_after: Option<u32>,
  pub no_build: bool,
  pub json: bool,
  pub output: Option<PathBuf>,
  pub review: bool,
  pub watch: bool,
}

/// Options for opening one retained run in the local review application.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewOptions {
  pub run: Option<String>,
}

/// Options for explicit baseline hydration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FetchOptions {
  pub selection: SelectionOptions,
  pub all: bool,
}

/// Options for categorized host diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorOptions {
  pub profile: Option<String>,
}

/// One local or remote cleanup operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CleanCommand {
  Runs { global: bool },
  Builds { global: bool },
  Baselines,
  Storage { apply: bool },
}

/// One canonical store-state operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageCommand {
  Publish,
}

#[derive(Debug, Parser)]
#[command(name = "ditto", version, about)]
struct Cli {
  /// Suite file. By default Ditto searches upward for ditto.toml.
  #[arg(long, global = true)]
  config: Option<PathBuf>,
  #[command(subcommand)]
  command: ParsedCommand,
}

#[derive(Debug, Subcommand)]
enum ParsedCommand {
  /// Build or reuse the exact immutable player selected by the suite.
  Build(BuildArgs),
  /// Execute scenarios and compare reached screenshots.
  Run(RunArgs),
  /// Execute scenarios without reading or changing baselines.
  Capture(CaptureArgs),
  /// Open a retained run in the local review application.
  Review(ReviewArgs),
  /// Download selected baseline objects into the local cache.
  Fetch(FetchArgs),
  /// List profiles, scenarios, and screenshot checkpoints.
  List(SelectionArgs),
  /// Check host tools, caches, and selected platform support.
  Doctor(DoctorArgs),
  /// Remove scoped inactive local data or retained store objects.
  Clean(CleanArgs),
  /// Manage canonical baseline-store state.
  Storage(StorageArgs),
}

#[derive(Debug, Args)]
struct BuildArgs {
  /// Profile to build instead of the suite default.
  #[arg(long)]
  profile: Option<String>,
  /// Write only the build result object to standard output.
  #[arg(long)]
  json: bool,
  /// Copy the build result to this path.
  #[arg(long)]
  output: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct RunArgs {
  #[command(flatten)]
  selection: SelectionArgs,
  /// Accept every reached screenshot checkpoint.
  #[arg(short = 'u', long)]
  update: bool,
  /// Stop after one or the supplied number of failed scenarios.
  #[arg(long, num_args = 0..=1, default_missing_value = "1", require_equals = true, value_parser = clap::value_parser!(u32).range(1..))]
  bail: Option<u32>,
  /// Require an exact cached build instead of compiling.
  #[arg(long)]
  no_build: bool,
  /// Write only the terminal result object to standard output.
  #[arg(long)]
  json: bool,
  /// Copy the terminal result to this path.
  #[arg(long)]
  output: Option<PathBuf>,
  /// Open the retained result in the local review application.
  #[arg(long)]
  review: bool,
  /// Keep the player and one live review tab warm across changes.
  #[arg(short = 'w', long, conflicts_with = "update")]
  watch: bool,
}

#[derive(Debug, Args)]
struct CaptureArgs {
  #[command(flatten)]
  selection: SelectionArgs,
  /// Full suite or fragment path; `-` reads standard input.
  #[arg(long)]
  fragment: Option<PathBuf>,
  /// Stop after one or the supplied number of failed scenarios.
  #[arg(long, num_args = 0..=1, default_missing_value = "1", require_equals = true, value_parser = clap::value_parser!(u32).range(1..))]
  bail: Option<u32>,
  /// Require an exact cached build instead of compiling.
  #[arg(long)]
  no_build: bool,
  /// Write only the terminal result object to standard output.
  #[arg(long)]
  json: bool,
  /// Copy the terminal result to this path.
  #[arg(long)]
  output: Option<PathBuf>,
  /// Open the retained result in the local review application.
  #[arg(long)]
  review: bool,
  /// Keep the player and one live review tab warm across changes.
  #[arg(short = 'w', long)]
  watch: bool,
}

#[derive(Debug, Args)]
struct ReviewArgs {
  /// Retained run ID. The newest reviewable run is selected when omitted.
  run: Option<String>,
}

#[derive(Debug, Args)]
struct FetchArgs {
  #[command(flatten)]
  selection: SelectionArgs,
  /// Hydrate every object named by ditto.lock.
  #[arg(
    long,
    conflicts_with_all = ["filters", "scenarios", "exclude", "profile", "allow_empty"]
  )]
  all: bool,
}

#[derive(Debug, Args)]
struct SelectionArgs {
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

#[derive(Debug, Args)]
struct DoctorArgs {
  /// Profile whose platform dependencies should be checked.
  #[arg(long)]
  profile: Option<String>,
}

#[derive(Debug, Args)]
struct CleanArgs {
  #[command(subcommand)]
  command: ParsedCleanCommand,
}

#[derive(Debug, Subcommand)]
enum ParsedCleanCommand {
  /// Remove inactive retained runs.
  Runs(GlobalArgs),
  /// Remove inactive immutable builds.
  Builds(GlobalArgs),
  /// Remove hydrated objects for the configured namespace.
  Baselines,
  /// Plan or apply retained remote object deletion.
  Storage {
    /// Apply the printed deletion plan.
    #[arg(long)]
    apply: bool,
  },
}

#[derive(Debug, Args)]
struct GlobalArgs {
  /// Include every inactive repository and suite.
  #[arg(long)]
  global: bool,
}

#[derive(Debug, Args)]
struct StorageArgs {
  #[command(subcommand)]
  command: ParsedStorageCommand,
}

#[derive(Debug, Subcommand)]
enum ParsedStorageCommand {
  /// Publish ditto.lock as the canonical live object set.
  Publish,
}

/// Parses one process-style argument sequence without performing side effects.
pub fn parse_from<I, T>(arguments: I) -> Result<Invocation, clap::Error>
where
  I: IntoIterator<Item = T>,
  T: Into<OsString> + Clone,
{
  let parsed = Cli::try_parse_from(arguments)?;
  Ok(Invocation {
    config: parsed.config,
    command: command(parsed.command),
  })
}

fn command(command: ParsedCommand) -> Command {
  match command {
    ParsedCommand::Build(args) => Command::Build(BuildOptions {
      profile: args.profile,
      json: args.json,
      output: args.output,
    }),
    ParsedCommand::Run(args) => Command::Run(RunOptions {
      selection: selection(args.selection),
      update: args.update,
      bail_after: args.bail,
      no_build: args.no_build,
      json: args.json,
      output: args.output,
      review: args.review,
      watch: args.watch,
    }),
    ParsedCommand::Capture(args) => Command::Capture(CaptureOptions {
      selection: selection(args.selection),
      fragment: args.fragment,
      bail_after: args.bail,
      no_build: args.no_build,
      json: args.json,
      output: args.output,
      review: args.review,
      watch: args.watch,
    }),
    ParsedCommand::Review(args) => Command::Review(ReviewOptions { run: args.run }),
    ParsedCommand::Fetch(args) => Command::Fetch(FetchOptions {
      selection: selection(args.selection),
      all: args.all,
    }),
    ParsedCommand::List(args) => Command::List(selection(args)),
    ParsedCommand::Doctor(args) => Command::Doctor(DoctorOptions {
      profile: args.profile,
    }),
    ParsedCommand::Clean(args) => Command::Clean(match args.command {
      ParsedCleanCommand::Runs(args) => CleanCommand::Runs {
        global: args.global,
      },
      ParsedCleanCommand::Builds(args) => CleanCommand::Builds {
        global: args.global,
      },
      ParsedCleanCommand::Baselines => CleanCommand::Baselines,
      ParsedCleanCommand::Storage { apply } => CleanCommand::Storage { apply },
    }),
    ParsedCommand::Storage(args) => Command::Storage(match args.command {
      ParsedStorageCommand::Publish => StorageCommand::Publish,
    }),
  }
}

fn selection(args: SelectionArgs) -> SelectionOptions {
  SelectionOptions {
    includes: args.filters.into_iter().chain(args.scenarios).collect(),
    excludes: args.exclude,
    profile: args.profile,
    allow_empty: args.allow_empty,
  }
}
