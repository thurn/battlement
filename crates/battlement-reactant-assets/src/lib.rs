//! Discovery and host-side generation for Reactant assets.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::{
  env, fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Serialize;
use tempfile::Builder;

mod browser;
mod browser_protocol;
mod dependency;
mod diagnostics;
mod discovery;
mod identity;
mod incremental;
mod manifest;
mod manifest_schema;
mod manifest_validation;
mod output_index;
mod png_output;
mod preview;
mod renderer_document;
mod source_scan;
mod transaction;
#[cfg(test)]
mod transaction_tests;
mod unity_metadata;

pub use discovery::{DiscoveredAsset, Discovery};
pub use identity::{AssetCatalog, CatalogAsset, DependencyIdentity, DirectoryIdentity};

const GENERATED_ROOT: &str = "Assets/Generated/BattlementReactant";
const GENERATED_ROOT_META: &str = "Assets/Generated/BattlementReactant.meta";
/// Operation performed by the asset generator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetCommand {
  /// Generate the exact declared asset set.
  Generate,
  /// Check that generated output is current without modifying the Unity project.
  Check,
  /// Generate and open the local asset preview.
  Preview,
}

/// Cargo feature flags shared by host and WebAssembly discovery.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FeatureSelection {
  /// Named Cargo features to enable.
  pub features: Vec<String>,
  /// Enable every feature exposed by the rules package.
  pub all_features: bool,
  /// Disable the package's default features.
  pub no_default_features: bool,
}

/// Inputs shared by all asset-generator commands.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandOptions {
  /// Unity project directory, or automatic ancestor discovery when omitted.
  pub project: Option<PathBuf>,
  /// Rules package manifest, defaulting to `rules/Cargo.toml` in the project.
  pub manifest_path: Option<PathBuf>,
  /// Cargo features used for both discovery graphs.
  pub feature_selection: FeatureSelection,
  /// Explicit Chrome or Chromium executable.
  pub browser: Option<PathBuf>,
  /// Optional destination for the aggregate command work report.
  pub work_report: Option<PathBuf>,
}

/// Aggregate observable work performed by one public command.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkReport {
  /// Browser contexts created.
  pub browser_contexts_created: u64,
  /// Browser executable files opened.
  pub browser_executable_opens: u64,
  /// Browser processes launched.
  pub browser_launches: u64,
  /// Bytes read directly from files.
  pub bytes_read: u64,
  /// Cargo metadata subprocesses run.
  pub cargo_metadata_runs: u64,
  /// Dependency files opened.
  pub dependency_file_opens: u64,
  /// Files opened directly by the command.
  pub files_opened: u64,
  /// Files created, replaced, or removed.
  pub files_written: u64,
  /// Generated PNG files opened.
  pub generated_png_opens: u64,
  /// Rust source files opened.
  pub rust_source_opens: u64,
  /// Filesystem metadata probes performed.
  pub stat_calls: u64,
  /// Child processes started.
  pub subprocesses_started: u64,
}

/// Runs one public asset-generator command and writes its requested work report.
pub fn run(command: AssetCommand, options: &CommandOptions) -> Result<()> {
  let mut report = WorkReport::default();
  let result = self::run_inner(command, options, &mut report);
  let report_result = options
    .work_report
    .as_deref()
    .map(|path| self::write_report(path, &report))
    .transpose();
  match (result, report_result) {
    (Err(error), _) => Err(error),
    (Ok(()), Err(error)) => Err(error),
    (Ok(()), Ok(_)) => Ok(()),
  }
}

