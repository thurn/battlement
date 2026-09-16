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
    /// Cargo manifest used to locate the plugin package.
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

#[derive(Debug, ClapArgs)]
struct SigningArgs {
  /// Code-signing identity. The default uses an ad-hoc signature.
  #[arg(long, default_value = "-", conflicts_with = "no_sign")]
  sign: String,
  /// Leave the modified application unsigned for a later signing step.
  #[arg(long)]
  no_sign: bool,
}

pub(crate) fn run(args: Args) -> Result<()> {
  match args.command {
    Command::Inspect { app } => battlement_tooling::plugin::inspect(&app),
    Command::Install {
      app,
      library,
      package,
      release,
      manifest_path,
      signing,
    } => {
      let identity = signing_identity(&signing);
      if let Some(library) = library {
        battlement_tooling::plugin::install(&app, &library, identity)
      } else {
        battlement_tooling::plugin::build_and_install(
          &app,
          package.as_deref().expect("clap requires a package"),
          release,
          manifest_path.as_deref(),
          identity,
        )
      }
    }
    Command::Restore { app, signing } => {
      battlement_tooling::plugin::restore(&app, signing_identity(&signing))
    }
    Command::Verify { library } => battlement_tooling::plugin::verify(&library).map(|_| ()),
  }
}

fn signing_identity(args: &SigningArgs) -> Option<&str> {
  (!args.no_sign).then_some(args.sign.as_str())
}
