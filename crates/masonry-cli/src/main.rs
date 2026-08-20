mod plugin;
mod plugin_build;
mod sample;
mod tools;

use std::{ffi::OsString, path::PathBuf};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "cargo masonry", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage the native rules plugin in a macOS Unity application.
    Plugin(PluginArgs),
    /// Build and run standalone Masonry samples.
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
        /// Build a non-Development release player.
        #[arg(long)]
        release: bool,
    },
    /// Build and open a standalone sample player.
    Run {
        /// Directory name below samples/.
        name: String,
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
        /// Path to a prebuilt libmasonry_rules.dylib.
        #[arg(required_unless_present = "package", conflicts_with = "package")]
        library: Option<PathBuf>,
        /// Cargo package that builds the masonry_rules cdylib.
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
    /// Validate a Masonry native plugin without installing it.
    Verify {
        /// Path to libmasonry_rules.dylib.
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
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse_from(cargo_subcommand_args());
    match cli.command {
        Command::Plugin(args) => match args.command {
            PluginCommand::Inspect { app } => plugin::inspect(&app),
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
                    plugin::install(&app, &library, identity)
                } else {
                    plugin::build_and_install(
                        &app,
                        package.as_deref().expect("clap requires a package"),
                        release,
                        manifest_path.as_deref(),
                        identity,
                    )
                }
            }
            PluginCommand::Restore { app, signing } => {
                plugin::restore(&app, signing_identity(&signing))
            }
            PluginCommand::Verify { library } => plugin::verify(&library).map(|_| ()),
        },
        Command::Sample(args) => match args.command {
            SampleCommand::Build { name, release } => sample::build(&name, release).map(|_| ()),
            SampleCommand::Run { name, release } => sample::run(&name, release),
        },
    }
}

fn cargo_subcommand_args() -> Vec<OsString> {
    let mut args: Vec<OsString> = std::env::args_os().collect();
    if args.get(1).is_some_and(|argument| argument == "masonry") {
        args.remove(1);
    }
    args
}

fn signing_identity(args: &SigningArgs) -> Option<&str> {
    (!args.no_sign).then_some(args.sign.as_str())
}
