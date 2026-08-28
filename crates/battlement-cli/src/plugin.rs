use std::{
  collections::BTreeSet,
  ffi::OsString,
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::{plugin_build, tools};

const PLUGIN_NAME: &str = "libbattlement_rules.dylib";
const REQUIRED_SYMBOLS: [&str; 7] = [
  "battlement_buffer_free",
  "battlement_connect",
  "battlement_engine_create",
  "battlement_engine_destroy",
  "battlement_logging_drain",
  "battlement_poll",
  "battlement_submit",
];

pub(crate) struct PluginDetails {
  architectures: Vec<String>,
}

pub(crate) fn inspect(app: &Path) -> Result<()> {
  let plugin = installed_plugin(app)?;
  let details = details(&plugin)?;
  println!("Application: {}", app.display());
  println!("Plugin: {}", plugin.display());
  println!("Architectures: {}", details.architectures.join(", "));
  println!("Battlement ABI: valid");
  println!(
    "Backup: {}",
    if backup_plugin(app).is_file() {
      backup_plugin(app).display().to_string()
    } else {
      "none".to_owned()
    }
  );
  println!(
    "Code signature: {}",
    if tools::signature_is_valid(app) {
      "valid"
    } else {
      "missing or invalid"
    }
  );
  Ok(())
}

pub(crate) fn verify(library: &Path) -> Result<PluginDetails> {
  let details = details(library)?;
  println!("Verified plugin: {}", library.display());
  println!("Architectures: {}", details.architectures.join(", "));
  println!("Battlement ABI: valid");
  Ok(details)
}

pub(crate) fn install(app: &Path, library: &Path, identity: Option<&str>) -> Result<()> {
  let replacement = verify(library)?;
  let destination = installed_plugin(app)?;
  if same_file(library, &destination)? {
    bail!("the source library is already installed in the application");
  }
  require_architectures(&replacement, &details(&destination)?)?;

  let backup = backup_plugin(app);
  if !backup.exists() {
    fs::create_dir_all(backup.parent().expect("backup has a parent"))
      .with_context(|| format!("failed to create backup directory for {}", app.display()))?;
    fs::copy(&destination, &backup)
      .with_context(|| format!("failed to back up {}", destination.display()))?;
  }

  let displaced = replace(app, &destination, library)?;
  if let Some(identity) = identity
    && let Err(error) = sign_installation(app, &destination, identity)
  {
    roll_back(&destination, &displaced)?;
    return Err(error).context("failed to sign installed plugin");
  }
  discard_displaced(&displaced)?;

  println!("Installed plugin: {}", destination.display());
  println!("Original plugin backup: {}", backup.display());
  print_signing(identity);
  Ok(())
}

pub(crate) fn build_and_install(
  app: &Path,
  package: &str,
  release: bool,
  manifest_path: Option<&Path>,
  identity: Option<&str>,
) -> Result<()> {
  let installed = installed_plugin(app)?;
  let architectures = details(&installed)?.architectures;
  let library = plugin_build::rules_plugin(package, &architectures, release, manifest_path)?;
  install(app, &library, identity)
}

pub(crate) fn restore(app: &Path, identity: Option<&str>) -> Result<()> {
  let destination = installed_plugin(app)?;
  let backup = backup_plugin(app);
  require_file(&backup, "backup")?;
  details(&backup).context("the saved plugin backup is invalid")?;
  let displaced = replace(app, &destination, &backup)?;

  if let Some(identity) = identity
    && let Err(error) = sign_installation(app, &destination, identity)
  {
    roll_back(&destination, &displaced)?;
    return Err(error).context("failed to sign restored plugin");
  }
  discard_displaced(&displaced)?;
  fs::remove_file(&backup).context("failed to remove restored backup")?;
  remove_empty_backup_directory(&backup)?;

  println!("Restored plugin: {}", destination.display());
  print_signing(identity);
  Ok(())
}

fn details(library: &Path) -> Result<PluginDetails> {
  require_file(library, "plugin")?;
  let architectures = tools::architectures(library)?;
  validate_symbols(&tools::exported_symbols(library)?)?;
  Ok(PluginDetails { architectures })
}

fn installed_plugin(app: &Path) -> Result<PathBuf> {
  if app.extension() != Some("app".as_ref()) {
    bail!("expected a macOS .app bundle: {}", app.display());
  }
  require_file(&app.join("Contents/Info.plist"), "application Info.plist")?;
  let plugin = app.join("Contents/PlugIns").join(PLUGIN_NAME);
  require_file(&plugin, "installed plugin")?;
  Ok(plugin)
}

fn backup_plugin(app: &Path) -> PathBuf {
  let mut directory: OsString = app.as_os_str().to_owned();
  directory.push(".battlement-backup");
  PathBuf::from(directory).join(PLUGIN_NAME)
}

fn validate_symbols(symbols: &[String]) -> Result<()> {
  let symbols: BTreeSet<&str> = symbols.iter().map(String::as_str).collect();
  let missing: Vec<&str> = REQUIRED_SYMBOLS
    .iter()
    .copied()
    .filter(|symbol| !symbols.contains(symbol))
    .collect();
  if !missing.is_empty() {
    bail!("plugin is missing required exports: {}", missing.join(", "));
  }
  Ok(())
}

fn require_architectures(replacement: &PluginDetails, installed: &PluginDetails) -> Result<()> {
  let replacement: BTreeSet<&str> = replacement
    .architectures
    .iter()
    .map(String::as_str)
    .collect();
  let missing: Vec<&str> = installed
    .architectures
    .iter()
    .map(String::as_str)
    .filter(|architecture| !replacement.contains(architecture))
    .collect();
  if !missing.is_empty() {
    bail!(
      "replacement plugin is missing packaged architectures: {}",
      missing.join(", ")
    );
  }
  Ok(())
}

fn replace(app: &Path, destination: &Path, source: &Path) -> Result<PathBuf> {
  let displaced = displaced_plugin(app);
  let directory = displaced.parent().expect("displaced plugin has a parent");
  let staged = directory.join(format!("{PLUGIN_NAME}.installing"));
  fs::create_dir_all(directory).context("failed to create temporary rollback directory")?;
  fs::copy(source, &staged).with_context(|| format!("failed to stage {}", source.display()))?;
  fs::rename(destination, &displaced)
    .with_context(|| format!("failed to preserve {}", destination.display()))?;
  if let Err(error) = fs::rename(&staged, destination) {
    fs::rename(&displaced, destination).context("failed to roll back replacement")?;
    fs::remove_file(&staged).ok();
    fs::remove_dir(directory).ok();
    return Err(error).with_context(|| format!("failed to replace {}", destination.display()));
  }
  Ok(displaced)
}

fn roll_back(destination: &Path, displaced: &Path) -> Result<()> {
  let failed = displaced.with_file_name(format!("{PLUGIN_NAME}.failed"));
  fs::rename(destination, &failed).context("failed to preserve rejected plugin")?;
  fs::rename(displaced, destination).context("failed to restore previous plugin")?;
  fs::remove_file(failed).context("failed to remove rejected plugin")?;
  remove_empty_backup_directory(displaced)
}

fn discard_displaced(displaced: &Path) -> Result<()> {
  fs::remove_file(displaced).context("failed to remove replaced plugin")?;
  remove_empty_backup_directory(displaced)
}

fn sign_installation(app: &Path, plugin: &Path, identity: &str) -> Result<()> {
  tools::sign(plugin, identity)?;
  tools::sign(app, identity)
}

fn print_signing(identity: Option<&str>) {
  if let Some(identity) = identity {
    println!("Code signing identity: {identity}");
  } else {
    println!("Code signing: skipped");
  }
}

fn require_file(path: &Path, description: &str) -> Result<()> {
  if !path.is_file() {
    bail!("{description} was not found at {}", path.display());
  }
  Ok(())
}

fn same_file(left: &Path, right: &Path) -> Result<bool> {
  Ok(fs::canonicalize(left)? == fs::canonicalize(right)?)
}

fn displaced_plugin(app: &Path) -> PathBuf {
  let mut directory: OsString = app.as_os_str().to_owned();
  directory.push(format!(".battlement-replaced.{}", std::process::id()));
  PathBuf::from(directory).join(PLUGIN_NAME)
}

fn remove_empty_backup_directory(backup: &Path) -> Result<()> {
  let directory = backup.parent().expect("backup has a parent");
  fs::remove_dir(directory).with_context(|| {
    format!(
      "failed to remove empty backup directory {}",
      directory.display()
    )
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn symbol_validation_reports_every_missing_export() {
    let error = validate_symbols(&["battlement_connect".to_owned()]).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("battlement_submit"));
    assert!(!message.contains("battlement_connect,"));
  }

  #[test]
  fn previous_symbol_set_is_rejected() {
    let symbols = REQUIRED_SYMBOLS
      .into_iter()
      .filter(|symbol| *symbol != "battlement_logging_drain")
      .map(str::to_owned)
      .collect::<Vec<_>>();
    assert_eq!(
      validate_symbols(&symbols).unwrap_err().to_string(),
      "plugin is missing required exports: battlement_logging_drain"
    );
  }

  #[test]
  fn backup_is_stored_beside_the_application_bundle() {
    assert_eq!(
      backup_plugin(Path::new("/build/Game.app")),
      Path::new("/build/Game.app.battlement-backup/libbattlement_rules.dylib")
    );
  }

  #[test]
  fn replacement_must_cover_every_packaged_architecture() {
    let replacement = PluginDetails {
      architectures: vec!["arm64".to_owned()],
    };
    let installed = PluginDetails {
      architectures: vec!["arm64".to_owned(), "x86_64".to_owned()],
    };
    assert_eq!(
      require_architectures(&replacement, &installed)
        .unwrap_err()
        .to_string(),
      "replacement plugin is missing packaged architectures: x86_64"
    );
  }
}
