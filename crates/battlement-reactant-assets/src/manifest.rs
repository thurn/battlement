use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::Path,
};

use anyhow::{Context, Result, bail};
use battlement_reactant_asset_syntax::{
  Compression, DeclarationKind, DependencyKind, FilterMode, GeneratorMetadata, WrapMode,
};
use sha2::{Digest, Sha256};

use crate::{
  AssetCatalog, CatalogAsset, WorkReport,
  browser::{self, BrowserRequest, BrowserRun},
  incremental::fingerprint,
  manifest_schema::{
    AssetRecord, BrowserRecord, DependencyRecord, FileIdentityRecord, ImportRecord,
    LogicalSizeRecord, Manifest, RasterSizeRecord, Sidecar, SliceInsetsRecord, SubjectBoundsRecord,
  },
  manifest_validation as validation, unity_metadata,
};

const GENERATED_ROOT: &str = "Assets/Generated/BattlementReactant";
const MANIFEST_PATH: &str = "Assets/Generated/BattlementReactant/manifest.json";
const RESOURCES_PATH: &str = "Assets/Generated/BattlementReactant/Resources";
const SIDECAR_NAME: &str = "BattlementReactantAssetCatalog.json";
const TEXTURES_PATH: &str = "Assets/Generated/BattlementReactant/textures";

pub(crate) fn install(
  project: &Path,
  catalog: &AssetCatalog,
  browser: &BrowserRun,
  report: &mut WorkReport,
) -> Result<()> {
  let set = self::build(project, catalog, browser, report)?;
  let root = project.join(GENERATED_ROOT);
  let root_meta = project.join(format!("{GENERATED_ROOT}.meta"));
  if root.exists() {
    fs::remove_dir_all(&root)
      .with_context(|| format!("failed to replace generated root {}", root.display()))?;
    report.files_written += 1;
  }
  if root_meta.exists() {
    fs::remove_file(&root_meta).with_context(|| {
      format!(
        "failed to replace generated metadata {}",
        root_meta.display()
      )
    })?;
    report.files_written += 1;
  }
  for directory in &set.directories {
    fs::create_dir_all(project.join(directory))
      .with_context(|| format!("failed to create generated directory {directory}"))?;
  }
  for (path, bytes) in set.files {
    fs::write(project.join(&path), bytes)
      .with_context(|| format!("failed to write generated asset {path}"))?;
    report.files_written += 1;
  }
  self::validate(project, catalog, report)
}

pub(crate) fn validate(
  project: &Path,
  catalog: &AssetCatalog,
  report: &mut WorkReport,
) -> Result<()> {
  let manifest_bytes = validation::read(project, MANIFEST_PATH, report)?;
  let manifest = validation::canonical::<Manifest>(&manifest_bytes, "manifest")?;
  self::validate_browser(&manifest.browser, report)?;
  if manifest.renderer_identity != browser::renderer_identity() {
    bail!("generated manifest renderer identity is stale");
  }
  if manifest.assets.len() != catalog.assets.len() {
    bail!("generated manifest asset set does not match discovered declarations");
  }
  let addresses = manifest
    .assets
    .iter()
    .map(|asset| asset.address.clone())
    .collect::<Vec<_>>();
  if !validation::strictly_sorted(&addresses) {
    bail!("generated manifest assets are not sorted by unique address");
  }
  let mut expected_paths = validation::base_paths(catalog);
  let mut derivations = BTreeMap::new();
  for (record, asset) in manifest.assets.iter().zip(&catalog.assets) {
    self::validate_record(record, asset, &manifest, &mut derivations)?;
    let png_path = format!("{GENERATED_ROOT}/{}", record.png);
    let png = validation::read(project, &png_path, report)?;
    report.generated_png_opens += 1;
    let normalized = crate::png_output::normalize(asset, &png)?;
    if normalized.bytes != png || normalized.sha256 != record.png_sha256 {
      bail!(
        "generated PNG {} is corrupt or nondeterministic",
        record.png
      );
    }
    if normalized.width != record.raster_size.width
      || normalized.height != record.raster_size.height
    {
      bail!("generated PNG {} has stale raster dimensions", record.png);
    }
    let metadata_path = format!("{png_path}.meta");
    let metadata = validation::read(project, &metadata_path, report)?;
    unity_metadata::validate_texture(
      &metadata,
      &record.unity_guid,
      asset.request.metadata.filter_mode,
      asset.request.metadata.wrap_mode,
      asset.request.metadata.compression,
    )?;
    expected_paths.insert(record.png.clone());
    expected_paths.insert(format!("{}.meta", record.png));
  }
  self::validate_nontexture_metadata(project, catalog, report, &mut derivations)?;
  let manifest_hash = validation::hex(&Sha256::digest(&manifest_bytes));
  let sidecar_path = format!("{RESOURCES_PATH}/{SIDECAR_NAME}");
  let sidecar_bytes = validation::read(project, &sidecar_path, report)?;
  let sidecar = validation::canonical::<Sidecar>(&sidecar_bytes, "runtime sidecar")?;
  if sidecar.addresses != addresses || sidecar.manifest_sha256 != manifest_hash {
    bail!("generated runtime sidecar does not match the authoritative manifest");
  }
  validation::validate_hash(&sidecar.manifest_sha256, 64, "sidecar manifest hash")?;
  validation::validate_tree(project, &expected_paths)?;
  Ok(())
}

