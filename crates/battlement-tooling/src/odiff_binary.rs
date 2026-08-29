//! Installation of the pinned ODiff macOS executable.

use std::{
  fs,
  path::{Path, PathBuf},
  process::Command,
};

use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};

const VERSION: &str = "4.5.0";
const ARM64_SHA256: &str = "3c681171c158f95e7e62d636ddd00c33e8f971c23c85239c6192b72d76ad665b";
const X64_SHA256: &str = "73e565e2a777b653fa0ceb90c138dec1c396c990913fdc1221fe8b01fa70c171";

/// Downloads one URL into an exact destination path.
pub trait ToolDownloader {
  fn download(&self, url: &str, destination: &Path) -> Result<()>;
}

/// Downloader backed by the host's curl executable.
pub struct CurlDownloader;

impl ToolDownloader for CurlDownloader {
  fn download(&self, url: &str, destination: &Path) -> Result<()> {
    let status = Command::new("curl")
      .args([
        "--fail",
        "--location",
        "--silent",
        "--show-error",
        "--output",
      ])
      .arg(destination)
      .arg(url)
      .status()
      .context("launch curl for ODiff")?;
    ensure!(status.success(), "ODiff download failed with {status}");
    Ok(())
  }
}

/// Resolves an explicit development override or installs the verified official binary.
pub fn resolve(
  tools_directory: &Path,
  override_path: Option<&Path>,
  architecture: &str,
  downloader: &dyn ToolDownloader,
) -> Result<PathBuf> {
  if let Some(path) = override_path {
    ensure!(path.is_file(), "ODiff override is not a file");
    return path.canonicalize().context("resolve ODiff override");
  }
  let (asset, expected) = pinned_asset(architecture)?;
  let directory = tools_directory.join("odiff").join(VERSION);
  let destination = directory.join(asset);
  if destination.is_file() && sha256(&destination)? == expected {
    return Ok(destination);
  }
  fs::create_dir_all(&directory).context("create ODiff tool cache")?;
  let temporary = directory.join(format!(".{asset}.download-{}", std::process::id()));
  let url = format!("https://github.com/dmtrKovalenko/odiff/releases/download/v{VERSION}/{asset}");
  let result = (|| {
    downloader.download(&url, &temporary)?;
    ensure!(
      sha256(&temporary)? == expected,
      "downloaded ODiff digest mismatch"
    );
    make_executable(&temporary)?;
    fs::rename(&temporary, &destination).context("publish verified ODiff binary")?;
    Ok(destination)
  })();
  if temporary.exists() {
    let _ = fs::remove_file(&temporary);
  }
  result
}

fn pinned_asset(architecture: &str) -> Result<(&'static str, &'static str)> {
  match architecture {
    "aarch64" | "arm64" => Ok(("odiff-macos-arm64", ARM64_SHA256)),
    "x86_64" | "x64" => Ok(("odiff-macos-x64", X64_SHA256)),
    _ => bail!("ODiff has no pinned macOS binary for {architecture}"),
  }
}

fn sha256(path: &Path) -> Result<String> {
  Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
  use std::os::unix::fs::PermissionsExt;

  let mut permissions = fs::metadata(path)?.permissions();
  permissions.set_mode(0o755);
  fs::set_permissions(path, permissions)?;
  Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
  Ok(())
}
