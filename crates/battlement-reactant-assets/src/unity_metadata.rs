use anyhow::{Context, Result, bail};
use battlement_reactant_asset_syntax::{Compression, FilterMode, WrapMode};

const TEXT_IMPORTER: &str = "TextScriptImporter:";
const TEXTURE_IMPORTER: &str = "TextureImporter:";

pub(crate) fn directory(guid: &str) -> Vec<u8> {
  format!(
    "fileFormatVersion: 2\nguid: {guid}\nfolderAsset: yes\nDefaultImporter:\n  externalObjects: {{}}\n  userData:\n  assetBundleName:\n  assetBundleVariant:\n"
  )
  .into_bytes()
}

pub(crate) fn text(guid: &str) -> Vec<u8> {
  format!(
    "fileFormatVersion: 2\nguid: {guid}\nTextScriptImporter:\n  externalObjects: {{}}\n  userData:\n  assetBundleName:\n  assetBundleVariant:\n"
  )
  .into_bytes()
}

pub(crate) fn texture(
  guid: &str,
  filter: FilterMode,
  wrap: WrapMode,
  compression: Compression,
) -> Vec<u8> {
  let (filter, mipmaps) = match filter {
    FilterMode::Bilinear => (1, 0),
    FilterMode::Nearest => (0, 0),
    FilterMode::Trilinear => (2, 1),
  };
  let wrap = match wrap {
    WrapMode::Clamp => 1,
    WrapMode::Repeat => 0,
  };
  let (compression, quality) = match compression {
    Compression::Lossless => (0, 50),
    Compression::LossyLow => (3, 0),
    Compression::LossyNormal => (1, 50),
    Compression::LossyHigh => (2, 100),
  };
  format!(
    "fileFormatVersion: 2
guid: {guid}
TextureImporter:
  internalIDToNameTable: []
  externalObjects: {{}}
  serializedVersion: 13
  mipmaps:
    mipMapMode: 0
    enableMipMap: {mipmaps}
    sRGBTexture: 1
    linearTexture: 0
    fadeOut: 0
    borderMipMap: 0
    mipMapsPreserveCoverage: 0
    alphaTestReferenceValue: 0.5
    mipMapFadeDistanceStart: 1
    mipMapFadeDistanceEnd: 3
  bumpmap:
    convertToNormalMap: 0
    externalNormalMap: 0
    heightScale: 0.25
    normalMapFilter: 0
    flipGreenChannel: 0
  isReadable: 0
  streamingMipmaps: 0
  streamingMipmapsPriority: 0
  vTOnly: 0
  ignoreMipmapLimit: 0
  grayScaleToAlpha: 0
  generateCubemap: 6
  cubemapConvolution: 0
  seamlessCubemap: 0
  textureFormat: 1
  maxTextureSize: 16384
  textureSettings:
    serializedVersion: 2
    filterMode: {filter}
    aniso: 1
    mipBias: 0
    wrapU: {wrap}
    wrapV: {wrap}
    wrapW: {wrap}
  nPOTScale: 0
  lightmap: 0
  compressionQuality: {quality}
  spriteMode: 0
  spriteExtrude: 1
  spriteMeshType: 1
  alignment: 0
  spritePivot: {{x: 0.5, y: 0.5}}
  spritePixelsToUnits: 100
  spriteBorder: {{x: 0, y: 0, z: 0, w: 0}}
  spriteGenerateFallbackPhysicsShape: 1
  alphaUsage: 1
  alphaIsTransparency: 1
  spriteTessellationDetail: -1
  textureType: 0
  textureShape: 1
  singleChannelComponent: 0
  flipbookRows: 1
  flipbookColumns: 1
  maxTextureSizeSet: 0
  compressionQualitySet: 0
  textureFormatSet: 0
  ignorePngGamma: 0
  applyGammaDecoding: 0
  swizzle: 50462976
  cookieLightType: 0
  platformSettings:
  - serializedVersion: 4
    buildTarget: DefaultTexturePlatform
    maxTextureSize: 16384
    resizeAlgorithm: 0
    textureFormat: -1
    textureCompression: {compression}
    compressionQuality: {quality}
    crunchedCompression: 0
    allowsAlphaSplitting: 0
    overridden: 0
    ignorePlatformSupport: 0
    androidETC2FallbackOverride: 0
    forceMaximumCompressionQuality_BC6H_BC7: 0
  spriteSheet:
    serializedVersion: 2
    sprites: []
    outline: []
    customData:
    physicsShape: []
    bones: []
    spriteID:
    internalID: 0
    vertices: []
    indices:
    edges: []
    weights: []
    secondaryTextures: []
    spriteCustomMetadata:
      entries: []
    nameFileIdTable: {{}}
  mipmapLimitGroupName:
  pSDRemoveMatte: 0
  userData:
  assetBundleName:
  assetBundleVariant:
"
  )
  .into_bytes()
}

pub(crate) fn validate_directory(bytes: &[u8], guid: &str) -> Result<()> {
  let text = self::utf8(bytes)?;
  self::common(text, guid)?;
  self::one(text, "folderAsset", "yes")?;
  self::header(text, "DefaultImporter:")?;
  self::labels(text)
}

pub(crate) fn validate_text(bytes: &[u8], guid: &str) -> Result<()> {
  let text = self::utf8(bytes)?;
  self::common(text, guid)?;
  self::header(text, TEXT_IMPORTER)?;
  self::labels(text)
}

