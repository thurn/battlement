//! Content-addressed baseline storage and local hydration.

use std::{
  fs,
  path::{Path, PathBuf},
  time::Duration,
};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
  baseline_manifest::{BaselineEntry, BaselineManifest, validate_namespace, validate_sha256},
  image_comparison::{ImageComparison, ImageComparisonRequest, OdiffServer},
  wire::job::Comparison,
};

/// Immutable baseline object operations shared by local and remote stores.
pub trait BaselineStore: Send + Sync {
  /// Makes one verified object available at the supplied cache root.
  fn hydrate(&self, namespace: &str, sha256: &str, cache_root: &Path) -> Result<PathBuf>;

  /// Publishes one verified PNG under its content digest.
  fn put(&self, namespace: &str, sha256: &str, source: &Path) -> Result<()>;
}

/// The baseline state for one checkpoint after it is actually reached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReachedBaseline {
  Missing,
  Hydrated { entry: BaselineEntry, path: PathBuf },
}

/// Inputs for comparing one screenshot checkpoint after execution reaches it.
pub struct ReachedComparisonRequest<'a> {
  pub profile: &'a str,
  pub scenario: &'a str,
  pub checkpoint: &'a str,
  pub actual: &'a Path,
  pub diff: &'a Path,
  pub settings: Comparison,
  pub timeout: Duration,
}

/// The ordinary-run visual result for one reached checkpoint.
#[derive(Debug, PartialEq)]
pub enum ReachedComparison {
  Missing,
  Compared {
    entry: BaselineEntry,
    baseline: PathBuf,
    comparison: Box<ImageComparison>,
  },
}

/// A filesystem-backed content-addressed baseline store.
#[derive(Clone, Debug)]
pub struct FilesystemBaselineStore {
  root: PathBuf,
}

impl FilesystemBaselineStore {
  /// Uses a store root that may be inside or outside the repository.
  pub fn new(root: PathBuf) -> Self {
    Self { root }
  }

  /// Returns the canonical object path for one namespace and digest.
  pub fn object_path(&self, namespace: &str, sha256: &str) -> Result<PathBuf> {
    object_path(&self.root, namespace, sha256)
  }
}

impl BaselineStore for FilesystemBaselineStore {
  fn hydrate(&self, namespace: &str, sha256: &str, cache_root: &Path) -> Result<PathBuf> {
    let source = self.object_path(namespace, sha256)?;
    verify_file(&source, sha256).context("verify filesystem baseline")?;
    let destination = object_path(cache_root, namespace, sha256)?;
    if destination == source {
      return Ok(destination);
    }
    if destination.exists() {
      verify_file(&destination, sha256).context("verify cached baseline")?;
      return Ok(destination);
    }
    write_atomic(&destination, &fs::read(source)?)?;
    verify_file(&destination, sha256).context("verify hydrated baseline")?;
    Ok(destination)
  }

  fn put(&self, namespace: &str, sha256: &str, source: &Path) -> Result<()> {
    verify_file(source, sha256).context("verify proposed baseline")?;
    let destination = self.object_path(namespace, sha256)?;
    if destination.exists() {
      return verify_file(&destination, sha256).context("verify existing baseline object");
    }
    write_atomic(&destination, &fs::read(source)?)?;
    verify_file(&destination, sha256).context("verify published baseline")
  }
}

/// Hydrates one reached checkpoint, without touching objects for any other checkpoint.
pub fn hydrate_reached(
  store: &dyn BaselineStore,
  manifest: Option<&BaselineManifest>,
  cache_root: &Path,
  profile: &str,
  scenario: &str,
  checkpoint: &str,
) -> Result<ReachedBaseline> {
  let Some(manifest) = manifest else {
    return Ok(ReachedBaseline::Missing);
  };
  let Some(entry) = manifest.find(profile, scenario, checkpoint) else {
    return Ok(ReachedBaseline::Missing);
  };
  Ok(ReachedBaseline::Hydrated {
    entry: entry.clone(),
    path: store.hydrate(&manifest.namespace, &entry.sha256, cache_root)?,
  })
}

/// Hydrates and compares exactly one reached checkpoint through the run's warm ODiff server.
pub fn compare_reached(
  store: &dyn BaselineStore,
  manifest: Option<&BaselineManifest>,
  cache_root: &Path,
  server: &mut OdiffServer,
  request: ReachedComparisonRequest<'_>,
) -> Result<ReachedComparison> {
  match hydrate_reached(
    store,
    manifest,
    cache_root,
    request.profile,
    request.scenario,
    request.checkpoint,
  )? {
    ReachedBaseline::Missing => Ok(ReachedComparison::Missing),
    ReachedBaseline::Hydrated { entry, path } => Ok(ReachedComparison::Compared {
      comparison: Box::new(server.compare(ImageComparisonRequest {
        baseline: &path,
        actual: request.actual,
        diff: request.diff,
        settings: request.settings,
        timeout: request.timeout,
      })?),
      entry,
      baseline: path,
    }),
  }
}

pub(crate) fn object_path(root: &Path, namespace: &str, sha256: &str) -> Result<PathBuf> {
  validate_namespace(namespace)?;
  validate_sha256("baseline sha256", sha256)?;
  Ok(
    root
      .join(namespace)
      .join("objects")
      .join(&sha256[..2])
      .join(format!("{sha256}.png")),
  )
}

pub(crate) fn verify_file(path: &Path, expected: &str) -> Result<()> {
  let bytes = fs::read(path).with_context(|| format!("read baseline object {}", path.display()))?;
  ensure!(
    format!("{:x}", Sha256::digest(&bytes)) == expected,
    "baseline object hash mismatch"
  );
  Ok(())
}

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
  let parent = path.parent().context("baseline path has no parent")?;
  fs::create_dir_all(parent)?;
  let temporary = parent.join(format!(
    ".{}.{}.tmp",
    path
      .file_name()
      .context("baseline path has no file name")?
      .to_string_lossy(),
    Uuid::new_v4()
  ));
  let result = (|| {
    fs::write(&temporary, bytes)?;
    fs::OpenOptions::new()
      .write(true)
      .open(&temporary)?
      .sync_all()?;
    fs::rename(&temporary, path)?;
    self::sync_directory(parent)?;
    Ok(())
  })();
  if result.is_err() {
    let _ = fs::remove_file(temporary);
  }
  result
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<()> {
  fs::File::open(path)?.sync_all()?;
  Ok(())
}

#[cfg(windows)]
fn sync_directory(_path: &Path) -> Result<()> {
  Ok(())
}
