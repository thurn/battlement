use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Manifest {
  pub(crate) assets: Vec<AssetRecord>,
  pub(crate) browser: BrowserRecord,
  pub(crate) renderer_identity: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct AssetRecord {
  pub(crate) address: String,
  pub(crate) cache_key: String,
  pub(crate) canonical_request_sha256: String,
  pub(crate) dependencies: Vec<DependencyRecord>,
  pub(crate) import: ImportRecord,
  pub(crate) kind: String,
  pub(crate) logical_canvas: LogicalSizeRecord,
  pub(crate) png: String,
  pub(crate) png_sha256: String,
  pub(crate) raster_scale: u8,
  pub(crate) raster_size: RasterSizeRecord,
  pub(crate) slice_insets: Option<SliceInsetsRecord>,
  pub(crate) subject_bounds: SubjectBoundsRecord,
  pub(crate) unity_guid: String,
  pub(crate) unity_guid_derivation_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct BrowserRecord {
  pub(crate) executable_file_identity: FileIdentityRecord,
  pub(crate) executable_path: String,
  pub(crate) executable_sha256: String,
  pub(crate) product: String,
  pub(crate) version: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct DependencyRecord {
  pub(crate) content_sha256: String,
  pub(crate) kind: String,
  pub(crate) path: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct FileIdentityRecord {
  pub(crate) byte_length: u64,
  pub(crate) file_id: String,
  pub(crate) modified_nanoseconds: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ImportRecord {
  pub(crate) alpha_is_transparency: bool,
  pub(crate) compression: String,
  pub(crate) filter_mode: String,
  pub(crate) mipmaps: bool,
  pub(crate) s_rgb: bool,
  pub(crate) texture_type: String,
  pub(crate) wrap_mode: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct LogicalSizeRecord {
  pub(crate) height: f64,
  pub(crate) width: f64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct RasterSizeRecord {
  pub(crate) height: u32,
  pub(crate) width: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct Sidecar {
  pub(crate) addresses: Vec<String>,
  pub(crate) manifest_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct SliceInsetsRecord {
  pub(crate) bottom: f64,
  pub(crate) left: f64,
  pub(crate) right: f64,
  pub(crate) top: f64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct SubjectBoundsRecord {
  pub(crate) height: f64,
  pub(crate) width: f64,
  pub(crate) x: f64,
  pub(crate) y: f64,
}
