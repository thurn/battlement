use std::{
  collections::{BTreeMap, BTreeSet},
  path::Path,
};

use crate::{AssetCatalog, WorkReport, browser, manifest, manifest_schema::AssetRecord};

const GENERATED_ROOT: &str = "Assets/Generated/BattlementReactant";

#[derive(Default)]
pub(crate) struct AssetDiagnostics {
  added: BTreeSet<String>,
  changed: BTreeSet<String>,
  corrupt: BTreeSet<String>,
  current: BTreeSet<String>,
  missing: BTreeSet<String>,
  stale: BTreeSet<String>,
}

impl AssetDiagnostics {
  pub(crate) fn current(catalog: &AssetCatalog) -> Self {
    Self {
      current: self::catalog_addresses(catalog),
      ..Self::default()
    }
  }

  pub(crate) fn current_count(&self) -> usize {
    self.current.len()
  }

  pub(crate) fn stale_count(&self) -> usize {
    let mut noncurrent = BTreeSet::new();
    for values in [
      &self.added,
      &self.changed,
      &self.corrupt,
      &self.missing,
      &self.stale,
    ] {
      noncurrent.extend(values.iter());
    }
    noncurrent.len()
  }

  pub(crate) fn is_clean(&self) -> bool {
    self.stale_count() == 0
  }

  pub(crate) fn emit(&self) {
    for (category, values) in [
      ("added", &self.added),
      ("changed", &self.changed),
      ("missing", &self.missing),
      ("corrupt", &self.corrupt),
      ("stale", &self.stale),
    ] {
      for address in values {
        println!("status={category} asset={address}");
      }
    }
  }
}

pub(crate) fn classify(
  project: &Path,
  catalog: &AssetCatalog,
  browser_stale: &BTreeSet<String>,
  report: &mut WorkReport,
) -> AssetDiagnostics {
  let mut output = AssetDiagnostics::default();
  let Ok((_, installed)) = manifest::read_authoritative(project, report) else {
    let manifest_path = project.join(GENERATED_ROOT).join("manifest.json");
    let target = if manifest_path.exists() {
      &mut output.corrupt
    } else {
      &mut output.added
    };
    target.extend(self::catalog_addresses(catalog));
    return output;
  };
  let installed_by_address = installed
    .assets
    .iter()
    .map(|record| (record.address.as_str(), record))
    .collect::<BTreeMap<_, _>>();
  let catalog_by_address = catalog
    .assets
    .iter()
    .map(|asset| (asset.address.as_str(), asset))
    .collect::<BTreeMap<_, _>>();
  let renderer_stale = installed.renderer_identity != browser::renderer_identity();
  for (address, asset) in &catalog_by_address {
    let Some(record) = installed_by_address.get(address) else {
      output.added.insert((*address).to_owned());
      continue;
    };
    if renderer_stale {
      output.stale.insert((*address).to_owned());
      continue;
    }
    if !manifest::record_matches(record, asset, &installed) {
      output.changed.insert((*address).to_owned());
      continue;
    }
    if self::asset_missing(project, record) {
      output.missing.insert((*address).to_owned());
      continue;
    }
    if manifest::validate_asset_output(project, record, asset, report).is_err() {
      output.corrupt.insert((*address).to_owned());
      continue;
    }
    if browser_stale.contains(*address) {
      output.stale.insert((*address).to_owned());
    } else {
      output.current.insert((*address).to_owned());
    }
  }
  for address in installed_by_address.keys() {
    if !catalog_by_address.contains_key(address) {
      output.stale.insert((*address).to_string());
    }
  }
  if output.is_clean() && manifest::validate(project, catalog, report).is_err() {
    output.current.clear();
    output.corrupt.extend(self::catalog_addresses(catalog));
  }
  output
}

fn asset_missing(project: &Path, record: &AssetRecord) -> bool {
  let png = project.join(GENERATED_ROOT).join(&record.png);
  !png.is_file() || !png.with_extension("png.meta").is_file()
}

fn catalog_addresses(catalog: &AssetCatalog) -> BTreeSet<String> {
  catalog
    .assets
    .iter()
    .map(|asset| asset.address.clone())
    .collect()
}
