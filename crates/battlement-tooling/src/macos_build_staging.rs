use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Result, ensure};

const IDENTITY_ASSET: &str = "Assets/Resources/BattlementDittoBuildIdentity.json";
const IDENTITY_META: &str = "Assets/Resources/BattlementDittoBuildIdentity.json.meta";
const NATIVE_PLUGIN: &str = "Assets/Plugins/macOS/libbattlement_rules.dylib";
const NATIVE_PLUGIN_META: &str = "Assets/Plugins/macOS/libbattlement_rules.dylib.meta";
const WEBGL_PLUGIN: &str = "Assets/Plugins/WebGL/libbattlement_rules.a";
const WEBGL_PLUGIN_META: &str = "Assets/Plugins/WebGL/libbattlement_rules.a.meta";
const IOS_PLUGIN: &str = "Assets/Plugins/iOS/libbattlement_rules.a";
const IOS_PLUGIN_META: &str = "Assets/Plugins/iOS/libbattlement_rules.a.meta";
const PLUGIN_META: &[u8] = b"fileFormatVersion: 2\nguid: 821c7f6f38454ea0ab770332096066f8\nPluginImporter:\n  externalObjects: {}\n  serializedVersion: 3\n  iconMap: {}\n  executionOrder: {}\n  defineConstraints: []\n  isPreloaded: 0\n  isOverridable: 0\n  isExplicitlyReferenced: 0\n  validateReferences: 1\n  platformData: []\n  userData:\n  assetBundleName:\n  assetBundleVariant:\n";
const RESOURCE_META: &[u8] = b"fileFormatVersion: 2\nguid: db637482c2d34aec968a458799848ce2\nTextScriptImporter:\n  externalObjects: {}\n  userData:\n  assetBundleName:\n  assetBundleVariant:\n";
const MUTABLE_PATHS: &[&str] = &[
  "Assets/AddressableAssetsData",
  "Assets/AddressableAssetsData.meta",
  "Assets/DefaultVolumeProfile.asset",
  "Assets/DefaultVolumeProfile.asset.meta",
  "Assets/Generated",
  "Assets/Generated.meta",
  "Assets/Original",
  "Assets/Plugins.meta",
  "Assets/Plugins/iOS.meta",
  "Assets/Plugins/macOS.meta",
  "Assets/Plugins/WebGL.meta",
  "Assets/Resources/PerformanceTestRunInfo.json",
  "Assets/Resources/PerformanceTestRunInfo.json.meta",
  "Assets/Resources/PerformanceTestRunSettings.json",
  "Assets/Resources/PerformanceTestRunSettings.json.meta",
  "Assets/Scenes.meta",
  "Assets/UniversalRenderPipelineGlobalSettings.asset",
  "Assets/UniversalRenderPipelineGlobalSettings.asset.meta",
  "Data",
  "Packages/packages-lock.json",
  "ProjectSettings/EditorBuildSettings.asset",
  "ProjectSettings/GraphicsSettings.asset",
  "ProjectSettings/ProjectAuditorSettings.asset",
  "ProjectSettings/ProjectSettings.asset",
  "ProjectSettings/ShaderGraphSettings.asset",
  "ProjectSettings/TimeManager.asset",
];

pub(super) struct ProjectStaging {
  files: Vec<StagedFile>,
  mutable: Vec<SavedPath>,
  backup_root: PathBuf,
  restored: bool,
}

impl ProjectStaging {
  pub(super) fn new(
    project: &Path,
    plugin: &Path,
    identity: &[u8],
    backup_root: &Path,
  ) -> Result<Self> {
    Self::for_plugin(
      project,
      plugin,
      identity,
      backup_root,
      NATIVE_PLUGIN,
      NATIVE_PLUGIN_META,
    )
  }

  pub(super) fn webgl(
    project: &Path,
    plugin: &Path,
    identity: &[u8],
    backup_root: &Path,
  ) -> Result<Self> {
    Self::for_plugin(
      project,
      plugin,
      identity,
      backup_root,
      WEBGL_PLUGIN,
      WEBGL_PLUGIN_META,
    )
  }

  pub(super) fn ios(
    project: &Path,
    plugin: &Path,
    identity: &[u8],
    backup_root: &Path,
  ) -> Result<Self> {
    Self::for_plugin(
      project,
      plugin,
      identity,
      backup_root,
      IOS_PLUGIN,
      IOS_PLUGIN_META,
    )
  }

