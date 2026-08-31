use std::{
  collections::{BTreeMap, BTreeSet},
  env, fs,
  io::Write,
  path::{Path, PathBuf},
  time::UNIX_EPOCH,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::Builder;

use crate::{
  AssetCatalog, FeatureSelection, WorkReport, browser::BrowserIndex, dependency::DependencyIndex,
  discovery::Package, output_index::OutputIndex, source_scan::SourceIndex,
};

const INDEX_SCHEMA: &str = "battlement-reactant-asset-index-v1";
const WASM_TARGET: &str = "wasm32-unknown-unknown";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct FileFingerprint {
  pub(crate) path: String,
  pub(crate) file_id: String,
  pub(crate) byte_length: u64,
  pub(crate) modified_nanoseconds: u64,
}

#[derive(Clone)]
pub(crate) struct ReusableGraph {
  pub(crate) host_target: String,
  pub(crate) host_packages: Vec<Package>,
  pub(crate) wasm_packages: Vec<Package>,
}

pub(crate) struct IncrementalIndex {
  path: PathBuf,
  original: Option<Vec<u8>>,
  state: IndexFile,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct IndexFile {
  schema: String,
  selection: SelectionRecord,
  graph: Option<GraphRecord>,
  sources: SourceIndex,
  dependencies: DependencyIndex,
  browser: BrowserIndex,
  outputs: OutputIndex,
  semantic_output_set_hash: String,
}

#[derive(Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct SelectionRecord {
  manifest: String,
  features: Vec<String>,
  all_features: bool,
  no_default_features: bool,
  host_target: String,
  wasm_target: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct GraphRecord {
  host_target: String,
  host_packages: Vec<Package>,
  wasm_packages: Vec<Package>,
  inputs: Vec<FileProbe>,
  environment: BTreeMap<String, Option<String>>,
  cargo_identity: ExecutableIdentity,
  generator_identity: ExecutableIdentity,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct FileProbe {
  path: String,
  fingerprint: Option<FileFingerprint>,
  content_hash: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ExecutableIdentity {
  path: String,
  fingerprint: Option<FileFingerprint>,
}

impl IncrementalIndex {
  pub(crate) fn load(
    project: &Path,
    manifest: &Path,
    features: &FeatureSelection,
    report: &mut WorkReport,
  ) -> Result<Self> {
    let selection = self::selection(project, manifest, features)?;
    let key = self::hash_json(&selection)?;
    let path = project
      .join("Library/BattlementReactant/asset-generator-state")
      .join(format!("{key}.json"));
    report.stat_calls += 1;
    let original = fs::read(&path).ok();
    if let Some(bytes) = &original {
      report.files_opened += 1;
      report.bytes_read += bytes.len() as u64;
    }
    let retained = original
      .as_deref()
      .and_then(|bytes| serde_json::from_slice::<IndexFile>(bytes).ok())
      .filter(|state| state.schema == INDEX_SCHEMA && state.selection == selection);
    Ok(Self {
      path,
      original,
      state: retained.unwrap_or_else(|| IndexFile {
        schema: INDEX_SCHEMA.to_owned(),
        selection,
        graph: None,
        sources: SourceIndex::default(),
        dependencies: DependencyIndex::default(),
        browser: BrowserIndex::default(),
        outputs: OutputIndex::default(),
        semantic_output_set_hash: String::new(),
      }),
    })
  }

  pub(crate) fn reusable_graph(&self, report: &mut WorkReport) -> Option<ReusableGraph> {
    let graph = self.state.graph.as_ref()?;
    if graph.host_target != self.state.selection.host_target {
      return None;
    }
    if graph.environment != self::resolution_environment() {
      return None;
    }
    if !graph
      .inputs
      .iter()
      .all(|probe| self::probe_matches(probe, report))
    {
      return None;
    }
    if !self::executable_matches(&graph.cargo_identity, report)
      || !self::executable_matches(&graph.generator_identity, report)
    {
      return None;
    }
    Some(ReusableGraph {
      host_target: graph.host_target.clone(),
      host_packages: graph.host_packages.clone(),
      wasm_packages: graph.wasm_packages.clone(),
    })
  }

  pub(crate) fn replace_graph(
    &mut self,
    project: &Path,
    manifest: &Path,
    graph: ReusableGraph,
    manifests: BTreeSet<PathBuf>,
    report: &mut WorkReport,
  ) -> Result<()> {
    let probe_paths = self::resolution_probe_paths(project, manifest, manifests);
    let inputs = probe_paths
      .into_iter()
      .map(|path| self::capture_probe(&path, report))
      .collect::<Result<Vec<_>>>()?;
    self.state.graph = Some(GraphRecord {
      host_target: graph.host_target,
      host_packages: graph.host_packages,
      wasm_packages: graph.wasm_packages,
      inputs,
      environment: self::resolution_environment(),
      cargo_identity: self::executable_identity(self::cargo_executable(), report),
      generator_identity: self::executable_identity(env::current_exe().ok(), report),
    });
    Ok(())
  }

  pub(crate) fn sources(&mut self) -> &mut SourceIndex {
    &mut self.state.sources
  }

  pub(crate) fn dependencies(&mut self) -> &mut DependencyIndex {
    &mut self.state.dependencies
  }

  pub(crate) fn render_state(&mut self) -> (&mut DependencyIndex, &mut BrowserIndex) {
    (&mut self.state.dependencies, &mut self.state.browser)
  }

  pub(crate) fn record_catalog(&mut self, catalog: &AssetCatalog) -> Result<()> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SemanticAsset<'a> {
      address: &'a str,
      guid: &'a str,
      request_identity: String,
      dependencies: Vec<SemanticDependency<'a>>,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SemanticDependency<'a> {
      path: &'a str,
      identity: String,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SemanticCatalog<'a> {
      assets: Vec<SemanticAsset<'a>>,
      directories: Vec<(&'a str, &'a str)>,
    }
    let assets = catalog
      .assets
      .iter()
      .map(|asset| SemanticAsset {
        address: &asset.address,
        guid: &asset.guid,
        request_identity: self::hex(&asset.request_identity),
        dependencies: asset
          .dependencies
          .iter()
          .map(|dependency| SemanticDependency {
            path: &dependency.path,
            identity: self::hex(&dependency.identity),
          })
          .collect(),
      })
      .collect::<Vec<_>>();
    let semantic = SemanticCatalog {
      assets,
      directories: catalog
        .directories
        .iter()
        .map(|directory| (directory.path.as_str(), directory.guid.as_str()))
        .collect(),
    };
    self.state.semantic_output_set_hash = self::hash_json(&semantic)?;
    Ok(())
  }

  pub(crate) fn refresh_outputs(&mut self, project: &Path, report: &mut WorkReport) -> Result<()> {
    self.state.outputs.refresh(project, report)
  }

  pub(crate) fn save(&mut self, report: &mut WorkReport) -> Result<()> {
    self.state.sources.retain_visited();
    self.state.dependencies.retain_visited();
    let mut bytes = serde_json::to_vec_pretty(&self.state)
      .context("failed to serialize Reactant asset discovery state")?;
    bytes.push(b'\n');
    if self.original.as_deref() == Some(bytes.as_slice()) {
      return Ok(());
    }
    let parent = self.path.parent().expect("index path has a parent");
    fs::create_dir_all(parent).with_context(|| {
      format!(
        "failed to create asset-generator state {}",
        parent.display()
      )
    })?;
    let mut temporary = Builder::new()
      .prefix(".reactant-asset-index-")
      .tempfile_in(parent)
      .with_context(|| {
        format!(
          "failed to stage asset-generator state {}",
          self.path.display()
        )
      })?;
    temporary.as_file_mut().write_all(&bytes)?;
    temporary.as_file_mut().sync_all()?;
    temporary
      .persist(&self.path)
      .map_err(|error| error.error)
      .with_context(|| {
        format!(
          "failed to install asset-generator state {}",
          self.path.display()
        )
      })?;
    report.files_written += 1;
    self.original = Some(bytes);
    Ok(())
  }
}

pub(crate) fn fingerprint(path: &Path, report: &mut WorkReport) -> Option<FileFingerprint> {
  report.stat_calls += 1;
  let metadata = fs::metadata(path).ok()?;
  let modified_nanoseconds = metadata
    .modified()
    .ok()?
    .duration_since(UNIX_EPOCH)
    .ok()?
    .as_nanos()
    .try_into()
    .ok()?;
  Some(FileFingerprint {
    path: self::normalized(path),
    file_id: self::file_id(&metadata),
    byte_length: metadata.len(),
    modified_nanoseconds,
  })
}

fn selection(
  project: &Path,
  manifest: &Path,
  features: &FeatureSelection,
) -> Result<SelectionRecord> {
  let mut selected_features = features.features.clone();
  selected_features.sort();
  selected_features.dedup();
  Ok(SelectionRecord {
    manifest: self::normalized(manifest.strip_prefix(project).with_context(|| {
      format!(
        "rules manifest {} is outside {}",
        manifest.display(),
        project.display()
      )
    })?),
    features: selected_features,
    all_features: features.all_features,
    no_default_features: features.no_default_features,
    host_target: self::target_hint(),
    wasm_target: WASM_TARGET.to_owned(),
  })
}

fn resolution_probe_paths(
  project: &Path,
  manifest: &Path,
  manifests: BTreeSet<PathBuf>,
) -> BTreeSet<PathBuf> {
  let mut paths = manifests;
  let manifest_dir = manifest.parent().expect("manifest path has a parent");
  for ancestor in manifest_dir.ancestors() {
    paths.insert(ancestor.join("Cargo.lock"));
    paths.insert(ancestor.join("rust-toolchain"));
    paths.insert(ancestor.join("rust-toolchain.toml"));
    paths.insert(ancestor.join(".cargo/config"));
    paths.insert(ancestor.join(".cargo/config.toml"));
  }
  paths.insert(project.join("Packages/manifest.json"));
  paths.insert(project.join("ProjectSettings/ProjectVersion.txt"));
  if let Some(cargo_home) = self::cargo_home() {
    paths.insert(cargo_home.join("config"));
    paths.insert(cargo_home.join("config.toml"));
  }
  paths
}

fn capture_probe(path: &Path, report: &mut WorkReport) -> Result<FileProbe> {
  let fingerprint = self::fingerprint(path, report);
  let content_hash = if fingerprint.is_some() {
    let bytes =
      fs::read(path).with_context(|| format!("failed to read graph input {}", path.display()))?;
    report.files_opened += 1;
    report.bytes_read += bytes.len() as u64;
    Some(self::hash(&bytes))
  } else {
    None
  };
  Ok(FileProbe {
    path: self::normalized(path),
    fingerprint,
    content_hash,
  })
}

fn probe_matches(probe: &FileProbe, report: &mut WorkReport) -> bool {
  self::fingerprint(Path::new(&probe.path), report) == probe.fingerprint
}

fn executable_identity(path: Option<PathBuf>, report: &mut WorkReport) -> ExecutableIdentity {
  let path = path.unwrap_or_default();
  ExecutableIdentity {
    fingerprint: self::fingerprint(&path, report),
    path: self::normalized(&path),
  }
}

fn executable_matches(identity: &ExecutableIdentity, report: &mut WorkReport) -> bool {
  self::fingerprint(Path::new(&identity.path), report) == identity.fingerprint
}

fn cargo_executable() -> Option<PathBuf> {
  env::var_os("CARGO").map(PathBuf::from).or_else(|| {
    env::var_os("PATH").and_then(|paths| {
      env::split_paths(&paths)
        .map(|path| path.join(if cfg!(windows) { "cargo.exe" } else { "cargo" }))
        .find(|path| path.is_file())
    })
  })
}

fn cargo_home() -> Option<PathBuf> {
  env::var_os("CARGO_HOME")
    .map(PathBuf::from)
    .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
}

fn resolution_environment() -> BTreeMap<String, Option<String>> {
  let mut values = env::vars()
    .filter(|(name, _)| {
      name.starts_with("CARGO_")
        || name.starts_with("RUST")
        || matches!(name.as_str(), "HTTP_PROXY" | "HTTPS_PROXY" | "NO_PROXY")
    })
    .map(|(name, value)| (name, Some(value)))
    .collect::<BTreeMap<_, _>>();
  for name in [
    "CARGO_HOME",
    "CARGO_BUILD_TARGET",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_NET_OFFLINE",
    "RUSTC",
    "RUSTFLAGS",
    "RUSTUP_TOOLCHAIN",
  ] {
    values.entry(name.to_owned()).or_insert(None);
  }
  values
}

fn target_hint() -> String {
  let architecture = env::consts::ARCH;
  if cfg!(target_os = "macos") {
    return format!("{architecture}-apple-darwin");
  }
  if cfg!(all(target_os = "windows", target_env = "gnu")) {
    return format!("{architecture}-pc-windows-gnu");
  }
  if cfg!(target_os = "windows") {
    return format!("{architecture}-pc-windows-msvc");
  }
  if cfg!(target_env = "musl") {
    return format!("{architecture}-unknown-linux-musl");
  }
  format!("{architecture}-unknown-linux-gnu")
}

fn file_id(metadata: &fs::Metadata) -> String {
  #[cfg(unix)]
  {
    use std::os::unix::fs::MetadataExt;
    return format!("{}:{}", metadata.dev(), metadata.ino());
  }
  #[cfg(windows)]
  {
    use std::os::windows::fs::MetadataExt;
    return metadata.file_index().unwrap_or_default().to_string();
  }
  #[allow(unreachable_code)]
  "unavailable".to_owned()
}

fn normalized(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
}

fn hash_json(value: &impl Serialize) -> Result<String> {
  Ok(self::hash(&serde_json::to_vec(value)?))
}

fn hash(bytes: &[u8]) -> String {
  self::hex(&Sha256::digest(bytes))
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
