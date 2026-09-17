use std::{
  ffi::OsString,
  path::PathBuf,
  sync::atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use battlement_ditto::coverage_ledger;
use battlement_reactant_assets::{AssetCommand, CommandOptions, FeatureSelection};
use battlement_tooling::application::BuildOptions;
use clap::{Args, Parser, Subcommand};

use crate::project::Overrides;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Parser)]
#[command(name = "rt", version, about = "Reactant project tooling")]
struct Cli {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Build a Reactant application and Unity player.
  Build(BuildArgs),
  /// Build and run a Reactant application.
  Run(RunArgs),
  /// Prepare a Reactant application and open it in Unity Play mode.
  Author(AuthorArgs),
  /// Discover, generate, check, or preview Reactant assets.
  Assets(crate::assets::Args),
  /// Generate or check typed Addressables constants.
  Addressables(crate::addressables::Args),
  /// Inspect, install, restore, or verify a native plugin.
  Plugin(crate::plugin::Args),
  /// Validate repository native states, checkpoints, and baseline coverage without building players.
  CheckNativeCoverage {
    /// Repository containing the samples coverage registry.
    #[arg(long, default_value = ".")]
    repository: PathBuf,
  },
  /// Run Battlement Ditto with an explicit suite configuration.
  #[command(disable_help_flag = true)]
  Ditto(DittoArgs),
}

#[derive(Debug, Args)]
struct DittoArgs {
  /// Explicit ditto.toml suite file.
  #[arg(long)]
  config: Option<PathBuf>,
  /// Arguments passed through to Ditto.
  #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
  arguments: Vec<OsString>,
}

#[derive(Debug, Args)]
struct ProjectArgs {
  /// Unity project root containing reactant.toml.
  #[arg(long, default_value = ".")]
  project: PathBuf,
  /// Application product name, overriding reactant.toml.
  #[arg(long)]
  application: Option<String>,
  /// Cargo manifest for the application plugin, overriding reactant.toml.
  #[arg(long)]
  manifest_path: Option<PathBuf>,
  /// Bootstrap scene, overriding reactant.toml.
  #[arg(long)]
  scene: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct BuildArgs {
  #[command(flatten)]
  project: ProjectArgs,
  /// Write the player to this path relative to the project root.
  #[arg(long)]
  output: Option<PathBuf>,
  /// Build the Rust application plugin with the release profile.
  #[arg(long)]
  release: bool,
  /// Build a browser player instead of a native player.
  #[arg(long)]
  web: bool,
  /// Build a direct Battlement project without Reactant asset preparation.
  #[arg(long)]
  skip_assets: bool,
}

#[derive(Debug, Args)]
struct RunArgs {
  #[command(flatten)]
  build: BuildArgs,
  /// Local static-server port for a Web player. Defaults to 8000.
  #[arg(long, requires = "web")]
  port: Option<u16>,
}

#[derive(Debug, Args)]
struct AuthorArgs {
  #[command(flatten)]
  project: ProjectArgs,
  /// Build the Rust application plugin with the release profile.
  #[arg(long)]
  release: bool,
  /// Author a direct Battlement project without Reactant asset preparation.
  #[arg(long)]
  skip_assets: bool,
}

/// Runs the `rt` process and reports failures to the terminal.
pub fn main() {
  match self::run() {
    Ok(0) => {}
    Ok(code) => std::process::exit(code.into()),
    Err(error) => {
      eprintln!("error: {error:#}");
      std::process::exit(1);
    }
  }
}

fn run() -> Result<u8> {
  INTERRUPTED.store(false, Ordering::SeqCst);
  ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::SeqCst))
    .map_err(|error| anyhow::anyhow!("failed to install interrupt handler: {error}"))?;
  match Cli::parse().command {
    Command::Build(args) => {
      let (project, options, prepare) = resolve_build(args)?;
      if prepare {
        battlement_tooling::application::build(&project, &options, &INTERRUPTED, prepare_assets)?;
      } else {
        battlement_tooling::application::build(&project, &options, &INTERRUPTED, |_, _| Ok(()))?;
      }
    }
    Command::Run(args) => {
      let port = args.port;
      let (project, options, prepare) = resolve_build(args.build)?;
      if prepare {
        battlement_tooling::application::run(
          &project,
          &options,
          port,
          &INTERRUPTED,
          prepare_assets,
        )?;
      } else {
        battlement_tooling::application::run(
          &project,
          &options,
          port,
          &INTERRUPTED,
          |_, _| Ok(()),
        )?;
      }
    }
    Command::Author(args) => {
      let project = resolve_project(args.project, args.skip_assets)?;
      if args.skip_assets {
        battlement_tooling::author::run(
          &project.root,
          Some(&project.manifest),
          Some(&project.scene),
          args.release,
          &INTERRUPTED,
          |_, _| Ok(()),
        )?;
      } else {
        battlement_tooling::author::run(
          &project.root,
          Some(&project.manifest),
          Some(&project.scene),
          args.release,
          &INTERRUPTED,
          prepare_assets,
        )?;
      }
    }
    Command::Assets(args) => crate::assets::run(args)?,
    Command::Addressables(args) => crate::addressables::run(args)?,
    Command::Plugin(args) => crate::plugin::run(args)?,
    Command::CheckNativeCoverage { repository } => {
      let report = coverage_ledger::check_repository(&repository)?;
      println!(
        "Native coverage checked for {} samples.",
        report.samples.len()
      );
    }
    Command::Ditto(args) => {
      return crate::ditto::run(args.config, args.arguments, &INTERRUPTED);
    }
  }
  Ok(0)
}

