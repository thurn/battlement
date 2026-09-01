mod author;
mod generate;
mod plugin;
mod plugin_build;
mod reactant_assets;
mod sample;
mod tools;

use std::{
  ffi::OsString,
  path::PathBuf,
  sync::atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Parser)]
#[command(name = "cargo battlement", version, about)]
struct Cli {
  #[command(subcommand)]
  command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
  /// Open a Battlement Unity game for authoring and enter Play mode.
  Author {
    /// Unity project directory. Defaults to the current directory.
    #[arg(long, default_value = ".")]
    project: PathBuf,
    /// Cargo manifest for the rules plugin. Defaults to rules/Cargo.toml.
    #[arg(long)]
    manifest_path: Option<PathBuf>,
    /// Bootstrap scene below the Unity project. Auto-detected when omitted.
    #[arg(long)]
    scene: Option<PathBuf>,
    /// Build the Rust rules plugin with the release profile.
    #[arg(long)]
    release: bool,
  },
  /// Run Battlement Ditto screenshot testing and visual review.
  #[command(disable_help_flag = true)]
  Ditto {
    /// Arguments passed to Ditto.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    arguments: Vec<OsString>,
  },
  /// Generate typed Rust constants for a Unity project's Addressables entries.
  Generate {
    /// Unity project directory. The default searches from the current directory.
    project: Option<PathBuf>,
    /// Generated module file. Relative paths resolve from the current directory.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Verify that the generated module is current without rewriting it.
    #[arg(long)]
    check: bool,
  },
  /// Manage the native rules plugin in a macOS Unity application.
  Plugin(PluginArgs),
  /// Work with Reactant projects.
  Reactant(ReactantArgs),
  /// Build and run standalone Battlement samples.
  Sample(SampleArgs),
}

#[derive(Debug, Args)]
struct SampleArgs {
  #[command(subcommand)]
  command: SampleCommand,
}

#[derive(Debug, Subcommand)]
enum SampleCommand {
  /// Build a standalone sample player.
  Build {
    /// Directory name below samples/.
    name: String,
    /// Build a browser player with the Rust engine embedded in WebAssembly.
    #[arg(long)]
    web: bool,
    /// Build a non-Development release player.
    #[arg(long)]
    release: bool,
  },
  /// Build and run a standalone sample player with terminal logging.
  Run {
    /// Directory name below samples/.
    name: String,
    /// Run a browser player with the Rust engine embedded in WebAssembly.
    #[arg(long)]
    web: bool,
    /// Local static-server port for a Web player.
    #[arg(long, requires = "web")]
    port: Option<u16>,
    /// Build a non-Development release player.
    #[arg(long)]
    release: bool,
  },
}

#[derive(Debug, Args)]
struct PluginArgs {
  #[command(subcommand)]
  command: PluginCommand,
}

#[derive(Debug, Args)]
struct ReactantArgs {
  #[command(subcommand)]
  command: ReactantCommand,
}

#[derive(Debug, Subcommand)]
enum ReactantCommand {
  /// Manage generated Reactant assets.
  Assets(reactant_assets::Args),
}

#[derive(Debug, Subcommand)]
enum PluginCommand {
  /// Inspect the plugin installed in a Unity application.
  Inspect {
    /// Path to the built Unity .app bundle.
    app: PathBuf,
  },
  /// Validate and install a plugin in a Unity application.
  Install {
    /// Path to the built Unity .app bundle.
    app: PathBuf,
    /// Path to a prebuilt libbattlement_rules.dylib.
    #[arg(required_unless_present = "package", conflicts_with = "package")]
    library: Option<PathBuf>,
    /// Cargo package that builds the battlement_rules cdylib.
    #[arg(long, conflicts_with = "library")]
    package: Option<String>,
    /// Build the Cargo package with the release profile.
    #[arg(long, requires = "package")]
    release: bool,
    /// Cargo manifest used to locate the rules package.
    #[arg(long, requires = "package")]
    manifest_path: Option<PathBuf>,
    #[command(flatten)]
    signing: SigningArgs,
  },
  /// Restore the plugin saved by the first install operation.
  Restore {
    /// Path to the built Unity .app bundle.
    app: PathBuf,
    #[command(flatten)]
    signing: SigningArgs,
  },
  /// Validate a Battlement native plugin without installing it.
  Verify {
    /// Path to libbattlement_rules.dylib.
    library: PathBuf,
  },
}

