use std::{collections::BTreeMap, fs, io::Cursor, path::Path};

use anyhow::{Context, Result, bail};
use battlement_reactant_asset_syntax::{DependencyKind, LocalDependency};
use bytes::Bytes;
use sha2::{Digest, Sha256};
use syn::LitStr;
use ttf_parser::{Face, name_id};

use crate::{Discovery, WorkReport};

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogAsset {
  /// Stable public Addressables address, independent of dependency bytes.
  pub address: String,
  /// Lowercase deterministic Unity GUID for the generated PNG.
  pub guid: String,
  /// Canonical request identity.
  pub request_identity: [u8; 32],
  /// Validated dependency records in canonical request order.
  pub dependencies: Vec<DependencyIdentity>,
  /// All declarations deduplicated into this output.
  pub source_symbols: Vec<String>,
}

/// Validated and deduplicated identity table used by later render stages.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
  report: &mut WorkReport,
) -> Result<AssetCatalog> {
  let project = project
    .canonicalize()
    .with_context(|| format!("failed to resolve Unity project {}", project.display()))?;
  let mut requests = BTreeMap::<[u8; 32], PendingAsset>::new();
  for declaration in &discovery.assets {
    let canonical = declaration.request.canonical_bytes();
    let request_identity = declaration.request.identity();
    let dependencies = declaration
      .request
      .dependencies
      .iter()
      .map(|dependency| self::dependency(dependency, &declaration.request, &project, report))
      .collect::<Result<Vec<_>>>()?;
    let address = self::address(request_identity);
    self::validate_address(&address)?;
    self::merge_request(
      &mut requests,
      request_identity,
      canonical,
      CatalogAsset {
        guid: self::guid(b"reactant-asset\0", address.as_bytes()),
        address,
        request_identity,
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
  let directories = [
    GENERATED_ROOT,
    concat!("Assets/Generated/BattlementReactant", "/Resources"),
  ]
  .into_iter()
  .map(|path| DirectoryIdentity {
    path: path.to_owned(),
    guid: self::guid(b"reactant-directory\0", path.as_bytes()),
  })
  .collect();
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

fn dependency(
  dependency: &LocalDependency,
  request: &battlement_reactant_asset_syntax::AssetRequest,
  project: &Path,
  report: &mut WorkReport,
) -> Result<DependencyIdentity> {
  let selected = project.join(&dependency.path);
  let resolved = selected
    .canonicalize()
    .with_context(|| format!("failed to open dependency {}", dependency.path))?;
  if !resolved.starts_with(project) {
    bail!(
      "dependency {} resolves outside Unity project {}",
      dependency.path,
      project.display()
    );
  }
  let bytes = fs::read(&resolved)
    .with_context(|| format!("failed to read dependency {}", dependency.path))?;
  report.files_opened += 1;
  report.dependency_file_opens += 1;
  report.bytes_read += bytes.len() as u64;
  let normalized = match dependency.kind {
    DependencyKind::Image => self::normalize_png(&bytes, &dependency.path)?,
    DependencyKind::Font => self::normalize_font(&bytes, &dependency.path, request)?,
  };
  let mut hash = Sha256::new();
  match dependency.kind {
    DependencyKind::Image => hash.update(b"reactant-image-dependency\0"),
    DependencyKind::Font => hash.update(b"reactant-font-dependency\0"),
  }
  hash.update(normalized);
  Ok(DependencyIdentity {
    kind: dependency.kind,
    path: dependency.path.clone(),
    identity: hash.finalize().into(),
  })
}

fn normalize_png(bytes: &[u8], path: &str) -> Result<Vec<u8>> {
  if !path.to_ascii_lowercase().ends_with(".png") {
    bail!("image dependency {path} must use a .png extension");
  }
  let mut decoder = png::Decoder::new(Cursor::new(bytes));
  decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
  let mut reader = decoder
    .read_info()
    .with_context(|| format!("image dependency {path} is not a decodable PNG"))?;
  if reader.info().animation_control.is_some() {
    bail!("image dependency {path} must be a single-frame PNG");
  }
  let mut pixels = vec![
    0;
    reader
      .output_buffer_size()
      .context("decoded PNG exceeds supported dimensions")?
  ];
  let output = reader
    .next_frame(&mut pixels)
    .with_context(|| format!("image dependency {path} is not a decodable PNG"))?;
  pixels.truncate(output.buffer_size());
  if output.bit_depth != png::BitDepth::Eight {
    bail!("image dependency {path} could not be normalized to 8-bit pixels");
  }
  let rgba = match output.color_type {
    png::ColorType::Grayscale => pixels
      .into_iter()
      .flat_map(|value| [value, value, value, u8::MAX])
      .collect(),
    png::ColorType::GrayscaleAlpha => pixels
      .chunks_exact(2)
      .flat_map(|value| [value[0], value[0], value[0], value[1]])
      .collect(),
    png::ColorType::Rgb => pixels
      .chunks_exact(3)
      .flat_map(|value| [value[0], value[1], value[2], u8::MAX])
      .collect(),
    png::ColorType::Rgba => pixels,
    png::ColorType::Indexed => {
      bail!("image dependency {path} retained an indexed color table after decoding")
    }
  };
  let mut normalized = b"decoded-png\0".to_vec();
  normalized.extend(output.width.to_be_bytes());
  normalized.extend(output.height.to_be_bytes());
  normalized.extend(rgba);
  Ok(normalized)
}

fn normalize_font(
  bytes: &[u8],
  path: &str,
  request: &battlement_reactant_asset_syntax::AssetRequest,
) -> Result<Vec<u8>> {
  let extension = Path::new(path)
    .extension()
    .and_then(|value| value.to_str())
    .map(str::to_ascii_lowercase)
    .context("font dependency has no extension")?;
  let normalized = match extension.as_str() {
    "woff2" => woff2_patched::convert_woff2_to_ttf(&mut Bytes::copy_from_slice(bytes))
      .with_context(|| format!("font dependency {path} is not valid WOFF2"))?,
    "ttf" => {
      if !matches!(bytes.get(..4), Some(b"\0\x01\0\0" | b"true" | b"typ1")) {
        bail!("font dependency {path} extension does not match its TrueType format");
      }
      bytes.to_vec()
    }
    "otf" => {
      if bytes.get(..4) != Some(b"OTTO") {
        bail!("font dependency {path} extension does not match its OpenType format");
      }
      bytes.to_vec()
    }
    _ => bail!("font dependency {path} uses an unsupported format"),
  };
  let face = Face::parse(&normalized, 0)
    .with_context(|| format!("font dependency {path} has invalid font metadata"))?;
  let has_family = face.names().into_iter().any(|name| {
    matches!(name.name_id, name_id::FAMILY | name_id::TYPOGRAPHIC_FAMILY)
      && name
        .to_string()
        .is_some_and(|value| !value.trim().is_empty())
  });
  if !has_family || face.units_per_em() == 0 {
    bail!("font dependency {path} is missing required family metadata");
  }
  let content = request
    .paint
    .iter()
    .find(|paint| paint.property == "content")
    .context("text image request is missing content")?;
  let text = syn::parse_str::<LitStr>(&content.value)
    .with_context(|| format!("text content for {path} is not a Rust string literal"))?
    .value();
  if let Some(character) = text
    .chars()
    .find(|character| face.glyph_index(*character).is_none())
  {
    bail!(
      "font dependency {path} does not cover authored character U+{:04X}",
      u32::from(character)
    );
  }
  Ok(normalized)
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
  use std::{collections::BTreeMap, io::Cursor};

  use battlement_reactant_asset_syntax::DependencyKind;

  use super::{
    CatalogAsset, DependencyIdentity, PendingAsset, merge_request, normalize_png, validate_address,
  };

  #[test]
  fn png_identity_normalizes_equivalent_rgb_and_rgba_encodings() {
    let rgb = self::png(png::ColorType::Rgb, &[255, 0, 0]);
    let rgba = self::png(png::ColorType::Rgba, &[255, 0, 0, 255]);

    assert_eq!(
      normalize_png(&rgb, "Assets/red.png").unwrap(),
      normalize_png(&rgba, "Assets/red.png").unwrap()
    );
  }

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
      address: format!("battlement-reactant/generated/{}.png", "0".repeat(64)),
      guid: "0".repeat(32),
      request_identity: identity,
      dependencies: vec![DependencyIdentity {
        kind: DependencyKind::Image,
        path: "Assets/a.png".to_owned(),
        identity: dependency,
      }],
      source_symbols: vec![source.to_owned()],
    }
  }

  fn png(color: png::ColorType, pixels: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    {
      let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), 1, 1);
      encoder.set_color(color);
      encoder.set_depth(png::BitDepth::Eight);
      encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels)
        .unwrap();
    }
    bytes
  }
}
