use std::{fs, io::Write, path::Path};

use anyhow::{Context, Result};
use battlement_tooling::build_identity::BuildIdentity;

use crate::{cli::BuildOptions, config::model::Suite};

pub(crate) struct Failure<'a> {
  pub identity: &'a BuildIdentity,
  pub phase: &'a str,
  pub error_ids: &'a [String],
  pub message: &'a str,
  pub log_path: &'a Path,
}

pub(crate) fn report_failure(
  suite: &Suite,
  profile: &str,
  options: &BuildOptions,
  failure: Failure<'_>,
  stdout: &mut dyn Write,
) -> Result<u8> {
  let value = serde_json::json!({
    "schema": 1,
    "suite": suite.name,
    "profile": profile,
    "source_fingerprint": failure.identity.source_fingerprint,
    "build_fingerprint": failure.identity.fingerprint,
    "disposition": "failed",
    "phase": failure.phase,
    "error_ids": failure.error_ids,
    "message": failure.message,
    "log_path": failure.log_path,
  });
  let encoded = serde_json::to_string_pretty(&value)? + "\n";
  if let Some(path) = &options.output {
    fs::write(path, &encoded)
      .with_context(|| format!("write failed build result {}", path.display()))?;
  }
  if options.json {
    write!(stdout, "{encoded}")?;
    stdout.flush()?;
  }
  anyhow::bail!(
    "{}\nBuild phase: {}\nBuild fingerprint: {}\nBuild log: {}",
    failure.message,
    failure.phase,
    failure.identity.fingerprint,
    failure.log_path.display(),
  )
}
