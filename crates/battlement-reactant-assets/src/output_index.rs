use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
  WorkReport,
  incremental::{FileFingerprint, fingerprint},
};

const GENERATED_ROOT: &str = "Assets/Generated/BattlementReactant";
const GENERATED_ROOT_META: &str = "Assets/Generated/BattlementReactant.meta";

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct OutputIndex {
  roots: Vec<OutputProbe>,
  directories: Vec<FileFingerprint>,
  files: BTreeMap<String, OutputRecord>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct OutputProbe {
  path: String,
  fingerprint: Option<FileFingerprint>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct OutputRecord {
  fingerprint: FileFingerprint,
  content_hash: String,
}

impl OutputIndex {
  pub(crate) fn refresh(&mut self, project: &Path, report: &mut WorkReport) -> Result<()> {
    if self.matches(report) {
      return Ok(());
    }
    let root = project.join(GENERATED_ROOT);
    let metadata = project.join(GENERATED_ROOT_META);
    self.roots = [&root, &metadata]
      .into_iter()
      .map(|path| OutputProbe {
        path: self::normalized(path),
        fingerprint: fingerprint(path, report),
      })
      .collect();
    self.directories.clear();
    self.files.clear();
    self::record_path(&root, self, report)?;
    self::record_path(&metadata, self, report)
  }

  pub(crate) fn matches(&self, report: &mut WorkReport) -> bool {
    if self.roots.len() != 2 {
      return false;
    }
    let roots_match = self
      .roots
      .iter()
      .all(|probe| fingerprint(Path::new(&probe.path), report) == probe.fingerprint);
    if !roots_match {
      return false;
    }
    let directories_match = self
      .directories
      .iter()
      .all(|retained| fingerprint(Path::new(&retained.path), report).as_ref() == Some(retained));
    if !directories_match {
      return false;
    }
    self.files.values().all(|record| {
      fingerprint(Path::new(&record.fingerprint.path), report).as_ref() == Some(&record.fingerprint)
    })
  }
}

fn record_path(path: &Path, index: &mut OutputIndex, report: &mut WorkReport) -> Result<()> {
  let metadata = match fs::symlink_metadata(path) {
    Ok(metadata) => metadata,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
    Err(error) => {
      return Err(error)
        .with_context(|| format!("failed to inspect generated output {}", path.display()));
    }
  };
  if metadata.file_type().is_symlink() {
    bail!(
      "generated output {} must not be a symbolic link",
      path.display()
    );
  }
  if metadata.is_dir() {
    let current = fingerprint(path, report).with_context(|| {
      format!(
        "failed to fingerprint generated directory {}",
        path.display()
      )
    })?;
    index.directories.push(current);
    let mut children = fs::read_dir(path)
      .with_context(|| format!("failed to read generated directory {}", path.display()))?
      .collect::<std::io::Result<Vec<_>>>()?;
    children.sort_by_key(fs::DirEntry::file_name);
    for child in children {
      self::record_path(&child.path(), index, report)?;
    }
    return Ok(());
  }
  if metadata.is_file() {
    let current = fingerprint(path, report)
      .with_context(|| format!("failed to fingerprint generated output {}", path.display()))?;
    let bytes = fs::read(path)
      .with_context(|| format!("failed to read generated output {}", path.display()))?;
    report.files_opened += 1;
    report.bytes_read += bytes.len() as u64;
    if path
      .extension()
      .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
      report.generated_png_opens += 1;
    }
    index.files.insert(
      self::normalized(path),
      OutputRecord {
        fingerprint: current,
        content_hash: self::hex(&Sha256::digest(bytes)),
      },
    );
  }
  Ok(())
}

fn normalized(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}

fn hex(bytes: &[u8]) -> String {
  const DIGITS: &[u8; 16] = b"0123456789abcdef";

  let mut output = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    output.push(char::from(DIGITS[usize::from(byte >> 4)]));
    output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
  }
  output
}