fn run_inner(
  command: AssetCommand,
  options: &CommandOptions,
  report: &mut WorkReport,
) -> Result<()> {
  let current = env::current_dir().context("failed to read the current directory")?;
  let project = self::select_project(options.project.as_deref(), &current, report)?;
  if command != AssetCommand::Check {
    transaction::recover(&project, report)?;
  }
  let manifest = self::select_manifest(options.manifest_path.as_deref(), &project, &current)?;
  let mut index =
    incremental::IncrementalIndex::load(&project, &manifest, &options.feature_selection, report)?;
  let discovery = discovery::discover(
    &manifest,
    &project,
    &options.feature_selection,
    &mut index,
    report,
  )?;
  let catalog = identity::resolve(&discovery, &project, index.dependencies(), report)?;
  let semantic_unchanged = index.record_catalog(&catalog)?;
  let outputs_current = index.outputs_current(report);
  let browser_stale = index.stale_browser_addresses(&catalog, &project, report);
  if !discovery.assets.is_empty() {
    let fast_current = semantic_unchanged && outputs_current && browser_stale.is_empty();
    let diagnostics = if fast_current {
      diagnostics::AssetDiagnostics::current(&catalog)
    } else {
      diagnostics::classify(&project, &catalog, &browser_stale, report)
    };
    for asset in &catalog.assets {
      let dependencies = asset
        .dependencies
        .iter()
        .map(|dependency| format!("{}={}", dependency.path, self::hex(&dependency.identity)))
        .collect::<Vec<_>>()
        .join(",");
      println!(
        "asset={} guid={} dependencies=[{}] sources=[{}]",
        asset.address,
        asset.guid,
        dependencies,
        asset.source_symbols.join(",")
      );
    }
    for directory in &catalog.directories {
      println!("directory={} guid={}", directory.path, directory.guid);
    }
    if command == AssetCommand::Check {
      diagnostics.emit();
      self::print_counts(
        &discovery,
        &catalog,
        diagnostics.current_count(),
        0,
        diagnostics.stale_count(),
      );
      println!("browser not started");
      if !diagnostics.is_clean() {
        bail!(
          "generated Reactant assets are stale; run `cargo battlement reactant assets generate`"
        );
      }
      index.refresh_outputs(&project, report)?;
      return Ok(());
    }
    let (dependencies, browser_index) = index.render_state();
    let browser = browser::prepare(
      options,
      &catalog,
      &project,
      dependencies,
      browser_index,
      report,
    )?;
    println!(
      "browser={} product={}/{} protocol={} executable={} renderer={} session-requests={}",
      browser.executable_path,
      browser.product,
      browser.version,
      browser.protocol_version,
      browser.executable_sha256,
      browser.renderer_identity,
      browser.session_requests
    );
    for request in &browser.requests {
      println!(
        "cache={} key={} probe={}",
        request.address, request.cache_key, request.image_hash
      );
      println!(
        "render={} dimensions={}x{} alpha={},{},{},{}",
        request.address,
        request.width,
        request.height,
        request.alpha.left,
        request.alpha.top,
        request.alpha.right,
        request.alpha.bottom
      );
      for warning in &request.warnings {
        println!("warning[{warning}] asset={}", request.address);
      }
    }
    if !diagnostics.is_clean() || browser.session_requests != 0 {
      manifest::install(&project, &catalog, &browser, report)?;
    }
    diagnostics.emit();
    self::print_counts(
      &discovery,
      &catalog,
      diagnostics.current_count(),
      browser.session_requests,
      diagnostics.stale_count(),
    );
    if browser.session_requests == 0 {
      println!("browser not started");
    }
    index.refresh_outputs(&project, report)?;
    index.save(report)?;
    if command == AssetCommand::Preview {
      preview::open(&project, &discovery, &catalog, &browser, report)?;
    }
    return Ok(());
  }
  match command {
    AssetCommand::Generate => self::remove_generated_output(&project, report)?,
    AssetCommand::Check => self::check_empty_output(&project, report)?,
    AssetCommand::Preview => {
      self::remove_generated_output(&project, report)?;
      preview::open_empty(&project, report)?;
    }
  }
  index.refresh_outputs(&project, report)?;
  if command != AssetCommand::Check {
    index.save(report)?;
  }
  println!("discovered=0 deduplicated=0 current=0 rendered=0 stale=0; browser not started");
  Ok(())
}