fn resolve_build(
  args: BuildArgs,
) -> Result<(battlement_tooling::application::Project, BuildOptions, bool)> {
  let prepare = !args.skip_assets;
  let project = resolve_project(args.project, args.skip_assets)?;
  let options = BuildOptions {
    output: crate::project::resolve_output(&project.root, args.output),
    release: args.release,
    web: args.web,
  };
  Ok((project, options, prepare))
}

fn resolve_project(
  args: ProjectArgs,
  allow_missing_configuration: bool,
) -> Result<battlement_tooling::application::Project> {
  crate::project::resolve(
    args.project,
    Overrides {
      application: args.application,
      manifest_path: args.manifest_path,
      scene: args.scene,
    },
    allow_missing_configuration,
  )
}

pub(crate) fn prepare_assets(project: &std::path::Path, manifest: &std::path::Path) -> Result<()> {
  battlement_reactant_assets::run_quiet(
    AssetCommand::Generate,
    &CommandOptions {
      project: Some(project.to_owned()),
      manifest_path: Some(manifest.to_owned()),
      feature_selection: FeatureSelection::default(),
      browser: None,
      work_report: None,
    },
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use clap::CommandFactory;

  #[test]
  fn help_exposes_only_path_driven_project_commands() {
    let command = Cli::command();
    let names = command
      .get_subcommands()
      .map(|command| command.get_name())
      .collect::<Vec<_>>();
    assert_eq!(
      names,
      [
        "build",
        "run",
        "author",
        "assets",
        "addressables",
        "plugin",
        "check-native-coverage",
        "ditto"
      ]
    );
    assert!(!names.contains(&"sample"));
    assert!(!names.contains(&"reactant"));
  }

  #[test]
  fn web_port_requires_web_mode() {
    assert!(Cli::try_parse_from(["rt", "run", "--port", "9000"]).is_err());
    assert!(Cli::try_parse_from(["rt", "run", "--web", "--port", "9000"]).is_ok());
  }

  #[test]
  fn support_commands_use_explicit_project_or_artifact_inputs() {
    assert!(Cli::try_parse_from(["rt", "assets", "check", "--project", "project"]).is_ok());
    assert!(Cli::try_parse_from(["rt", "addressables", "check"]).is_err());
    assert!(Cli::try_parse_from(["rt", "addressables", "check", "--project", "project"]).is_ok());
    assert!(Cli::try_parse_from(["rt", "plugin", "inspect", "Game.app"]).is_ok());
    assert!(
      Cli::try_parse_from([
        "rt",
        "ditto",
        "--config",
        "project/ditto.toml",
        "run",
        "smoke"
      ])
      .is_ok()
    );
  }
}
