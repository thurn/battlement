use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result, bail};
use battlement_reactant_asset_syntax::{AssetRequest, DependencyKind};
use sha2::{Digest, Sha256};

use crate::{Discovery, WorkReport, dependency::DependencyIndex};

const ADDRESS_PREFIX: &str = "battlement-reactant/generated/";
const GENERATED_ROOT: &str = "Assets/Generated/BattlementReactant";

/// One validated project dependency and its normalized content identity.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DependencyIdentity {
  /// Dependency family expected by the declaration.
  pub kind: DependencyKind,
  /// Canonical Unity-project-relative path.
  pub path: String,
  /// SHA-256 of decoded, domain-separated dependency contents.
  pub identity: [u8; 32],
}

/// Deterministic metadata identity for one generated directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryIdentity {
  /// Unity-project-relative generated directory.
  pub path: String,
  /// Lowercase 128-bit Unity GUID.
  pub guid: String,
}

/// One unique render request with all equivalent declaration origins.
#[derive(Clone, Debug, PartialEq)]
pub struct CatalogAsset {
  /// Canonical request bytes used only for output-cache identity.
  pub canonical_request: Vec<u8>,
  /// Stable public Addressables address, independent of dependency bytes.
  pub address: String,
  /// Lowercase deterministic Unity GUID for the generated PNG.
  pub guid: String,
  /// Canonical request identity.
  pub request_identity: [u8; 32],
  /// Effective number of raster pixels per logical unit.
  pub raster_scale: u8,
  /// Complete typed request used by the renderer.
  pub request: AssetRequest,
  /// Validated dependency records in canonical request order.
  pub dependencies: Vec<DependencyIdentity>,
  /// All declarations deduplicated into this output.
  pub source_symbols: Vec<String>,
}

/// Validated and deduplicated identity table used by later render stages.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AssetCatalog {
  /// Unique generated outputs in stable address order.
  pub assets: Vec<CatalogAsset>,
  /// Domain-separated metadata identities for generated directories.
  pub directories: Vec<DirectoryIdentity>,
}

struct PendingAsset {
  canonical: Vec<u8>,
  asset: CatalogAsset,
}

pub(crate) fn resolve(
  discovery: &Discovery,
  project: &Path,
  index: &mut DependencyIndex,
  report: &mut WorkReport,
) -> Result<AssetCatalog> {
  let project = project
    .canonicalize()
    .with_context(|| format!("failed to resolve Unity project {}", project.display()))?;
  let mut requests = BTreeMap::<[u8; 32], PendingAsset>::new();
  index.begin_run();
  for declaration in &discovery.assets {
    let canonical = declaration.request.canonical_bytes();
    let request_identity = declaration.request.identity();
    let dependencies = declaration
      .request
      .dependencies
      .iter()
      .map(|dependency| {
        crate::dependency::resolve(dependency, &declaration.request, &project, index, report)
      })
      .collect::<Result<Vec<_>>>()?;
    let address = self::address(request_identity);
    self::validate_address(&address)?;
    self::merge_request(
      &mut requests,
      request_identity,
      canonical,
      CatalogAsset {
        canonical_request: declaration.request.canonical_bytes(),
        guid: self::guid(b"reactant-asset\0", address.as_bytes()),
        address,
        request_identity,
        raster_scale: declaration.request.metadata.raster_scale,
        request: declaration.request.clone(),
        dependencies,
        source_symbols: vec![declaration.source_symbol.clone()],
      },
    )?;
  }
  let mut assets = requests
    .into_values()
    .map(|pending| pending.asset)
    .collect::<Vec<_>>();
  assets.sort_by(|left, right| left.address.cmp(&right.address));
  let directories = if assets.is_empty() {
    Vec::new()
  } else {
    [
      GENERATED_ROOT,
      concat!("Assets/Generated/BattlementReactant", "/Resources"),
      concat!("Assets/Generated/BattlementReactant", "/textures"),
    ]
    .into_iter()
    .map(|path| DirectoryIdentity {
      path: path.to_owned(),
      guid: self::guid(b"reactant-directory\0", path.as_bytes()),
    })
    .collect()
  };
  Ok(AssetCatalog {
    assets,
    directories,
  })
}