#[derive(Debug, Args)]
struct SigningArgs {
  /// Code-signing identity. The default uses an ad-hoc signature.
  #[arg(long, default_value = "-", conflicts_with = "no_sign")]
  sign: String,
  /// Leave the modified application unsigned for a later signing step.
  #[arg(long)]
  no_sign: bool,
}

fn main() {
  match run() {
    Ok(0) => {}
    Ok(code) => std::process::exit(code.into()),
    Err(error) => {
      eprintln!("error: {error:#}");
      std::process::exit(1);
    }
  }
}

fn reset_interrupted() {
  INTERRUPTED.store(false, Ordering::SeqCst);
}

fn interrupted() -> bool {
  INTERRUPTED.load(Ordering::SeqCst)
}

fn install_interrupt_handler() -> Result<()> {
  ctrlc::set_handler(|| INTERRUPTED.store(true, Ordering::SeqCst))
    .map_err(|error| anyhow::anyhow!("failed to install interrupt handler: {error}"))
}

fn run() -> Result<u8> {
  let cli = Cli::parse_from(cargo_subcommand_args());
  let code = match cli.command {
    Command::Author {
      project,
      manifest_path,
      scene,
      release,
    } => {
      install_interrupt_handler()?;
      author::run(
        &project,
        manifest_path.as_deref(),
        scene.as_deref(),
        release,
      )?;
      0
    }
    Command::Ditto { arguments } => {
      install_interrupt_handler()?;
      battlement_ditto::process_from_with_interrupt(
        std::iter::once(OsString::from("ditto")).chain(arguments),
        &mut std::io::stdout(),
        &mut std::io::stderr(),
        &INTERRUPTED,
      )
    }
    Command::Generate {
      project,
      output,
      check,
    } => {
      generate::run(project.as_deref(), output.as_deref(), check)?;
      0
    }
    Command::Plugin(args) => match args.command {
      PluginCommand::Inspect { app } => {
        plugin::inspect(&app)?;
        0
      }
      PluginCommand::Install {
        app,
        library,
        package,
        release,
        manifest_path,
        signing,
      } => {
        let identity = signing_identity(&signing);
        if let Some(library) = library {
          plugin::install(&app, &library, identity)?;
        } else {
          plugin::build_and_install(
            &app,
            package.as_deref().expect("clap requires a package"),
            release,
            manifest_path.as_deref(),
            identity,
          )?;
        }
        0
      }
      PluginCommand::Restore { app, signing } => {
        plugin::restore(&app, signing_identity(&signing))?;
        0
      }
      PluginCommand::Verify { library } => {
        plugin::verify(&library)?;
        0
      }
    },
    Command::Reactant(args) => match args.command {
      ReactantCommand::Assets(args) => {
        reactant_assets::run(args)?;
        0
      }
    },
    Command::Sample(args) => {
      install_interrupt_handler()?;
      match args.command {
        SampleCommand::Build { name, web, release } => {
          sample::build(&name, web, release)?;
          0
        }
        SampleCommand::Run {
          name,
          web,
          port,
          release,
        } => {
          sample::run(&name, web, port, release)?;
          0
        }
      }
    }
  };
  Ok(code)
}

fn cargo_subcommand_args() -> Vec<OsString> {
  let mut args: Vec<OsString> = std::env::args_os().collect();
  if args.get(1).is_some_and(|argument| argument == "battlement") {
    args.remove(1);
  }
  args
}

fn signing_identity(args: &SigningArgs) -> Option<&str> {
  (!args.no_sign).then_some(args.sign.as_str())
}
