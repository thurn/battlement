use std::{
  collections::BTreeSet,
  fs::{self, File},
  io::{ErrorKind, Write},
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

use crate::{
  WorkReport,
  manifest_schema::{Manifest, Sidecar},
  manifest_validation::{self as validation, GeneratedSet},
  unity_metadata,
};

const BACKUP_NAME: &str = ".BattlementReactant.backup";
const GENERATED_PARENT: &str = "Assets/Generated";
const GENERATED_ROOT: &str = "Assets/Generated/BattlementReactant";
const MANIFEST_NAME: &str = "manifest.json";
const META_BACKUP_NAME: &str = ".BattlementReactant.meta.backup";
const META_STAGED_NAME: &str = ".BattlementReactant.meta.staged";
const ROOT_META_NAME: &str = "BattlementReactant.meta";
const ROOT_NAME: &str = "BattlementReactant";
const SIDECAR_NAME: &str = "BattlementReactantAssetCatalog.json";
const STAGED_NAME: &str = ".BattlementReactant.staged";

pub(crate) fn install(
  project: &Path,
  set: &GeneratedSet,
  report: &mut WorkReport,
  validate: impl FnOnce(&mut WorkReport) -> Result<()>,
) -> Result<()> {
  self::recover(project, report)?;
  let paths = TransactionPaths::new(project);
  self::remove_directory(&paths.staged, report)?;
  self::remove_directory(&paths.backup, report)?;
  self::remove_file(&paths.meta_staged, report)?;
  self::remove_file(&paths.meta_backup, report)?;
  let metadata = set
    .files
    .get(&format!("{GENERATED_ROOT}.meta"))
    .context("generated root metadata is missing from the staged set")?;
  self::write_staged_root(&paths.staged, set, report)?;
  self::write_staged_metadata(&paths, metadata, report)?;
  self::sync_directory(&paths.parent)?;

  let had_root = paths.root.exists();
  let had_metadata = paths.root_meta.exists();
  if had_root {
    fs::rename(&paths.root, &paths.backup)
      .with_context(|| format!("failed to preserve generated root {}", paths.root.display()))?;
    report.files_written += 1;
    if let Err(error) = self::sync_directory(&paths.parent) {
      self::restore_root(&paths, had_root, report)?;
      return Err(error);
    }
  }
  if let Err(error) = self::activate_staged_root(&paths, report) {
    self::restore_root(&paths, had_root, report)?;
    return Err(error);
  }
  if let Err(error) = self::activate_staged_metadata(&paths, report) {
    self::restore_metadata(&paths, had_metadata, report)?;
    self::restore_root(&paths, had_root, report)?;
    return Err(error);
  }
  if let Err(error) = validate(report) {
    self::restore_metadata(&paths, had_metadata, report)?;
    self::restore_root(&paths, had_root, report)?;
    return Err(error).context("installed generated asset set failed final validation");
  }
  self::remove_directory(&paths.backup, report).ok();
  self::remove_file(&paths.meta_backup, report).ok();
  self::sync_directory(&paths.parent).ok();
  Ok(())
}

pub(crate) fn recover(project: &Path, report: &mut WorkReport) -> Result<()> {
  let paths = TransactionPaths::new(project);
  if !self::has_recovery_artifact(&paths) {
    return Ok(());
  }
  fs::create_dir_all(&paths.parent).with_context(|| {
    format!(
      "failed to open generated asset directory {}",
      paths.parent.display()
    )
  })?;
  let stable_complete = self::manifest_complete(&paths.root)?;
  let backup_complete = self::manifest_complete(&paths.backup)?;
  let staged_complete = self::manifest_complete(&paths.staged)?;

  if !stable_complete {
    self::remove_directory(&paths.root, report)?;
    if backup_complete {
      fs::rename(&paths.backup, &paths.root)
        .context("failed to recover the previous generated asset root")?;
      report.files_written += 1;
    } else if staged_complete {
      fs::rename(&paths.staged, &paths.root)
        .context("failed to recover the staged generated asset root")?;
      report.files_written += 1;
    }
  }
  self::remove_directory(&paths.staged, report)?;
  self::remove_directory(&paths.backup, report)?;
  self::recover_metadata(&paths, report)?;
  self::sync_directory(&paths.parent)?;
  Ok(())
}

struct TransactionPaths {
  backup: PathBuf,
  meta_backup: PathBuf,
  meta_staged: PathBuf,
  parent: PathBuf,
  root: PathBuf,
  root_meta: PathBuf,
  staged: PathBuf,
}

impl TransactionPaths {
  fn new(project: &Path) -> Self {
    let parent = project.join(GENERATED_PARENT);
    Self {
      backup: parent.join(BACKUP_NAME),
      meta_backup: parent.join(META_BACKUP_NAME),
      meta_staged: parent.join(META_STAGED_NAME),
      root: parent.join(ROOT_NAME),
      root_meta: parent.join(ROOT_META_NAME),
      staged: parent.join(STAGED_NAME),
      parent,
    }
  }
}

fn write_staged_root(staged: &Path, set: &GeneratedSet, report: &mut WorkReport) -> Result<()> {
  fs::create_dir_all(staged).with_context(|| {
    format!(
      "failed to create generated staging root {}",
      staged.display()
    )
  })?;
  for directory in &set.directories {
    let relative = self::inside_root(directory)?;
    fs::create_dir_all(staged.join(relative))
      .with_context(|| format!("failed to create staged generated directory {directory}"))?;
  }
  let mut files = set
    .files
    .iter()
    .filter(|(path, _)| path.as_str() != format!("{GENERATED_ROOT}.meta"))
    .collect::<Vec<_>>();
  files.sort_by_key(|(path, _)| usize::from(path.ends_with(MANIFEST_NAME)));
  for (path, bytes) in files {
    let destination = staged.join(self::inside_root(path)?);
    self::write_file(&destination, bytes)
      .with_context(|| format!("failed to write staged generated asset {path}"))?;
    report.files_written += 1;
  }
  self::validate_staged_root(staged, set)?;
  self::sync_tree_directories(staged)?;
  Ok(())
}

fn write_staged_metadata(
  paths: &TransactionPaths,
  expected: &[u8],
  report: &mut WorkReport,
) -> Result<()> {
  if fs::read(&paths.root_meta).is_ok_and(|bytes| bytes == expected) {
    return Ok(());
  }
  self::write_file(&paths.meta_staged, expected).with_context(|| {
    format!(
      "failed to stage generated root metadata {}",
      paths.meta_staged.display()
    )
  })?;
  report.files_written += 1;
  Ok(())
}

fn activate_staged_root(paths: &TransactionPaths, report: &mut WorkReport) -> Result<()> {
  fs::rename(&paths.staged, &paths.root)
    .context("failed to install the staged generated asset root")?;
  report.files_written += 1;
  self::sync_directory(&paths.parent)
}

fn activate_staged_metadata(paths: &TransactionPaths, report: &mut WorkReport) -> Result<()> {
  if !paths.meta_staged.exists() {
    return Ok(());
  }
  if paths.root_meta.exists() {
    fs::rename(&paths.root_meta, &paths.meta_backup)
      .context("failed to preserve generated root metadata")?;
    report.files_written += 1;
  }
  if let Err(error) = fs::rename(&paths.meta_staged, &paths.root_meta) {
    if paths.meta_backup.exists() {
      fs::rename(&paths.meta_backup, &paths.root_meta)
        .context("failed to restore generated root metadata")?;
      report.files_written += 1;
    }
    return Err(error).context("failed to install generated root metadata");
  }
  report.files_written += 1;
  self::sync_directory(&paths.parent)
}

fn restore_root(paths: &TransactionPaths, had_root: bool, report: &mut WorkReport) -> Result<()> {
  self::remove_directory(&paths.root, report)?;
  if had_root && paths.backup.exists() {
    fs::rename(&paths.backup, &paths.root)
      .context("failed to roll back generated asset replacement")?;
    report.files_written += 1;
  }
  self::sync_directory(&paths.parent)
}

fn restore_metadata(
  paths: &TransactionPaths,
  had_metadata: bool,
  report: &mut WorkReport,
) -> Result<()> {
  if paths.meta_backup.exists() {
    self::remove_file(&paths.root_meta, report)?;
    fs::rename(&paths.meta_backup, &paths.root_meta)
      .context("failed to roll back generated root metadata")?;
    report.files_written += 1;
  } else if !had_metadata {
    self::remove_file(&paths.root_meta, report)?;
  }
  Ok(())
}

fn recover_metadata(paths: &TransactionPaths, report: &mut WorkReport) -> Result<()> {
  if !paths.root.exists() {
    self::remove_file(&paths.root_meta, report)?;
    self::remove_file(&paths.meta_staged, report)?;
    self::remove_file(&paths.meta_backup, report)?;
    return Ok(());
  }
  let guid = validation::hex(
    &validation::derivation(b"reactant-directory\0", GENERATED_ROOT.as_bytes())[..16],
  );
  let expected = unity_metadata::directory(&guid);
  if fs::read(&paths.root_meta).is_ok_and(|bytes| bytes == expected) {
    self::remove_file(&paths.meta_staged, report)?;
    self::remove_file(&paths.meta_backup, report)?;
    return Ok(());
  }
  self::write_file(&paths.meta_staged, &expected)?;
  report.files_written += 1;
  self::remove_file(&paths.root_meta, report)?;
  fs::rename(&paths.meta_staged, &paths.root_meta)
    .context("failed to recover generated root metadata")?;
  report.files_written += 1;
  self::remove_file(&paths.meta_backup, report)?;
  Ok(())
}

fn manifest_complete(root: &Path) -> Result<bool> {
  match self::check_manifest_complete(root) {
    Ok(complete) => Ok(complete),
    Err(error) => {
      let filesystem = error.downcast_ref::<std::io::Error>();
      if filesystem.is_some_and(|error| error.kind() != ErrorKind::NotFound) {
        return Err(error).with_context(|| {
          format!(
            "failed to inspect generated transaction root {}",
            root.display()
          )
        });
      }
      Ok(false)
    }
  }
}

fn check_manifest_complete(root: &Path) -> Result<bool> {
  let metadata = match fs::symlink_metadata(root) {
    Ok(metadata) => metadata,
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
    Err(error) => return Err(error.into()),
  };
  if !metadata.is_dir() || metadata.file_type().is_symlink() {
    return Ok(false);
  }
  let manifest_bytes = fs::read(root.join(MANIFEST_NAME))?;
  let manifest = validation::canonical::<Manifest>(&manifest_bytes, "recovery manifest")?;
  let sidecar_path = root.join("Resources").join(SIDECAR_NAME);
  let sidecar =
    validation::canonical::<Sidecar>(&fs::read(sidecar_path)?, "recovery runtime sidecar")?;
  let addresses = manifest
    .assets
    .iter()
    .map(|asset| asset.address.clone())
    .collect::<Vec<_>>();
  if !validation::strictly_sorted(&addresses) || sidecar.addresses != addresses {
    return Ok(false);
  }
  if sidecar.manifest_sha256 != validation::hex(&Sha256::digest(&manifest_bytes)) {
    return Ok(false);
  }
  let mut expected = [
    "Resources".to_owned(),
    "Resources.meta".to_owned(),
    format!("Resources/{SIDECAR_NAME}"),
    format!("Resources/{SIDECAR_NAME}.meta"),
    MANIFEST_NAME.to_owned(),
    format!("{MANIFEST_NAME}.meta"),
    "textures".to_owned(),
    "textures.meta".to_owned(),
  ]
  .into_iter()
  .collect::<BTreeSet<_>>();
  for asset in manifest.assets {
    validation::validate_path(&asset.png)?;
    let png = fs::read(root.join(&asset.png))?;
    if validation::hex(&Sha256::digest(&png)) != asset.png_sha256 {
      return Ok(false);
    }
    expected.insert(asset.png.clone());
    expected.insert(format!("{}.meta", asset.png));
  }
  Ok(self::tree(root)? == expected)
}

fn validate_staged_root(staged: &Path, set: &GeneratedSet) -> Result<()> {
  let expected = set
    .directories
    .iter()
    .filter(|path| path.as_str() != GENERATED_ROOT)
    .map(|path| self::inside_root(path).map(str::to_owned))
    .chain(set.files.keys().filter_map(|path| {
      if path == &format!("{GENERATED_ROOT}.meta") {
        None
      } else {
        Some(self::inside_root(path).map(str::to_owned))
      }
    }))
    .collect::<Result<BTreeSet<_>>>()?;
  if self::tree(staged)? != expected {
    bail!("generated output staging tree is incomplete");
  }
  for (path, bytes) in &set.files {
    if path == &format!("{GENERATED_ROOT}.meta") {
      continue;
    }
    if fs::read(staged.join(self::inside_root(path)?))? != *bytes {
      bail!("staged generated asset {path} changed while being written");
    }
  }
  Ok(())
}

fn tree(root: &Path) -> Result<BTreeSet<String>> {
  let mut output = BTreeSet::new();
  self::walk(root, root, &mut output)?;
  Ok(output)
}

fn walk(root: &Path, directory: &Path, output: &mut BTreeSet<String>) -> Result<()> {
  let mut entries = fs::read_dir(directory)?.collect::<std::io::Result<Vec<_>>>()?;
  entries.sort_by_key(fs::DirEntry::file_name);
  for entry in entries {
    let path = entry.path();
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() {
      bail!("generated transaction paths must not be symbolic links");
    }
    output.insert(self::normalized(path.strip_prefix(root)?));
    if metadata.is_dir() {
      self::walk(root, &path, output)?;
    }
  }
  Ok(())
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
  let mut file = File::create(path)?;
  file.write_all(bytes)?;
  file.sync_all()?;
  Ok(())
}

fn inside_root(path: &str) -> Result<&str> {
  if path == GENERATED_ROOT {
    return Ok("");
  }
  path
    .strip_prefix(&format!("{GENERATED_ROOT}/"))
    .context("generated staging path escaped its root")
}

fn has_recovery_artifact(paths: &TransactionPaths) -> bool {
  [
    &paths.staged,
    &paths.backup,
    &paths.meta_staged,
    &paths.meta_backup,
  ]
  .into_iter()
  .any(|path| path.exists())
}

fn remove_directory(path: &Path, report: &mut WorkReport) -> Result<()> {
  if path.exists() {
    fs::remove_dir_all(path)
      .with_context(|| format!("failed to remove transaction directory {}", path.display()))?;
    report.files_written += 1;
  }
  Ok(())
}

fn remove_file(path: &Path, report: &mut WorkReport) -> Result<()> {
  if path.exists() {
    fs::remove_file(path)
      .with_context(|| format!("failed to remove transaction file {}", path.display()))?;
    report.files_written += 1;
  }
  Ok(())
}

fn sync_tree_directories(root: &Path) -> Result<()> {
  let mut directories = vec![root.to_owned()];
  self::collect_directories(root, &mut directories)?;
  directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
  for directory in directories {
    self::sync_directory(&directory)?;
  }
  Ok(())
}

fn collect_directories(path: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
  for entry in fs::read_dir(path)? {
    let entry = entry?;
    let metadata = entry.file_type()?;
    if metadata.is_dir() && !metadata.is_symlink() {
      output.push(entry.path());
      self::collect_directories(&entry.path(), output)?;
    }
  }
  Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
  File::open(path)?.sync_all()?;
  Ok(())
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
  Ok(())
}

fn normalized(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}
