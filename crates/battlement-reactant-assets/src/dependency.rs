use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  io::Cursor,
  path::Path,
};

use anyhow::{Context, Result, bail};
use battlement_reactant_asset_syntax::{AssetRequest, DependencyKind, LocalDependency};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use syn::LitStr;
use ttf_parser::{Face, name_id};

use crate::{
  WorkReport,
  identity::DependencyIdentity,
  incremental::{FileFingerprint, fingerprint},
};

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct DependencyIndex {
  records: BTreeMap<String, DependencyRecord>,
  #[serde(skip)]
  visited: BTreeSet<String>,
  #[serde(skip)]
  render_bytes: BTreeMap<String, Vec<u8>>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct DependencyRecord {
  kind: String,
  path: String,
  fingerprint: FileFingerprint,
  identity: [u8; 32],
  font_codepoints: Vec<u32>,
  diagnostics: Vec<String>,
}

impl DependencyIndex {
  pub(crate) fn begin_run(&mut self) {
    self.visited.clear();
    self.render_bytes.clear();
  }

  pub(crate) fn retain_visited(&mut self) {
    self.records.retain(|key, _| self.visited.contains(key));
  }

  fn reusable(&mut self, key: &str, fingerprint: &FileFingerprint) -> Option<DependencyRecord> {
    self.visited.insert(key.to_owned());
    self
      .records
      .get(key)
      .filter(|record| record.fingerprint == *fingerprint)
      .cloned()
  }

  fn insert(&mut self, key: String, record: DependencyRecord) {
    self.visited.insert(key.clone());
    self.records.insert(key, record);
  }

  pub(crate) fn render_bytes(
    &mut self,
    dependency: &DependencyIdentity,
    project: &Path,
    report: &mut WorkReport,
  ) -> Result<Vec<u8>> {
    let key = format!("{}:{}", self::kind_name(dependency.kind), dependency.path);
    if let Some(bytes) = self.render_bytes.get(&key) {
      return Ok(bytes.clone());
    }
    let resolved = project
      .join(&dependency.path)
      .canonicalize()
      .with_context(|| format!("failed to open dependency {}", dependency.path))?;
    if !resolved.starts_with(project) {
      bail!(
        "dependency {} resolves outside Unity project",
        dependency.path
      );
    }
    let bytes = fs::read(&resolved)
      .with_context(|| format!("failed to read dependency {}", dependency.path))?;
    report.files_opened += 1;
    report.dependency_file_opens += 1;
    report.bytes_read += bytes.len() as u64;
    let normalized = match dependency.kind {
      DependencyKind::Image => self::normalize_png(&bytes, &dependency.path)?,
      DependencyKind::Font => self::normalize_font(&bytes, &dependency.path)?.0,
    };
    if self::dependency_identity(dependency.kind, &normalized) != dependency.identity {
      bail!("dependency {} changed during rendering", dependency.path);
    }
    self.render_bytes.insert(key, bytes.clone());
    Ok(bytes)
  }
}

pub(crate) fn resolve(
  dependency: &LocalDependency,
  request: &AssetRequest,
  project: &Path,
  index: &mut DependencyIndex,
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
  let current = fingerprint(&resolved, report)
    .with_context(|| format!("failed to fingerprint dependency {}", dependency.path))?;
  let kind = self::kind_name(dependency.kind);
  let key = format!("{kind}:{}", dependency.path);
  if let Some(record) = index.reusable(&key, &current) {
    if dependency.kind == DependencyKind::Font {
      self::validate_font_coverage(&record.font_codepoints, &dependency.path, request)?;
    }
    return Ok(DependencyIdentity {
      kind: dependency.kind,
      path: dependency.path.clone(),
      identity: record.identity,
    });
  }
  let bytes = fs::read(&resolved)
    .with_context(|| format!("failed to read dependency {}", dependency.path))?;
  report.files_opened += 1;
  report.dependency_file_opens += 1;
  report.bytes_read += bytes.len() as u64;
  let (normalized, font_codepoints) = match dependency.kind {
    DependencyKind::Image => (self::normalize_png(&bytes, &dependency.path)?, Vec::new()),
    DependencyKind::Font => self::normalize_font(&bytes, &dependency.path)?,
  };
  let identity = self::dependency_identity(dependency.kind, &normalized);
  index.render_bytes.insert(key.clone(), bytes.clone());
  if dependency.kind == DependencyKind::Font {
    self::validate_font_coverage(&font_codepoints, &dependency.path, request)?;
  }
  index.insert(
    key,
    DependencyRecord {
      kind: kind.to_owned(),
      path: dependency.path.clone(),
      fingerprint: current,
      identity,
      font_codepoints,
      diagnostics: Vec::new(),
    },
  );
  Ok(DependencyIdentity {
    kind: dependency.kind,
    path: dependency.path.clone(),
    identity,
  })
}

fn kind_name(kind: DependencyKind) -> &'static str {
  match kind {
    DependencyKind::Image => "image",
    DependencyKind::Font => "font",
  }
}

