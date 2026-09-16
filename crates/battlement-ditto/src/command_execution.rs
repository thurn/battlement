use std::{io::Write, sync::atomic::AtomicBool};

use anyhow::Result;

use crate::{
  cli::{Command, Invocation},
  config, gallery_commands, macos_run, maintenance_commands, profile_commands, review_commands,
  run_commands, selection, storage_commands, suite, webgl_run,
};

pub(crate) fn execute(
  invocation: Invocation,
  stdout: &mut dyn Write,
  stderr: &mut dyn Write,
  interrupted: &AtomicBool,
) -> Result<u8> {
  match invocation.command {
    Command::Build(options) => {
      let suite = config::load(invocation.config.as_deref())?;
      let profile_name = options.profile.as_deref().unwrap_or(&suite.default_profile);
      let target = suite
        .profiles
        .get(profile_name)
        .ok_or_else(|| anyhow::anyhow!("profile {profile_name:?} does not exist"))?
        .target();
      match target {
        crate::config::model::Target::Macos => macos_run::build(&suite, options, stdout),
        crate::config::model::Target::Webgl => webgl_run::build(&suite, options, stdout),
        crate::config::model::Target::IosSimulator => {
          anyhow::bail!("build does not support iOS Simulator profiles")
        }
      }
    }
    Command::List(options) => {
      writeln!(
        stdout,
        "{}",
        suite::load(
          invocation.config.as_deref(),
          selection::Options {
            profile: options.profile,
            includes: options.includes,
            excludes: options.excludes,
            allow_empty: options.allow_empty,
          },
        )?
      )?;
      Ok(0)
    }
    Command::Run(options) => run_commands::run(
      invocation.config.as_deref(),
      options,
      stdout,
      stderr,
      interrupted,
    ),
    Command::Capture(options) => run_commands::capture(
      invocation.config.as_deref(),
      options,
      stdout,
      stderr,
      interrupted,
    ),
    Command::Profile(options) => profile_commands::profile(
      invocation.config.as_deref(),
      options,
      stdout,
      stderr,
      interrupted,
    ),
    Command::Review(options) => {
      review_commands::review(invocation.config.as_deref(), options, stderr, interrupted)
    }
    Command::Gallery(options) => {
      gallery_commands::gallery(invocation.config.as_deref(), options, stderr, interrupted)
    }
    Command::Fetch(options) => {
      storage_commands::fetch(invocation.config.as_deref(), options, stdout)
    }
    Command::Doctor(options) => {
      maintenance_commands::doctor(invocation.config.as_deref(), options, stdout)
    }
    Command::Clean(command) => {
      maintenance_commands::clean(invocation.config.as_deref(), command, stdout)
    }
    Command::Storage(command) => {
      storage_commands::storage(invocation.config.as_deref(), command, stdout)
    }
  }
}
