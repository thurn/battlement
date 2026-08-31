use std::{
  collections::{BTreeMap, BTreeSet, VecDeque},
  path::{Path, PathBuf},
  process::Command,
};

use anyhow::{Context, Result, bail};
use battlement_reactant_asset_syntax::AssetRequest;
use serde::{Deserialize, Serialize};

use crate::{
  FeatureSelection, WorkReport,
  incremental::{IncrementalIndex, ReusableGraph},
};

const REACTANT_PACKAGE: &str = "battlement-reactant";
const REACTANT_CRATE: &str = "battlement_reactant";
const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// One declaration found without expanding or executing the rules package.
#[derive(Clone, Debug, PartialEq)]
pub struct DiscoveredAsset {
  /// Portable package coordinate followed by the Rust module and static name.
  pub source_symbol: String,
  /// Canonical package coordinate used to compare target graphs.
  pub package: String,
  /// Source file containing the declaration.
  pub source_file: PathBuf,
  /// Fully validated request parsed by the shared syntax crate.
  pub request: AssetRequest,
}

/// Portable declaration catalog shared by host and WebAssembly builds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Discovery {
  /// Declarations in stable package-and-symbol order.
  pub assets: Vec<DiscoveredAsset>,
}

#[derive(Deserialize)]
struct Metadata {
  packages: Vec<Package>,
  resolve: Option<Resolve>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Package {
  pub(crate) id: String,
  pub(crate) name: String,
  pub(crate) version: String,
  pub(crate) source: Option<String>,
  pub(crate) manifest_path: PathBuf,
  pub(crate) targets: Vec<Target>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Target {
  pub(crate) name: String,
  pub(crate) kind: Vec<String>,
  pub(crate) src_path: PathBuf,
}

#[derive(Deserialize)]
struct Resolve {
  nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
  id: String,
  deps: Vec<NodeDependency>,
}

#[derive(Deserialize)]
struct NodeDependency {
  name: String,
  pkg: String,
}

struct GraphDiscovery {
  packages: Vec<String>,
  assets: Vec<DiscoveredAsset>,
}

struct GraphResolution {
  packages: Vec<Package>,
  manifests: BTreeSet<PathBuf>,
}

pub(crate) fn discover(
  manifest: &Path,
  project: &Path,
  features: &FeatureSelection,
  index: &mut IncrementalIndex,
  report: &mut WorkReport,
) -> Result<Discovery> {
  index.sources().begin_run();
  let (host_target, host_packages, wasm_packages) =
    if let Some(graph) = index.reusable_graph(report) {
      (graph.host_target, graph.host_packages, graph.wasm_packages)
    } else {
      let host_target = self::host_target(report)?;
      let host = self::resolve_graph("host", &host_target, manifest, features, report)?;
      let wasm = self::resolve_graph("WebAssembly", WASM_TARGET, manifest, features, report)?;
      let manifests = host.manifests.union(&wasm.manifests).cloned().collect();
      index.replace_graph(
        project,
        manifest,
        ReusableGraph {
          host_target: host_target.clone(),
          host_packages: host.packages.clone(),
          wasm_packages: wasm.packages.clone(),
        },
        manifests,
        report,
      )?;
      (host_target, host.packages, wasm.packages)
    };
  let host = self::scan_graph(&host_packages, project, index, report)?;
  let wasm_packages = self::package_coordinates(&wasm_packages, project)?;
  if host.packages != wasm_packages {
    bail!(
      "host ({host_target}) and WebAssembly ({WASM_TARGET}) reachable declaration packages differ: host={:?}; WebAssembly={:?}",
      host.packages,
      wasm_packages
    );
  }
  Ok(Discovery {
    assets: host.assets,
  })
}

fn host_target(report: &mut WorkReport) -> Result<String> {
  report.subprocesses_started += 1;
  let output = Command::new("rustc")
    .arg("-vV")
    .output()
    .context("failed to query the host Rust target")?;
  if !output.status.success() {
    bail!(
      "rustc -vV failed: {}",
      String::from_utf8_lossy(&output.stderr).trim()
    );
  }
  String::from_utf8(output.stdout)?
    .lines()
    .find_map(|line| line.strip_prefix("host: ").map(str::to_owned))
    .context("rustc -vV did not report a host target")
}

fn resolve_graph(
  origin: &str,
  target: &str,
  manifest: &Path,
  features: &FeatureSelection,
  report: &mut WorkReport,
) -> Result<GraphResolution> {
  let metadata = self::metadata(target, manifest, features, report)
    .with_context(|| format!("failed to resolve the {origin} Cargo graph ({target})"))?;
  let root = metadata
    .packages
    .iter()
    .find(|package| package.manifest_path == manifest)
    .with_context(|| {
      format!(
        "{origin} graph does not contain selected manifest {}",
        manifest.display()
      )
    })?;
  let nodes = metadata
    .resolve
    .as_ref()
    .context("Cargo metadata omitted its dependency graph")?
    .nodes
    .iter()
    .map(|node| (node.id.as_str(), node))
    .collect::<BTreeMap<_, _>>();
  let package_map = metadata
    .packages
    .iter()
    .map(|package| (package.id.as_str(), package))
    .collect::<BTreeMap<_, _>>();
  let reachable = self::reachable_packages(&root.id, &nodes)?;
  let mut candidates = Vec::new();
  for id in reachable {
    let package = package_map
      .get(id.as_str())
      .with_context(|| format!("Cargo graph references missing package {id}"))?;
    let node = nodes
      .get(id.as_str())
      .with_context(|| format!("Cargo graph omits dependency node {id}"))?;
    let reactant_dependencies = node
      .deps
      .iter()
      .filter(|dependency| {
        package_map
          .get(dependency.pkg.as_str())
          .is_some_and(|dependency_package| dependency_package.name == REACTANT_PACKAGE)
      })
      .collect::<Vec<_>>();
    if let Some(alias) = reactant_dependencies
      .iter()
      .find(|dependency| dependency.name != REACTANT_CRATE)
    {
      bail!(
        "package {} aliases {REACTANT_PACKAGE} as {}; declarations require the exact battlement_reactant::asset_generator::generate! path",
        package.name,
        alias.name
      );
    }
    if !reactant_dependencies.is_empty() {
      candidates.push(*package);
    }
  }
  Ok(GraphResolution {
    packages: candidates.into_iter().cloned().collect(),
    manifests: metadata
      .packages
      .into_iter()
      .map(|package| package.manifest_path)
      .collect(),
  })
}

fn scan_graph(
  candidates: &[Package],
  project: &Path,
  index: &mut IncrementalIndex,
  report: &mut WorkReport,
) -> Result<GraphDiscovery> {
  let mut packages = Vec::new();
  let mut assets = Vec::new();
  for package in candidates {
    let coordinate = self::coordinate(package, project)?;
    packages.push(coordinate.clone());
    crate::source_scan::scan_package(package, &coordinate, index.sources(), report, &mut assets)?;
  }
  packages.sort();
  packages.dedup();
  assets.sort_by(|left, right| left.source_symbol.cmp(&right.source_symbol));
  let mut symbols = BTreeSet::new();
  if let Some(duplicate) = assets
    .iter()
    .find(|asset| !symbols.insert(asset.source_symbol.clone()))
  {
    bail!(
      "duplicate discovered asset symbol {}",
      duplicate.source_symbol
    );
  }
  Ok(GraphDiscovery { packages, assets })
}

fn package_coordinates(candidates: &[Package], project: &Path) -> Result<Vec<String>> {
  let mut packages = candidates
    .iter()
    .map(|package| self::coordinate(package, project))
    .collect::<Result<Vec<_>>>()?;
  packages.sort();
  packages.dedup();
  Ok(packages)
}

fn metadata(
  target: &str,
  manifest: &Path,
  features: &FeatureSelection,
  report: &mut WorkReport,
) -> Result<Metadata> {
  let mut command = Command::new("cargo");
  command.args(["metadata", "--format-version", "1", "--manifest-path"]);
  command.arg(manifest).args(["--filter-platform", target]);
  if features.all_features {
    command.arg("--all-features");
  }
  if features.no_default_features {
    command.arg("--no-default-features");
  }
  if !features.features.is_empty() {
    command.arg("--features").arg(features.features.join(","));
  }
  report.cargo_metadata_runs += 1;
  report.subprocesses_started += 1;
  let output = command.output().context("failed to run Cargo metadata")?;
  if !output.status.success() {
    bail!(
      "Cargo metadata failed: {}",
      String::from_utf8_lossy(&output.stderr).trim()
    );
  }
  serde_json::from_slice(&output.stdout).context("Cargo metadata returned invalid JSON")
}

fn reachable_packages(root: &str, nodes: &BTreeMap<&str, &Node>) -> Result<BTreeSet<String>> {
  let mut reachable = BTreeSet::new();
  let mut pending = VecDeque::from([root.to_owned()]);
  while let Some(id) = pending.pop_front() {
    if !reachable.insert(id.clone()) {
      continue;
    }
    let node = nodes
      .get(id.as_str())
      .with_context(|| format!("Cargo graph omits dependency node {id}"))?;
    pending.extend(node.deps.iter().map(|dependency| dependency.pkg.clone()));
  }
  Ok(reachable)
}

fn coordinate(package: &Package, project: &Path) -> Result<String> {
  if let Some(source) = &package.source {
    return Ok(format!("{source}#{}@{}", package.name, package.version));
  }
  let manifest = package.manifest_path.canonicalize().with_context(|| {
    format!(
      "failed to resolve package manifest {}",
      package.manifest_path.display()
    )
  })?;
  let relative = manifest.strip_prefix(project).with_context(|| {
    format!(
      "local package {} at {} is outside Unity project {} and has no portable coordinate",
      package.name,
      manifest.display(),
      project.display()
    )
  })?;
  Ok(format!(
    "path:{}#{}@{}",
    relative.to_string_lossy().replace('\\', "/"),
    package.name,
    package.version
  ))
}

#[cfg(test)]
mod tests {
  use super::{Package, coordinate};

  #[test]
  fn registry_and_git_packages_keep_their_cargo_coordinates() {
    for source in [
      "registry+https://github.com/rust-lang/crates.io-index",
      "git+https://example.invalid/assets.git?rev=main#0123456789abcdef",
    ] {
      let package = Package {
        id: "fixture".to_owned(),
        name: "portable-assets".to_owned(),
        version: "1.2.3".to_owned(),
        source: Some(source.to_owned()),
        manifest_path: "unused/Cargo.toml".into(),
        targets: Vec::new(),
      };

      assert_eq!(
        coordinate(&package, "/unused".as_ref()).unwrap(),
        format!("{source}#portable-assets@1.2.3")
      );
    }
  }
}