pub(crate) fn validate_texture(
  bytes: &[u8],
  guid: &str,
  filter: FilterMode,
  wrap: WrapMode,
  compression: Compression,
) -> Result<()> {
  let text = self::utf8(bytes)?;
  self::common(text, guid)?;
  self::header(text, TEXTURE_IMPORTER)?;
  self::one(
    text,
    "enableMipMap",
    if filter == FilterMode::Trilinear {
      "1"
    } else {
      "0"
    },
  )?;
  self::one(text, "sRGBTexture", "1")?;
  self::one(text, "alphaIsTransparency", "1")?;
  self::one(text, "textureType", "0")?;
  self::one(
    text,
    "filterMode",
    match filter {
      FilterMode::Bilinear => "1",
      FilterMode::Nearest => "0",
      FilterMode::Trilinear => "2",
    },
  )?;
  let wrap = match wrap {
    WrapMode::Clamp => "1",
    WrapMode::Repeat => "0",
  };
  for field in ["wrapU", "wrapV", "wrapW"] {
    self::one(text, field, wrap)?;
  }
  self::one(text, "buildTarget", "DefaultTexturePlatform")?;
  self::one(text, "overridden", "0")?;
  self::one(
    text,
    "textureCompression",
    match compression {
      Compression::Lossless => "0",
      Compression::LossyLow => "3",
      Compression::LossyNormal => "1",
      Compression::LossyHigh => "2",
    },
  )?;
  self::labels(text)
}

fn utf8(bytes: &[u8]) -> Result<&str> {
  std::str::from_utf8(bytes).context("Unity metadata is not UTF-8")
}

fn common(text: &str, guid: &str) -> Result<()> {
  self::one(text, "fileFormatVersion", "2")?;
  self::one(text, "guid", guid)?;
  if text
    .lines()
    .any(|line| line.trim_start().starts_with("labels:"))
  {
    bail!("generated Unity metadata must not contain labels");
  }
  Ok(())
}

fn header(text: &str, expected: &str) -> Result<()> {
  let importers = text
    .lines()
    .filter(|line| {
      !line.starts_with(char::is_whitespace)
        && matches!(
          line.trim(),
          "DefaultImporter:" | TEXT_IMPORTER | TEXTURE_IMPORTER
        )
    })
    .collect::<Vec<_>>();
  if importers != [expected] {
    bail!("Unity metadata has the wrong importer family");
  }
  Ok(())
}

fn labels(text: &str) -> Result<()> {
  for field in ["userData", "assetBundleName", "assetBundleVariant"] {
    self::one(text, field, "")?;
  }
  Ok(())
}

fn one(text: &str, key: &str, expected: &str) -> Result<()> {
  let values = text
    .lines()
    .filter_map(|line| {
      line
        .trim_start()
        .strip_prefix(key)
        .and_then(|value| value.strip_prefix(':'))
        .map(str::trim)
    })
    .collect::<Vec<_>>();
  if values.as_slice() != [expected] {
    bail!("Unity metadata field {key} must be {expected:?}");
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use battlement_reactant_asset_syntax::{Compression, FilterMode, WrapMode};

  #[test]
  fn trilinear_filtering_enables_mipmaps() {
    let bytes = super::texture(
      "0123456789abcdef0123456789abcdef",
      FilterMode::Trilinear,
      WrapMode::Clamp,
      Compression::Lossless,
    );
    let text = String::from_utf8(bytes.clone()).unwrap();

    assert!(text.contains("    enableMipMap: 1\n"));
    assert!(text.contains("    filterMode: 2\n"));
    super::validate_texture(
      &bytes,
      "0123456789abcdef0123456789abcdef",
      FilterMode::Trilinear,
      WrapMode::Clamp,
      Compression::Lossless,
    )
    .unwrap();
  }

  #[test]
  fn semantic_validation_accepts_key_reordering_and_rejects_overrides_and_labels() {
    let bytes = super::texture(
      "0123456789abcdef0123456789abcdef",
      FilterMode::Nearest,
      WrapMode::Repeat,
      Compression::LossyHigh,
    );
    super::validate_texture(
      &bytes,
      "0123456789abcdef0123456789abcdef",
      FilterMode::Nearest,
      WrapMode::Repeat,
      Compression::LossyHigh,
    )
    .unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let reordered = text.replacen(
      "  isReadable: 0\n  streamingMipmaps: 0",
      "  streamingMipmaps: 0\n  isReadable: 0",
      1,
    );
    super::validate_texture(
      reordered.as_bytes(),
      "0123456789abcdef0123456789abcdef",
      FilterMode::Nearest,
      WrapMode::Repeat,
      Compression::LossyHigh,
    )
    .unwrap();
    let override_added = text.replace(
      "  spriteSheet:",
      "  - buildTarget: Standalone\n    overridden: 1\n  spriteSheet:",
    );
    assert!(
      super::validate_texture(
        override_added.as_bytes(),
        "0123456789abcdef0123456789abcdef",
        FilterMode::Nearest,
        WrapMode::Repeat,
        Compression::LossyHigh,
      )
      .is_err()
    );
    let labeled = text.replace("  assetBundleName:\n", "  assetBundleName: generated\n");
    assert!(
      super::validate_texture(
        labeled.as_bytes(),
        "0123456789abcdef0123456789abcdef",
        FilterMode::Nearest,
        WrapMode::Repeat,
        Compression::LossyHigh,
      )
      .is_err()
    );
  }
}