fn build(
  project: &Path,
  catalog: &AssetCatalog,
  run: &BrowserRun,
  report: &mut WorkReport,
) -> Result<validation::GeneratedSet> {
  let requests = run
    .requests
    .iter()
    .map(|request| (request.address.as_str(), request))
    .collect::<BTreeMap<_, _>>();
  if requests.len() != catalog.assets.len() {
    bail!("browser result set does not match the generated asset catalog");
  }
  let browser = self::browser_record(run);
  let mut files = BTreeMap::new();
  let mut assets = Vec::new();
  let mut derivations = BTreeMap::new();
  for asset in &catalog.assets {
    let request = requests
      .get(asset.address.as_str())
      .with_context(|| format!("browser omitted {}", asset.address))?;
    let record = self::asset_record(asset, request);
    validation::record_derivation(
      &mut derivations,
      &record.unity_guid,
      &record.unity_guid_derivation_sha256,
      &record.address,
    )?;
    let cache_path = browser::cached_png_path(project, &request.cache_key);
    let png = fs::read(&cache_path)
      .with_context(|| format!("failed to read rendered PNG cache {}", cache_path.display()))?;
    report.files_opened += 1;
    report.generated_png_opens += 1;
    report.bytes_read += png.len() as u64;
    if validation::hex(&Sha256::digest(&png)) != request.image_hash {
      bail!("rendered PNG cache changed for {}", asset.address);
    }
    let png_path = format!(
      "{TEXTURES_PATH}/{}.png",
      validation::hex(&asset.request_identity)
    );
    files.insert(png_path.clone(), png);
    files.insert(
      format!("{png_path}.meta"),
      unity_metadata::texture(
        &asset.guid,
        asset.request.metadata.filter_mode,
        asset.request.metadata.wrap_mode,
        asset.request.metadata.compression,
      ),
    );
    assets.push(record);
  }
  let manifest = Manifest {
    assets,
    browser,
    renderer_identity: run.renderer_identity.clone(),
  };
  let manifest_bytes = validation::canonical_bytes(&manifest)?;
  let sidecar = Sidecar {
    addresses: manifest
      .assets
      .iter()
      .map(|asset| asset.address.clone())
      .collect(),
    manifest_sha256: validation::hex(&Sha256::digest(&manifest_bytes)),
  };
  let sidecar_bytes = validation::canonical_bytes(&sidecar)?;
  let sidecar_path = format!("{RESOURCES_PATH}/{SIDECAR_NAME}");
  for path in [MANIFEST_PATH, sidecar_path.as_str()] {
    let derivation = validation::derivation(b"reactant-file\0", path.as_bytes());
    validation::record_derivation(
      &mut derivations,
      &validation::file_guid(path),
      &validation::hex(&derivation),
      path,
    )?;
  }
  files.insert(MANIFEST_PATH.to_owned(), manifest_bytes);
  files.insert(
    format!("{MANIFEST_PATH}.meta"),
    unity_metadata::text(&validation::file_guid(MANIFEST_PATH)),
  );
  files.insert(sidecar_path.clone(), sidecar_bytes);
  files.insert(
    format!("{sidecar_path}.meta"),
    unity_metadata::text(&validation::file_guid(&sidecar_path)),
  );
  for directory in &catalog.directories {
    let derivation = validation::derivation(b"reactant-directory\0", directory.path.as_bytes());
    validation::record_derivation(
      &mut derivations,
      &directory.guid,
      &validation::hex(&derivation),
      &directory.path,
    )?;
    let metadata = if directory.path == GENERATED_ROOT {
      format!("{GENERATED_ROOT}.meta")
    } else {
      format!("{}.meta", directory.path)
    };
    files.insert(metadata, unity_metadata::directory(&directory.guid));
  }
  let directories = catalog
    .directories
    .iter()
    .map(|directory| directory.path.clone())
    .collect::<BTreeSet<_>>();
  let set = validation::GeneratedSet { directories, files };
  validation::validate_built_set(&set, catalog)?;
  Ok(set)
}

