use std::{
  path::PathBuf,
  sync::atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use battlement_reactant_assets::{AssetCommand, CommandOptions, FeatureSelection};
use battlement_tooling::application::BuildOptions;
use clap::{Args, Parser, Subcommand};

use crate::project::Overrides;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Parser)]
#[command(name = "rt", version, about = "Build and run Reactant applications")]
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
}

/// Runs the `rt` process and reports failures to the terminal.
pub fn main() {
  if let Err(error) = self::run() {
    eprintln!("error: {error:#}");
    std::process::exit(1);
  }
}

fn run() -> Result<()> {
  INTERRUPTED.store(false, Ordering::SeqCst);
  ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::SeqCst))
    .map_err(|error| anyhow::anyhow!("failed to install interrupt handler: {error}"))?;
  match Cli::parse().command {
    Command::Build(args) => {
      let (project, options) = resolve_build(args)?;
      battlement_tooling::application::build(&project, &options, &INTERRUPTED, prepare_assets)?;
    }
    Command::Run(args) => {
      let port = args.port;
      let (project, options) = resolve_build(args.build)?;
      battlement_tooling::application::run(&project, &options, port, &INTERRUPTED, prepare_assets)?;
    }
    Command::Author(args) => {
      let project = resolve_project(args.project)?;
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
  Ok(())
}

fn resolve_build(
  args: BuildArgs,
) -> Result<(battlement_tooling::application::Project, BuildOptions)> {
  let project = resolve_project(args.project)?;
  let options = BuildOptions {
    output: crate::project::resolve_output(&project.root, args.output),
    release: args.release,
    web: args.web,
  };
  Ok((project, options))
}

fn resolve_project(args: ProjectArgs) -> Result<battlement_tooling::application::Project> {
  crate::project::resolve(
    args.project,
    Overrides {
      application: args.application,
      manifest_path: args.manifest_path,
      scene: args.scene,
    },
  )
}

fn prepare_assets(project: &std::path::Path, manifest: &std::path::Path) -> Result<()> {
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
    assert_eq!(names, ["build", "run", "author"]);
  }

  #[test]
  fn web_port_requires_web_mode() {
    assert!(Cli::try_parse_from(["rt", "run", "--port", "9000"]).is_err());
    assert!(Cli::try_parse_from(["rt", "run", "--web", "--port", "9000"]).is_ok());
  }
}
