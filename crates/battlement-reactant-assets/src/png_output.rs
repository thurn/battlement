use std::io::Cursor;

use anyhow::{Context, Result, bail};
use battlement_reactant_asset_syntax::{ClipEdge, Compression};
use sha2::{Digest, Sha256};

use crate::CatalogAsset;

const LARGE_RASTER_PIXELS: u64 = 4_194_304;

#[derive(Clone)]
pub(crate) struct RenderedPng {
  pub(crate) bytes: Vec<u8>,
  pub(crate) sha256: String,
  pub(crate) width: u32,
  pub(crate) height: u32,
  pub(crate) alpha: AlphaBounds,
  pub(crate) warnings: Vec<&'static str>,
}

#[derive(Clone, Copy, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct AlphaBounds {
  pub(crate) left: u32,
  pub(crate) top: u32,
  pub(crate) right: u32,
  pub(crate) bottom: u32,
}

pub(crate) fn normalize(asset: &CatalogAsset, captured: &[u8]) -> Result<RenderedPng> {
  let expected_width = self::raster_dimension(
    asset.request.metadata.canvas.width,
    asset.raster_scale,
    "width",
  )?;
  let expected_height = self::raster_dimension(
    asset.request.metadata.canvas.height,
    asset.raster_scale,
    "height",
  )?;
  let mut decoder = png::Decoder::new(Cursor::new(captured));
  decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
  let mut reader = decoder
    .read_info()
    .context("browser capture is not a decodable PNG")?;
  let mut pixels = vec![
    0;
    reader
      .output_buffer_size()
      .context("browser capture exceeds supported dimensions")?
  ];
  let output = reader
    .next_frame(&mut pixels)
    .context("browser capture has invalid image data")?;
  pixels.truncate(output.buffer_size());
  if output.width != expected_width || output.height != expected_height {
    bail!(
      "render for {} produced {}x{} pixels instead of {}x{}",
      asset.address,
      output.width,
      output.height,
      expected_width,
      expected_height
    );
  }
  if output.bit_depth != png::BitDepth::Eight {
    bail!("render for {} is not an 8-bit sRGB image", asset.address);
  }
  let pixels = match output.color_type {
    png::ColorType::Rgba => pixels,
    png::ColorType::Rgb => pixels
      .as_chunks::<3>()
      .0
      .iter()
      .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], u8::MAX])
      .collect(),
    _ => bail!(
      "render for {} is not an sRGB RGB or RGBA image",
      asset.address
    ),
  };
  let alpha = self::alpha_bounds(&pixels, output.width, output.height).with_context(|| {
    format!(
      "render for {} contains no nontransparent paint",
      asset.address
    )
  })?;
  self::validate_edges(asset, alpha, output.width, output.height)?;
  let warnings = self::warnings(asset, &pixels, alpha, output.width, output.height);
  let bytes = self::encode(output.width, output.height, &pixels)?;
  self::validate_structure(&bytes)?;
  Ok(RenderedPng {
    sha256: self::hex(&Sha256::digest(&bytes)),
    bytes,
    width: output.width,
    height: output.height,
    alpha,
    warnings,
  })
}

fn raster_dimension(logical: f64, scale: u8, name: &str) -> Result<u32> {
  let pixels = logical * f64::from(scale);
  if pixels < 1.0 || pixels > f64::from(u32::MAX) || pixels.fract() != 0.0 {
    bail!("raster {name} {pixels} is outside PNG dimensions");
  }
  Ok(pixels as u32)
}

fn alpha_bounds(pixels: &[u8], width: u32, height: u32) -> Option<AlphaBounds> {
  let mut bounds = AlphaBounds {
    left: width,
    top: height,
    right: 0,
    bottom: 0,
  };
  let mut found = false;
  for (index, pixel) in pixels.as_chunks::<4>().0.iter().enumerate() {
    if pixel[3] == 0 {
      continue;
    }
    found = true;
    let x = u32::try_from(index).ok()? % width;
    let y = u32::try_from(index).ok()? / width;
    bounds.left = bounds.left.min(x);
    bounds.top = bounds.top.min(y);
    bounds.right = bounds.right.max(x);
    bounds.bottom = bounds.bottom.max(y);
  }
  found.then_some(bounds)
}