fn merge_request(
  requests: &mut BTreeMap<[u8; 32], PendingAsset>,
  identity: [u8; 32],
  canonical: Vec<u8>,
  asset: CatalogAsset,
) -> Result<()> {
  let Some(existing) = requests.get_mut(&identity) else {
    requests.insert(identity, PendingAsset { canonical, asset });
    return Ok(());
  };
  if existing.canonical != canonical {
    bail!(
      "canonical request hash collision between {} and {} at {}",
      existing.asset.source_symbols.join(", "),
      asset.source_symbols.join(", "),
      asset.address
    );
  }
  if existing.asset.dependencies != asset.dependencies {
    bail!(
      "canonical request {} resolves different dependency bytes for {} and {}",
      self::hex(&identity),
      existing.asset.source_symbols.join(", "),
      asset.source_symbols.join(", ")
    );
  }
  existing.asset.source_symbols.extend(asset.source_symbols);
  Ok(())
}

fn address(identity: [u8; 32]) -> String {
  format!("{ADDRESS_PREFIX}{}.png", self::hex(&identity))
}

fn validate_address(address: &str) -> Result<()> {
  let identity = address
    .strip_prefix(ADDRESS_PREFIX)
    .and_then(|value| value.strip_suffix(".png"));
  let valid_length = identity.is_some_and(|value| value.len() == 64);
  let valid_digits = identity.is_some_and(|value| {
    value
      .bytes()
      .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
  });
  if !valid_length || !valid_digits {
    bail!("generated address {address} conflicts with the reserved namespace contract");
  }
  Ok(())
}

fn guid(domain: &[u8], value: &[u8]) -> String {
  let mut hash = Sha256::new();
  hash.update(domain);
  hash.update(value);
  self::hex(&hash.finalize()[..16])
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

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;

  use battlement_reactant_asset_syntax::DependencyKind;

  use super::{CatalogAsset, DependencyIdentity, PendingAsset, merge_request, validate_address};

  #[test]
  fn conflict_diagnostics_distinguish_hash_and_dependency_collisions() {
    let identity = [7; 32];
    let mut requests = BTreeMap::<[u8; 32], PendingAsset>::new();
    merge_request(
      &mut requests,
      identity,
      vec![1],
      asset(identity, [1; 32], "first"),
    )
    .unwrap();

    let hash_collision = merge_request(
      &mut requests,
      identity,
      vec![2],
      asset(identity, [1; 32], "second"),
    )
    .unwrap_err()
    .to_string();
    assert!(hash_collision.contains("canonical request hash collision"));
    assert!(hash_collision.contains("first"));
    assert!(hash_collision.contains("second"));

    let dependency_collision = merge_request(
      &mut requests,
      identity,
      vec![1],
      asset(identity, [2; 32], "second"),
    )
    .unwrap_err()
    .to_string();
    assert!(dependency_collision.contains("resolves different dependency bytes"));
    assert!(dependency_collision.contains("first"));
    assert!(dependency_collision.contains("second"));
  }

  #[test]
  fn reserved_address_diagnostic_names_the_conflicting_value() {
    let address = "battlement-reactant/generated/not-a-hash.png";
    assert!(
      validate_address(address)
        .unwrap_err()
        .to_string()
        .contains(address)
    );
  }

  fn asset(identity: [u8; 32], dependency: [u8; 32], source: &str) -> CatalogAsset {
    CatalogAsset {
      canonical_request: vec![1],
      address: format!("battlement-reactant/generated/{}.png", "0".repeat(64)),
      guid: "0".repeat(32),
      request_identity: identity,
      raster_scale: 2,
      request: battlement_reactant_asset_syntax::parse(
        "@background TEST { @canvas 2px 2px; background: linear-gradient(red, blue); }",
      )
      .unwrap(),
      dependencies: vec![DependencyIdentity {
        kind: DependencyKind::Image,
        path: "Assets/a.png".to_owned(),
        identity: dependency,
      }],
      source_symbols: vec![source.to_owned()],
    }
  }
}