fn dependency_identity(kind: DependencyKind, normalized: &[u8]) -> [u8; 32] {
  let mut hash = Sha256::new();
  match kind {
    DependencyKind::Image => hash.update(b"reactant-image-dependency\0"),
    DependencyKind::Font => hash.update(b"reactant-font-dependency\0"),
  }
  hash.update(normalized);
  hash.finalize().into()
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
      .as_chunks::<2>()
      .0
      .iter()
      .flat_map(|value| [value[0], value[0], value[0], value[1]])
      .collect(),
    png::ColorType::Rgb => pixels
      .as_chunks::<3>()
      .0
      .iter()
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

fn normalize_font(bytes: &[u8], path: &str) -> Result<(Vec<u8>, Vec<u32>)> {
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
  let mut codepoints = BTreeSet::new();
  let cmap = face
    .tables()
    .cmap
    .context("font dependency has no Unicode character map")?;
  for subtable in cmap.subtables {
    if subtable.is_unicode() {
      subtable.codepoints(|codepoint| {
        if subtable.glyph_index(codepoint).is_some() {
          codepoints.insert(codepoint);
        }
      });
    }
  }
  Ok((normalized, codepoints.into_iter().collect()))
}

fn validate_font_coverage(codepoints: &[u32], path: &str, request: &AssetRequest) -> Result<()> {
  let content = request
    .paint
    .iter()
    .find(|paint| paint.property == "content")
    .context("text image request is missing content")?;
  let text = syn::parse_str::<LitStr>(&content.value)
    .with_context(|| format!("text content for {path} is not a Rust string literal"))?
    .value();
  if let Some(character) = text.chars().find(|character| {
    !self::layout_control(*character)
      && !self::shaping_control(*character)
      && codepoints.binary_search(&u32::from(*character)).is_err()
  }) {
    bail!(
      "font dependency {path} does not cover authored character U+{:04X}",
      u32::from(character)
    );
  }
  Ok(())
}

fn layout_control(character: char) -> bool {
  matches!(character, '\n' | '\r' | '\t')
}

fn shaping_control(character: char) -> bool {
  matches!(u32::from(character), 0x200C | 0x200D | 0xFE00..=0xFE0F | 0xE0100..=0xE01EF)
}

#[cfg(test)]
mod tests {
  use std::io::Cursor;

  use super::{layout_control, normalize_png};

  #[test]
  fn font_coverage_ignores_text_layout_controls() {
    for character in ['\n', '\r', '\t'] {
      assert!(layout_control(character));
    }
    assert!(!layout_control(' '));
    assert!(!layout_control('A'));
  }

  #[test]
  fn png_identity_normalizes_equivalent_rgb_and_rgba_encodings() {
    let rgb = self::png(png::ColorType::Rgb, &[255, 0, 0]);
    let rgba = self::png(png::ColorType::Rgba, &[255, 0, 0, 255]);

    assert_eq!(
      normalize_png(&rgb, "Assets/red.png").unwrap(),
      normalize_png(&rgba, "Assets/red.png").unwrap()
    );
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