fn select_project(
  explicit: Option<&Path>,
  current: &Path,
  report: &mut WorkReport,
) -> Result<PathBuf> {
  if let Some(path) = explicit {
    return self::canonical_project(&current.join(path), report);
  }
  for ancestor in current.ancestors() {
    report.stat_calls += 3;
    if self::is_unity_project(ancestor) {
      return ancestor
        .canonicalize()
        .with_context(|| format!("failed to open Unity project {}", ancestor.display()));
    }
  }
  bail!(
    "could not find a Unity project from {}; pass --project <path>",
    current.display()
  )
}

fn canonical_project(path: &Path, report: &mut WorkReport) -> Result<PathBuf> {
  let path = path
    .canonicalize()
    .with_context(|| format!("failed to open Unity project {}", path.display()))?;
  report.stat_calls += 3;
  if !self::is_unity_project(&path) {
    bail!("{} is not a Unity project", path.display());
  }
  Ok(path)
}

fn is_unity_project(path: &Path) -> bool {
  path.join("Assets").is_dir()
    && path.join("Packages/manifest.json").is_file()
    && path.join("ProjectSettings/ProjectVersion.txt").is_file()
}

fn select_manifest(explicit: Option<&Path>, project: &Path, current: &Path) -> Result<PathBuf> {
  let selected = explicit
    .map(|path| current.join(path))
    .unwrap_or_else(|| project.join("rules/Cargo.toml"));
  let manifest = selected
    .canonicalize()
    .with_context(|| format!("failed to locate rules manifest {}", selected.display()))?;
  if !manifest.starts_with(project) {
    bail!(
      "rules manifest {} must be contained by Unity project {}",
      manifest.display(),
      project.display()
    );
  }
  Ok(manifest)
}

fn remove_generated_output(project: &Path, report: &mut WorkReport) -> Result<()> {
  let root = project.join(GENERATED_ROOT);
  let metadata = project.join(GENERATED_ROOT_META);
  report.stat_calls += 2;
  if root.exists() {
    fs::remove_dir_all(&root)
      .with_context(|| format!("failed to remove generated root {}", root.display()))?;
    report.files_written += 1;
  }
  if metadata.exists() {
    fs::remove_file(&metadata)
      .with_context(|| format!("failed to remove generated metadata {}", metadata.display()))?;
    report.files_written += 1;
  }
  Ok(())
}

fn check_empty_output(project: &Path, report: &mut WorkReport) -> Result<()> {
  let root = project.join(GENERATED_ROOT);
  let metadata = project.join(GENERATED_ROOT_META);
  report.stat_calls += 2;
  if root.exists() || metadata.exists() {
    bail!("generated Reactant assets are stale; run `cargo battlement reactant assets generate`");
  }
  Ok(())
}

fn write_report(path: &Path, report: &WorkReport) -> Result<()> {
  let parent = path
    .parent()
    .filter(|parent| !parent.as_os_str().is_empty())
    .unwrap_or(Path::new("."));
  let mut temporary = Builder::new()
    .prefix(".reactant-asset-work-report-")
    .tempfile_in(parent)
    .with_context(|| format!("failed to create work report beside {}", path.display()))?;
  serde_json::to_writer_pretty(temporary.as_file_mut(), report)
    .context("failed to serialize asset work report")?;
  use std::io::Write;
  temporary.as_file_mut().write_all(b"\n")?;
  temporary
    .persist(path)
    .map_err(|error| error.error)
    .with_context(|| format!("failed to install work report {}", path.display()))?;
  Ok(())
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

fn print_counts(
  discovery: &Discovery,
  catalog: &AssetCatalog,
  current: usize,
  rendered: usize,
  stale: usize,
) {
  println!(
    "discovered={} deduplicated={} current={current} rendered={rendered} stale={stale}",
    discovery.assets.len(),
    catalog.assets.len(),
  );
}
