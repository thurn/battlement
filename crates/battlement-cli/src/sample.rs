use std::{
  path::{Path, PathBuf},
  sync::atomic::Ordering,
};

use anyhow::{Context, Result, bail};

use crate::{INTERRUPTED, reactant_assets};

struct DittoBuilder;

impl battlement_tooling::project::ComponentizedMacosBuilder for DittoBuilder {
  fn build(&self, config: &Path, release: bool) -> Result<PathBuf> {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut arguments = vec![
      "ditto".into(),
      "--config".into(),
      config.as_os_str().to_owned(),
      "build".into(),
      "--json".into(),
    ];
    if !release {
      arguments.push("--debug-rules".into());
    }
    let code = battlement_ditto::process_from(arguments, &mut stdout, &mut stderr);
    if code != 0 {
      bail!(
        "componentized sample build failed: {}",
        String::from_utf8_lossy(&stderr).trim()
      );
    }
    let value: serde_json::Value =
      serde_json::from_slice(&stdout).context("Ditto build returned invalid JSON")?;
    value
      .get("application_path")
      .and_then(serde_json::Value::as_str)
      .map(PathBuf::from)
      .context("Ditto build omitted application_path")
  }
}

pub(crate) fn build(name: &str, web: bool, release: bool) -> Result<PathBuf> {
  INTERRUPTED.store(false, Ordering::SeqCst);
  battlement_tooling::project::build(
    name,
    web,
    release,
    &INTERRUPTED,
    reactant_assets::generate,
    &DittoBuilder,
  )
}

pub(crate) fn run(name: &str, web: bool, port: Option<u16>, release: bool) -> Result<()> {
  INTERRUPTED.store(false, Ordering::SeqCst);
  battlement_tooling::project::run(
    name,
    web,
    port,
    release,
    &INTERRUPTED,
    reactant_assets::generate,
    &DittoBuilder,
  )
}
