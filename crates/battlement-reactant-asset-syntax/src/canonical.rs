use sha2::{Digest, Sha256};

use crate::{
  AssetRequest, ClipEdge, Compression, DEFAULT_RASTER_SCALE, DeclarationKind, FilterMode,
  GeneratorMetadata, WrapMode,
};

pub(crate) fn request(request: &AssetRequest) -> Vec<u8> {
  let mut bytes = b"battlement-reactant-asset\0".to_vec();
  bytes.push(match request.kind {
    DeclarationKind::Background => 1,
    DeclarationKind::NineSlice => 2,
    DeclarationKind::TextImage => 3,
  });
  self::metadata(&mut bytes, &request.metadata);
  self::length(&mut bytes, request.paint.len());
  for declaration in &request.paint {
    self::string(&mut bytes, &declaration.property);
    self::blob(&mut bytes, declaration.canonical_value());
  }
  bytes
}

pub(crate) fn identity(bytes: &[u8]) -> [u8; 32] {
  Sha256::digest(bytes).into()
}

pub(crate) fn number(bytes: &mut Vec<u8>, value: f64) {
  bytes.extend(
    if value == 0.0 { 0.0 } else { value }
      .to_bits()
      .to_be_bytes(),
  );
}

pub(crate) fn string(bytes: &mut Vec<u8>, value: &str) {
  self::blob(bytes, value.as_bytes());
}

pub(crate) fn blob(bytes: &mut Vec<u8>, value: &[u8]) {
  self::length(bytes, value.len());
  bytes.extend(value);
}

fn length(bytes: &mut Vec<u8>, value: usize) {
  bytes.extend(
    u32::try_from(value)
      .expect("canonical collection length overflow")
      .to_be_bytes(),
  );
}

fn metadata(bytes: &mut Vec<u8>, metadata: &GeneratorMetadata) {
  for value in [
    metadata.canvas.width,
    metadata.canvas.height,
    metadata.subject.x,
    metadata.subject.y,
    metadata.subject.width,
    metadata.subject.height,
  ] {
    self::number(bytes, value);
  }
  match metadata.slices {
    Some(slices) => {
      bytes.push(1);
      for value in [slices.top, slices.right, slices.bottom, slices.left] {
        self::number(bytes, value);
      }
    }
    None => bytes.push(0),
  }
  self::length(bytes, metadata.allowed_clipping.len());
  for edge in &metadata.allowed_clipping {
    bytes.push(match edge {
      ClipEdge::Top => 1,
      ClipEdge::Right => 2,
      ClipEdge::Bottom => 3,
      ClipEdge::Left => 4,
    });
  }
  if metadata.raster_scale == DEFAULT_RASTER_SCALE {
    bytes.push(0);
  } else {
    bytes.extend([1, metadata.raster_scale]);
  }
  bytes.push(match metadata.filter_mode {
    FilterMode::Bilinear => 1,
    FilterMode::Nearest => 2,
  });
  bytes.push(match metadata.wrap_mode {
    WrapMode::Clamp => 1,
    WrapMode::Repeat => 2,
  });
  bytes.push(match metadata.compression {
    Compression::Lossless => 1,
    Compression::LossyLow => 2,
    Compression::LossyNormal => 3,
    Compression::LossyHigh => 4,
  });
  match &metadata.font_file {
    Some(path) => {
      bytes.push(1);
      self::string(bytes, path);
    }
    None => bytes.push(0),
  }
}
