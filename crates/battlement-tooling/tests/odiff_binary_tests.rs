use std::{fs, path::Path};

use anyhow::Result;
use battlement_tooling::odiff_binary::{self, ToolDownloader};

struct BytesDownloader(&'static [u8]);

impl ToolDownloader for BytesDownloader {
  fn download(&self, _url: &str, destination: &Path) -> Result<()> {
    fs::write(destination, self.0)?;
    Ok(())
  }
}

#[test]
fn explicit_override_is_resolved_without_download() {
  let temporary = tempfile::tempdir().unwrap();
  let binary = temporary.path().join("odiff-dev");
  fs::write(&binary, b"development override").unwrap();

  let resolved = odiff_binary::resolve(
    &temporary.path().join("tools"),
    Some(&binary),
    "arm64",
    &BytesDownloader(b"unused"),
  )
  .unwrap();

  assert_eq!(resolved, binary.canonicalize().unwrap());
  assert!(!temporary.path().join("tools").exists());
}

#[test]
fn wrong_download_digest_never_publishes_a_binary() {
  let temporary = tempfile::tempdir().unwrap();
  let tools = temporary.path().join("tools");

  let error = odiff_binary::resolve(
    &tools,
    None,
    "aarch64",
    &BytesDownloader(b"not the official binary"),
  )
  .unwrap_err();

  assert!(error.to_string().contains("digest mismatch"));
  let directory = tools.join("odiff/4.5.0");
  assert!(!directory.join("odiff-macos-arm64").exists());
  assert_eq!(fs::read_dir(directory).unwrap().count(), 0);
}

#[test]
fn unsupported_architecture_is_rejected_before_download() {
  let temporary = tempfile::tempdir().unwrap();
  let error = odiff_binary::resolve(
    temporary.path(),
    None,
    "unsupported",
    &BytesDownloader(b"unused"),
  )
  .unwrap_err();
  assert!(error.to_string().contains("no pinned macOS binary"));
}