fn asset_record(asset: &CatalogAsset, request: &BrowserRequest) -> AssetRecord {
  let metadata = &asset.request.metadata;
  let request_hash = validation::hex(&asset.request_identity);
  let derivation = validation::derivation(b"reactant-asset\0", asset.address.as_bytes());
  AssetRecord {
    address: asset.address.clone(),
    cache_key: request.cache_key.clone(),
    canonical_request_sha256: request_hash.clone(),
    dependencies: asset
      .dependencies
      .iter()
      .map(|dependency| DependencyRecord {
        content_sha256: validation::hex(&dependency.identity),
        kind: match dependency.kind {
          DependencyKind::Font => "font",
          DependencyKind::Image => "image",
        }
        .to_owned(),
        path: dependency.path.clone(),
      })
      .collect(),
    import: self::import_record(metadata),
    kind: match asset.request.kind {
      DeclarationKind::Background => "background",
      DeclarationKind::NineSlice => "nineSlice",
      DeclarationKind::TextImage => "textImage",
    }
    .to_owned(),
    logical_canvas: LogicalSizeRecord {
      height: metadata.canvas.height,
      width: metadata.canvas.width,
    },
    png: format!("textures/{request_hash}.png"),
    png_sha256: request.image_hash.clone(),
    raster_scale: metadata.raster_scale,
    raster_size: RasterSizeRecord {
      height: request.height,
      width: request.width,
    },
    slice_insets: metadata.slices.map(|insets| SliceInsetsRecord {
      bottom: insets.bottom,
      left: insets.left,
      right: insets.right,
      top: insets.top,
    }),
    subject_bounds: SubjectBoundsRecord {
      height: metadata.subject.height,
      width: metadata.subject.width,
      x: metadata.subject.x,
      y: metadata.subject.y,
    },
    unity_guid: asset.guid.clone(),
    unity_guid_derivation_sha256: validation::hex(&derivation),
  }
}

fn browser_record(run: &BrowserRun) -> BrowserRecord {
  BrowserRecord {
    executable_file_identity: FileIdentityRecord {
      byte_length: run.executable_fingerprint.byte_length,
      file_id: run.executable_fingerprint.file_id.clone(),
      modified_nanoseconds: run.executable_fingerprint.modified_nanoseconds,
    },
    executable_path: run.executable_path.clone(),
    executable_sha256: run.executable_sha256.clone(),
    product: run.product.clone(),
    version: run.version.clone(),
  }
}

fn import_record(metadata: &GeneratorMetadata) -> ImportRecord {
  ImportRecord {
    alpha_is_transparency: true,
    compression: match metadata.compression {
      Compression::Lossless => "lossless",
      Compression::LossyLow => "lossyLow",
      Compression::LossyNormal => "lossyNormal",
      Compression::LossyHigh => "lossyHigh",
    }
    .to_owned(),
    filter_mode: match metadata.filter_mode {
      FilterMode::Bilinear => "bilinear",
      FilterMode::Nearest => "nearest",
    }
    .to_owned(),
    mipmaps: false,
    s_rgb: true,
    texture_type: "default".to_owned(),
    wrap_mode: match metadata.wrap_mode {
      WrapMode::Clamp => "clamp",
      WrapMode::Repeat => "repeat",
    }
    .to_owned(),
  }
}

