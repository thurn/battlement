use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::Path,
};

use anyhow::{Context, Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};

use crate::{
  AssetCatalog, WorkReport,
  manifest_schema::{Manifest, Sidecar},
};

const GENERATED_ROOT: &str = "Assets/Generated/BattlementReactant";
const MANIFEST_PATH: &str = "Assets/Generated/BattlementReactant/manifest.json";
const RESOURCES_PATH: &str = "Assets/Generated/BattlementReactant/Resources";
const SIDECAR_NAME: &str = "BattlementReactantAssetCatalog.json";

pub(crate) struct GeneratedSet {
  pub(crate) directories: BTreeSet<String>,
  pub(crate) files: BTreeMap<String, Vec<u8>>,
}

pub(crate) fn validate_built_set(set: &GeneratedSet, catalog: &AssetCatalog) -> Result<()> {
  let expected = self::base_paths(catalog);
  let actual = set
    .files
    .keys()
    .filter_map(|path| {
      path
        .strip_prefix(&format!("{GENERATED_ROOT}/"))
        .map(str::to_owned)
    })
    .chain(set.directories.iter().filter_map(|path| {
      path
        .strip_prefix(&format!("{GENERATED_ROOT}/"))
        .map(str::to_owned)
    }))
    .collect::<BTreeSet<_>>();
  if actual != expected {
    bail!("generated output staging set is incomplete");
  }
  let manifest = set
    .files
    .get(MANIFEST_PATH)
    .context("staged manifest is missing")?;
  self::canonical::<Manifest>(manifest, "manifest")?;
  let sidecar = set
    .files
    .get(&format!("{RESOURCES_PATH}/{SIDECAR_NAME}"))
    .context("staged runtime sidecar is missing")?;
  self::canonical::<Sidecar>(sidecar, "runtime sidecar")?;
  Ok(())
}

pub(crate) fn base_paths(catalog: &AssetCatalog) -> BTreeSet<String> {
  let mut paths = [
    "Resources".to_owned(),
    "Resources.meta".to_owned(),
    format!("Resources/{SIDECAR_NAME}"),
    format!("Resources/{SIDECAR_NAME}.meta"),
    "manifest.json".to_owned(),
    "manifest.json.meta".to_owned(),
    "textures".to_owned(),
    "textures.meta".to_owned(),
  ]
  .into_iter()
  .collect::<BTreeSet<_>>();
  for asset in &catalog.assets {
    let hash = self::hex(&asset.request_identity);
    paths.insert(format!("textures/{hash}.png"));
    paths.insert(format!("textures/{hash}.png.meta"));
  }
  paths
}

pub(crate) fn validate_tree(project: &Path, expected: &BTreeSet<String>) -> Result<()> {
  let root = project.join(GENERATED_ROOT);
  let mut actual = BTreeSet::new();
  self::walk(&root, &root, &mut actual)?;
  if actual != *expected {
    bail!("generated output tree contains missing, unknown, or stale paths");
  }
  let root_meta = project.join(format!("{GENERATED_ROOT}.meta"));
  let metadata =
    fs::symlink_metadata(&root_meta).context("generated root metadata is missing or invalid")?;
  if !metadata.is_file() || metadata.file_type().is_symlink() {
    bail!("generated root metadata is missing or invalid");
  }
  Ok(())
}

pub(crate) fn canonical<T: DeserializeOwned + Serialize>(bytes: &[u8], name: &str) -> Result<T> {
  let value = serde_json::from_slice(bytes)
    .with_context(|| format!("generated {name} has an unrecognized schema"))?;
  if self::canonical_bytes(&value)? != bytes {
    bail!("generated {name} is not canonical JSON");
  }
  Ok(value)
}

pub(crate) fn canonical_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
  let mut bytes = serde_json::to_vec_pretty(value)?;
  bytes.push(b'\n');
  Ok(bytes)
}

