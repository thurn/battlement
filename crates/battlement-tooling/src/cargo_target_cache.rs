use std::{
  fs::File,
  path::{Path, PathBuf},
};

use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::{build_cache_io, build_identity::BuildIdentity};

pub(crate) struct CargoTargetCache {
  path: PathBuf,
  _lock: File,
}

impl CargoTargetCache {
  /// Serializes compilation and artifact copying within one worktree's Cargo cache.
  pub(crate) fn acquire(
    repository: &Path,
    manifest: &Path,
    identity: &BuildIdentity,
  ) -> Result<Self> {
    let inputs = identity
      .inputs
      .iter()
      .filter(|input| input.name != "source")
      .collect::<Vec<_>>();
    let key = Sha256::digest(serde_json::to_vec(&(manifest.canonicalize()?, inputs))?);
    let path = repository
      .join("target/ditto-rules")
      .join(format!("{key:x}"));
    let lock = build_cache_io::lock_exclusive(&path.with_extension("lock"))?;
    Ok(Self { path, _lock: lock })
  }

  pub(crate) fn path(&self) -> &Path {
    &self.path
  }
}