fn validate_record(
  record: &AssetRecord,
  asset: &CatalogAsset,
  manifest: &Manifest,
  derivations: &mut BTreeMap<String, (String, String)>,
) -> Result<()> {
  let expected_hash = validation::hex(&asset.request_identity);
  let expected = self::asset_record(
    asset,
    &BrowserRequest {
      address: asset.address.clone(),
      cache_key: browser::manifest_cache_key(
        asset,
        &manifest.browser.executable_sha256,
        &manifest.browser.product,
        &manifest.browser.version,
        &manifest.renderer_identity,
      ),
      image_hash: record.png_sha256.clone(),
      width: validation::raster_dimension(asset.request.metadata.canvas.width, asset.raster_scale)?,
      height: validation::raster_dimension(
        asset.request.metadata.canvas.height,
        asset.raster_scale,
      )?,
      alpha: crate::png_output::AlphaBounds {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
      },
      warnings: Vec::new(),
    },
  );
  let actual = validation::canonical_bytes(record)?;
  let expected = validation::canonical_bytes(&expected)?;
  if actual != expected {
    bail!("generated manifest record for {} is stale", asset.address);
  }
  validation::validate_hash(
    &record.canonical_request_sha256,
    64,
    "canonical request hash",
  )?;
  validation::validate_hash(&record.cache_key, 64, "cache key")?;
  validation::validate_hash(&record.png_sha256, 64, "PNG hash")?;
  validation::validate_hash(&record.unity_guid, 32, "Unity GUID")?;
  validation::validate_hash(
    &record.unity_guid_derivation_sha256,
    64,
    "Unity GUID derivation",
  )?;
  if record.png != format!("textures/{expected_hash}.png") {
    bail!("generated PNG path does not agree with its canonical request hash");
  }
  for dependency in &record.dependencies {
    validation::validate_hash(&dependency.content_sha256, 64, "dependency hash")?;
    validation::validate_path(&dependency.path)?;
  }
  for value in [
    record.logical_canvas.height,
    record.logical_canvas.width,
    record.subject_bounds.height,
    record.subject_bounds.width,
    record.subject_bounds.x,
    record.subject_bounds.y,
  ] {
    validation::validate_number(value)?;
  }
  if let Some(insets) = &record.slice_insets {
    for value in [insets.bottom, insets.left, insets.right, insets.top] {
      validation::validate_number(value)?;
    }
  }
  validation::record_derivation(
    derivations,
    &record.unity_guid,
    &record.unity_guid_derivation_sha256,
    &record.address,
  )
}

fn validate_browser(browser: &BrowserRecord, report: &mut WorkReport) -> Result<()> {
  if browser.executable_path.is_empty() || browser.product.is_empty() || browser.version.is_empty()
  {
    bail!("generated manifest browser strings must not be empty");
  }
  validation::validate_hash(&browser.executable_sha256, 64, "browser executable hash")?;
  if !validation::valid_file_id(&browser.executable_file_identity.file_id) {
    bail!("generated manifest browser file identity is invalid");
  }
  let current = fingerprint(Path::new(&browser.executable_path), report)
    .context("generated manifest browser executable is unavailable")?;
  let expected = &browser.executable_file_identity;
  if current.byte_length != expected.byte_length
    || current.file_id != expected.file_id
    || current.modified_nanoseconds != expected.modified_nanoseconds
  {
    bail!("generated manifest browser executable identity is stale");
  }
  Ok(())
}

fn validate_nontexture_metadata(
  project: &Path,
  catalog: &AssetCatalog,
  report: &mut WorkReport,
  derivations: &mut BTreeMap<String, (String, String)>,
) -> Result<()> {
  let expected_directories = [GENERATED_ROOT, RESOURCES_PATH, TEXTURES_PATH];
  if catalog.directories.len() != expected_directories.len() {
    bail!("generated directory identity set is incomplete");
  }
  for (directory, expected_path) in catalog.directories.iter().zip(expected_directories) {
    if directory.path != expected_path {
      bail!("generated directory identity path is stale");
    }
    let derivation = validation::hex(&validation::derivation(
      b"reactant-directory\0",
      directory.path.as_bytes(),
    ));
    validation::record_derivation(derivations, &directory.guid, &derivation, &directory.path)?;
    let path = if directory.path == GENERATED_ROOT {
      format!("{GENERATED_ROOT}.meta")
    } else {
      format!("{}.meta", directory.path)
    };
    unity_metadata::validate_directory(
      &validation::read(project, &path, report)?,
      &directory.guid,
    )?;
  }
  for path in [
    MANIFEST_PATH.to_owned(),
    format!("{RESOURCES_PATH}/{SIDECAR_NAME}"),
  ] {
    let guid = validation::file_guid(&path);
    let derivation = validation::hex(&validation::derivation(b"reactant-file\0", path.as_bytes()));
    validation::record_derivation(derivations, &guid, &derivation, &path)?;
    unity_metadata::validate_text(
      &validation::read(project, &format!("{path}.meta"), report)?,
      &guid,
    )?;
  }
  Ok(())
}
