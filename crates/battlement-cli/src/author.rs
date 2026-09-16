use std::{path::Path, sync::atomic::Ordering};

use anyhow::Result;

use crate::{INTERRUPTED, reactant_assets};

pub(crate) fn run(
  project: &Path,
  manifest_path: Option<&Path>,
  scene: Option<&Path>,
  release: bool,
) -> Result<()> {
  INTERRUPTED.store(false, Ordering::SeqCst);
  battlement_tooling::author::run(
    project,
    manifest_path,
    scene,
    release,
    &INTERRUPTED,
    reactant_assets::generate,
  )
}