  fn for_plugin(
    project: &Path,
    plugin: &Path,
    identity: &[u8],
    backup_root: &Path,
    plugin_path: &str,
    plugin_meta: &str,
  ) -> Result<Self> {
    fs::create_dir(backup_root)?;
    let mutable = MUTABLE_PATHS
      .iter()
      .enumerate()
      .map(|(index, relative)| {
        SavedPath::capture(
          &project.join(relative),
          &backup_root.join(index.to_string()),
        )
      })
      .collect::<Result<Vec<_>>>()?;
    Ok(Self {
      files: vec![
        StagedFile::write(&project.join(plugin_path), &fs::read(plugin)?)?,
        StagedFile::write(&project.join(plugin_meta), PLUGIN_META)?,
        StagedFile::write(&project.join(IDENTITY_ASSET), identity)?,
        StagedFile::write(&project.join(IDENTITY_META), RESOURCE_META)?,
      ],
      mutable,
      backup_root: backup_root.to_owned(),
      restored: false,
    })
  }

  pub(super) fn restore(mut self) -> Result<()> {
    for file in self.files.iter_mut().rev() {
      file.restore()?;
    }
    for path in &self.mutable {
      path.restore()?;
    }
    fs::remove_dir_all(&self.backup_root)?;
    self.restored = true;
    Ok(())
  }
}

impl Drop for ProjectStaging {
  fn drop(&mut self) {
    if self.restored {
      return;
    }
    for file in self.files.iter_mut().rev() {
      let _ = file.restore();
    }
    for path in &self.mutable {
      let _ = path.restore();
    }
    if self.backup_root.exists() {
      let _ = fs::remove_dir_all(&self.backup_root);
    }
  }
}

struct SavedPath {
  path: PathBuf,
  backup: Option<PathBuf>,
}

impl SavedPath {
  fn capture(path: &Path, backup: &Path) -> Result<Self> {
    let backup = if path.exists() {
      self::copy_path(path, backup)?;
      Some(backup.to_owned())
    } else {
      None
    };
    Ok(Self {
      path: path.to_owned(),
      backup,
    })
  }

  fn restore(&self) -> Result<()> {
    self::remove_path(&self.path)?;
    if let Some(backup) = &self.backup {
      self::copy_path(backup, &self.path)?;
    }
    Ok(())
  }
}

struct StagedFile {
  path: PathBuf,
  previous: Option<Vec<u8>>,
  restored: bool,
}

impl StagedFile {
  fn write(path: &Path, bytes: &[u8]) -> Result<Self> {
    if path.exists() {
      ensure!(
        !fs::symlink_metadata(path)?.file_type().is_symlink(),
        "build staging path is a symlink"
      );
      ensure!(path.is_file(), "build staging path is not a file");
    }
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    let previous = path.is_file().then(|| fs::read(path)).transpose()?;
    fs::write(path, bytes)?;
    Ok(Self {
      path: path.to_owned(),
      previous,
      restored: false,
    })
  }

  fn restore(&mut self) -> Result<()> {
    if self.restored {
      return Ok(());
    }
    if let Some(previous) = &self.previous {
      fs::write(&self.path, previous)?;
    } else if self.path.exists() {
      fs::remove_file(&self.path)?;
    }
    self.restored = true;
    Ok(())
  }
}

impl Drop for StagedFile {
  fn drop(&mut self) {
    let _ = self.restore();
  }
}

fn copy_path(source: &Path, destination: &Path) -> Result<()> {
  let metadata = fs::symlink_metadata(source)?;
  ensure!(
    !metadata.file_type().is_symlink(),
    "mutable Unity path is a symlink"
  );
  if metadata.is_dir() {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
      let entry = entry?;
      self::copy_path(&entry.path(), &destination.join(entry.file_name()))?;
    }
  } else {
    fs::create_dir_all(destination.parent().expect("backup has a parent"))?;
    fs::copy(source, destination)?;
  }
  Ok(())
}

fn remove_path(path: &Path) -> Result<()> {
  if path.is_dir() {
    fs::remove_dir_all(path)?;
  } else if path.exists() {
    fs::remove_file(path)?;
  }
  Ok(())
}
