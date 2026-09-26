use std::path::PathBuf;

use anyhow::Result;
use battlement_reactant_assets::{AssetCommand, CommandOptions, FeatureSelection};
use battlement_tooling::build_control::BuildControl;
use clap::{Args as ClapArgs, Subcommand};

use crate::project::Overrides;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Generate the exact declared Reactant asset set.
  Generate(Selection),
  /// Check generated Reactant assets without modifying the Unity project.
  Check(Selection),
  /// Generate and open a local Reactant asset gallery.
  Preview(Selection),
}

#[derive(Debug, ClapArgs)]
struct Selection {
  /// Unity project root containing reactant.toml.
  #[arg(long, default_value = ".")]
  project: PathBuf,
  /// Cargo manifest for the application plugin, overriding reactant.toml.
  #[arg(long)]
  manifest_path: Option<PathBuf>,
  /// Space- or comma-separated Cargo features to enable.
  #[arg(long, value_delimiter = ',')]
  features: Vec<String>,
  /// Enable all Cargo features.
  #[arg(long)]
  all_features: bool,
  /// Disable default Cargo features.
  #[arg(long)]
  no_default_features: bool,
  /// Chrome or Chromium executable to use for rendering.
  #[arg(long)]
  browser: Option<PathBuf>,
  /// Write aggregate command work as canonical JSON.
  #[arg(long)]
  work_report: Option<PathBuf>,
}

pub(crate) fn run(args: Args, control: BuildControl<'_>) -> Result<()> {
  let (command, selection) = match args.command {
    Command::Generate(selection) => (AssetCommand::Generate, selection),
    Command::Check(selection) => (AssetCommand::Check, selection),
    Command::Preview(selection) => (AssetCommand::Preview, selection),
  };
  let project = crate::project::resolve(
    selection.project,
    Overrides {
      manifest_path: selection.manifest_path,
      ..Overrides::default()
    },
    false,
  )?;
  battlement_reactant_assets::run(
    command,
    &CommandOptions {
      project: Some(project.root),
      manifest_path: Some(project.manifest),
      feature_selection: FeatureSelection {
        features: selection.features,
        all_features: selection.all_features,
        no_default_features: selection.no_default_features,
      },
      browser: selection.browser,
      work_report: selection.work_report,
    },
    control,
  )
}