fn validate_edges(
  asset: &CatalogAsset,
  bounds: AlphaBounds,
  width: u32,
  height: u32,
) -> Result<()> {
  for (edge, touching) in [
    (ClipEdge::Top, bounds.top == 0),
    (ClipEdge::Right, bounds.right + 1 == width),
    (ClipEdge::Bottom, bounds.bottom + 1 == height),
    (ClipEdge::Left, bounds.left == 0),
  ] {
    if touching && !asset.request.metadata.allowed_clipping.contains(&edge) {
      bail!(
        "rendered paint touches the unpermitted {} canvas edge for {} from {}",
        self::edge_name(edge),
        asset.address,
        asset.source_symbols.join(", ")
      );
    }
  }
  Ok(())
}

fn warnings(
  asset: &CatalogAsset,
  pixels: &[u8],
  bounds: AlphaBounds,
  width: u32,
  height: u32,
) -> Vec<&'static str> {
  let mut warnings = Vec::new();
  if u64::from(width) * u64::from(height) > LARGE_RASTER_PIXELS {
    warnings.push("large-raster-allocation");
  }
  let translucent = pixels
    .as_chunks::<4>()
    .0
    .iter()
    .any(|pixel| !matches!(pixel[3], 0 | u8::MAX));
  if translucent && asset.request.metadata.compression != Compression::Lossless {
    warnings.push("lossy-translucent-compression");
  }
  let near_permitted = [
    (ClipEdge::Top, bounds.top <= 1),
    (ClipEdge::Right, width - bounds.right <= 2),
    (ClipEdge::Bottom, height - bounds.bottom <= 2),
    (ClipEdge::Left, bounds.left <= 1),
  ]
  .into_iter()
  .any(|(edge, near)| near && asset.request.metadata.allowed_clipping.contains(&edge));
  if near_permitted {
    warnings.push("near-permitted-edge");
  }
  warnings
}

fn edge_name(edge: ClipEdge) -> &'static str {
  match edge {
    ClipEdge::Top => "top",
    ClipEdge::Right => "right",
    ClipEdge::Bottom => "bottom",
    ClipEdge::Left => "left",
  }
}

fn encode(width: u32, height: u32, pixels: &[u8]) -> Result<Vec<u8>> {
  let mut bytes = Vec::new();
  {
    let mut encoder = png::Encoder::new(Cursor::new(&mut bytes), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
    encoder.set_deflate_compression(png::DeflateCompression::Level(6));
    encoder.set_filter(png::Filter::Paeth);
    encoder
      .write_header()
      .context("failed to encode deterministic PNG header")?
      .write_image_data(pixels)
      .context("failed to encode deterministic PNG pixels")?;
  }
  Ok(bytes)
}

fn validate_structure(bytes: &[u8]) -> Result<()> {
  if bytes.get(..8) != Some(b"\x89PNG\r\n\x1a\n") {
    bail!("deterministic PNG signature is invalid");
  }
  let mut offset = 8;
  let mut chunks = Vec::new();
  while offset + 12 <= bytes.len() {
    let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into()?) as usize;
    let kind = bytes
      .get(offset + 4..offset + 8)
      .context("deterministic PNG chunk header is truncated")?;
    chunks.push(kind);
    offset = offset
      .checked_add(12 + length)
      .context("deterministic PNG chunk length overflow")?;
  }
  if offset != bytes.len()
    || chunks.first() != Some(&b"IHDR".as_slice())
    || chunks.last() != Some(&b"IEND".as_slice())
    || !chunks.contains(&b"sRGB".as_slice())
    || chunks
      .iter()
      .any(|kind| !matches!(*kind, b"IHDR" | b"sRGB" | b"IDAT" | b"IEND"))
  {
    bail!("deterministic PNG contains an invalid ancillary chunk layout");
  }
  Ok(())
}

fn hex(bytes: &[u8]) -> String {
  bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