pub(crate) fn read(project: &Path, path: &str, report: &mut WorkReport) -> Result<Vec<u8>> {
  let selected = project.join(path);
  let metadata = fs::symlink_metadata(&selected)
    .with_context(|| format!("generated output {path} is missing"))?;
  if !metadata.is_file() || metadata.file_type().is_symlink() {
    bail!("generated output {path} is not a regular file");
  }
  let bytes = fs::read(&selected).with_context(|| format!("failed to read generated {path}"))?;
  report.files_opened += 1;
  report.bytes_read += bytes.len() as u64;
  Ok(bytes)
}

pub(crate) fn file_guid(path: &str) -> String {
  self::hex(&self::derivation(b"reactant-file\0", path.as_bytes())[..16])
}

pub(crate) fn derivation(domain: &[u8], value: &[u8]) -> [u8; 32] {
  let mut hash = Sha256::new();
  hash.update(domain);
  hash.update(value);
  hash.finalize().into()
}

pub(crate) fn record_derivation(
  seen: &mut BTreeMap<String, (String, String)>,
  guid: &str,
  derivation: &str,
  source: &str,
) -> Result<()> {
  if let Some((existing, existing_source)) = seen.get(guid) {
    if existing != derivation {
      bail!("Unity GUID collision between {existing_source} and {source}");
    }
  } else {
    seen.insert(guid.to_owned(), (derivation.to_owned(), source.to_owned()));
  }
  Ok(())
}

pub(crate) fn raster_dimension(logical: f64, scale: u8) -> Result<u32> {
  let pixels = logical * f64::from(scale);
  if pixels < 1.0 || pixels > f64::from(u32::MAX) || pixels.fract() != 0.0 {
    bail!("manifest raster dimension is invalid");
  }
  Ok(pixels as u32)
}

pub(crate) fn validate_hash(value: &str, length: usize, field: &str) -> Result<()> {
  let valid_length = value.len() == length;
  let valid_digits = value
    .bytes()
    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
  if !valid_length || !valid_digits {
    bail!("generated manifest {field} is not lowercase hexadecimal");
  }
  Ok(())
}

pub(crate) fn validate_number(value: f64) -> Result<()> {
  let invalid_zero = value == 0.0 && value.is_sign_negative();
  if !value.is_finite() || value < 0.0 || invalid_zero {
    bail!("generated manifest geometry contains an invalid number");
  }
  Ok(())
}

pub(crate) fn validate_path(path: &str) -> Result<()> {
  let invalid_prefix = path.is_empty() || path.contains('\\') || path.starts_with('/');
  let invalid_segment = path
    .split('/')
    .any(|part| part.is_empty() || matches!(part, "." | ".."));
  if invalid_prefix || invalid_segment {
    bail!("generated manifest dependency path {path:?} is invalid");
  }
  Ok(())
}

pub(crate) fn valid_file_id(value: &str) -> bool {
  let mut parts = value.split(':');
  let family = parts.next();
  let first = parts.next();
  let second = parts.next();
  let valid = |part: Option<&str>| {
    part.is_some_and(|part| {
      !part.is_empty()
        && part
          .bytes()
          .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
  };
  let valid_family = matches!(family, Some("unix" | "windows"));
  let valid_identity = valid(first) && valid(second);
  valid_family && valid_identity && parts.next().is_none()
}

pub(crate) fn strictly_sorted(values: &[String]) -> bool {
  values.windows(2).all(|pair| pair[0] < pair[1])
}

pub(crate) fn hex(bytes: &[u8]) -> String {
  const DIGITS: &[u8; 16] = b"0123456789abcdef";

  let mut output = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    output.push(char::from(DIGITS[usize::from(byte >> 4)]));
    output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
  }
  output
}

fn walk(root: &Path, path: &Path, output: &mut BTreeSet<String>) -> Result<()> {
  let mut entries = fs::read_dir(path)
    .with_context(|| format!("failed to read generated output {}", path.display()))?
    .collect::<std::io::Result<Vec<_>>>()?;
  entries.sort_by_key(fs::DirEntry::file_name);
  for entry in entries {
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() {
      bail!(
        "generated output {} must not be a symbolic link",
        path.display()
      );
    }
    output.insert(self::normalized(path.strip_prefix(root)?));
    if metadata.is_dir() {
      self::walk(root, &path, output)?;
    }
  }
  Ok(())
}

fn normalized(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}
