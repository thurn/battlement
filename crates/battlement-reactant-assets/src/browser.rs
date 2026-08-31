use std::{
  collections::BTreeMap,
  env, fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
  AssetCatalog, CommandOptions, WorkReport,
  browser_protocol::BrowserSession,
  incremental::{FileFingerprint, fingerprint},
};

const RENDERER_SOURCE: &str = concat!(
  "battlement-reactant-browser-renderer\0",
  env!("CARGO_PKG_VERSION"),
  "\0locale=en-US;timezone=UTC;color=light;motion=reduce;font=16px;time=0"
);

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct BrowserIndex {
  identity: Option<BrowserIdentity>,
  renderer_identity: String,
  requests: BTreeMap<String, BrowserProbe>,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct BrowserIdentity {
  pub(crate) executable_path: String,
  pub(crate) executable_fingerprint: FileFingerprint,
  pub(crate) executable_sha256: String,
  pub(crate) product: String,
  pub(crate) protocol_version: String,
  pub(crate) revision: String,
  pub(crate) user_agent: String,
  pub(crate) javascript_version: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct BrowserProbe {
  cache_key: String,
  image_hash: String,
}

pub(crate) struct BrowserRun {
  pub(crate) executable_path: String,
  pub(crate) product: String,
  pub(crate) protocol_version: String,
  pub(crate) executable_sha256: String,
  pub(crate) renderer_identity: String,
  pub(crate) requests: Vec<BrowserRequest>,
  pub(crate) session_requests: usize,
}

pub(crate) struct BrowserRequest {
  pub(crate) address: String,
  pub(crate) cache_key: String,
  pub(crate) image_hash: String,
}

pub(crate) fn prepare(
  options: &CommandOptions,
  catalog: &AssetCatalog,
  index: &mut BrowserIndex,
  report: &mut WorkReport,
) -> Result<BrowserRun> {
  let executable = self::select(options.browser.as_deref(), report)?;
  let current = fingerprint(&executable, report)
    .with_context(|| format!("failed to fingerprint browser {}", executable.display()))?;
  let renderer_identity = self::hex(&Sha256::digest(RENDERER_SOURCE.as_bytes()));
  let retained = index.identity.as_ref().filter(|identity| {
    identity.executable_path == self::normalized(&executable)
      && identity.executable_fingerprint == current
      && index.renderer_identity == renderer_identity
  });
  if let Some(identity) = retained {
    let requests = self::request_records(catalog, identity, &renderer_identity, &index.requests);
    if requests.iter().all(|record| record.is_some()) {
      return Ok(self::run_record(
        identity,
        renderer_identity,
        requests.into_iter().flatten().collect(),
        0,
      ));
    }
  }

  let cached_hash = retained.map(|identity| identity.executable_sha256.as_str());
  let (mut session, identity) = BrowserSession::launch(
    &executable,
    current,
    cached_hash,
    options.browser.is_some(),
    report,
  )?;
  let expected = catalog
    .assets
    .iter()
    .map(|asset| self::cache_key(asset, &identity, &renderer_identity))
    .collect::<Vec<_>>();
  let mut requests = BTreeMap::new();
  let mut records = Vec::new();
  let mut session_requests = 0;
  for (asset, cache_key) in catalog.assets.iter().zip(expected) {
    let retained_hash = index
      .requests
      .get(&asset.address)
      .filter(|probe| probe.cache_key == cache_key)
      .map(|probe| probe.image_hash.clone());
    let image_hash = if let Some(image_hash) = retained_hash {
      image_hash
    } else {
      session_requests += 1;
      session.probe(&cache_key)?
    };
    requests.insert(
      asset.address.clone(),
      BrowserProbe {
        cache_key: cache_key.clone(),
        image_hash: image_hash.clone(),
      },
    );
    records.push(BrowserRequest {
      address: asset.address.clone(),
      cache_key,
      image_hash,
    });
  }
  session.finish()?;
  index.identity = Some(identity.clone());
  index.renderer_identity.clone_from(&renderer_identity);
  index.requests = requests;
  Ok(self::run_record(
    &identity,
    renderer_identity,
    records,
    session_requests,
  ))
}

fn request_records(
  catalog: &AssetCatalog,
  identity: &BrowserIdentity,
  renderer_identity: &str,
  probes: &BTreeMap<String, BrowserProbe>,
) -> Vec<Option<BrowserRequest>> {
  catalog
    .assets
    .iter()
    .map(|asset| {
      let cache_key = self::cache_key(asset, identity, renderer_identity);
      probes.get(&asset.address).and_then(|probe| {
        (probe.cache_key == cache_key).then(|| BrowserRequest {
          address: asset.address.clone(),
          cache_key,
          image_hash: probe.image_hash.clone(),
        })
      })
    })
    .collect()
}

fn run_record(
  identity: &BrowserIdentity,
  renderer_identity: String,
  requests: Vec<BrowserRequest>,
  session_requests: usize,
) -> BrowserRun {
  BrowserRun {
    executable_path: identity.executable_path.clone(),
    product: identity.product.clone(),
    protocol_version: identity.protocol_version.clone(),
    executable_sha256: identity.executable_sha256.clone(),
    renderer_identity,
    requests,
    session_requests,
  }
}

fn select(explicit: Option<&Path>, report: &mut WorkReport) -> Result<PathBuf> {
  if let Some(path) = explicit {
    let current = env::current_dir().context("failed to read the current directory")?;
    return self::canonical_executable(&current.join(path))
      .with_context(|| format!("explicit browser {} is not executable", path.display()));
  }
  let candidates = self::default_candidates(report);
  if let Some(path) = candidates.iter().find(|path| self::is_executable(path)) {
    return self::canonical_executable(path);
  }
  bail!(
    "no supported stable Chrome or Chromium browser was found; searched {}; pass --browser <path>",
    candidates
      .iter()
      .map(|path| path.display().to_string())
      .collect::<Vec<_>>()
      .join(", ")
  )
}

fn default_candidates(report: &mut WorkReport) -> Vec<PathBuf> {
  #[cfg(target_os = "macos")]
  {
    let _ = report;
    vec![
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome".into(),
      "/Applications/Chromium.app/Contents/MacOS/Chromium".into(),
    ]
  }
  #[cfg(target_os = "windows")]
  {
    return self::windows_candidates(report);
  }
  #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
  {
    let _ = report;
    let path = env::var_os("PATH").unwrap_or_default();
    let directories = env::split_paths(&path).collect::<Vec<_>>();
    return ["google-chrome-stable", "google-chrome", "chromium"]
      .into_iter()
      .flat_map(|name| {
        directories
          .iter()
          .map(move |directory| directory.join(name))
      })
      .collect();
  }
}

#[cfg(target_os = "windows")]
fn windows_candidates(report: &mut WorkReport) -> Vec<PathBuf> {
  let mut candidates = Vec::new();
  for executable in ["chrome.exe", "chromium.exe"] {
    for hive in ["HKCU", "HKLM"] {
      report.subprocesses_started += 1;
      let key =
        format!("{hive}\\Software\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{executable}");
      if let Ok(output) = std::process::Command::new("reg")
        .args(["query", &key, "/ve"])
        .output()
      {
        let text = String::from_utf8_lossy(&output.stdout);
        if let Some(path) = text
          .lines()
          .find_map(|line| line.split_once("REG_SZ").map(|(_, value)| value.trim()))
        {
          candidates.push(PathBuf::from(path));
        }
      }
    }
  }
  candidates
}

fn canonical_executable(path: &Path) -> Result<PathBuf> {
  let path = path
    .canonicalize()
    .with_context(|| format!("failed to locate browser {}", path.display()))?;
  if !self::is_executable(&path) {
    bail!("{} is not executable", path.display());
  }
  Ok(path)
}

fn is_executable(path: &Path) -> bool {
  let Ok(metadata) = fs::metadata(path) else {
    return false;
  };
  if !metadata.is_file() {
    return false;
  }
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
  }
  #[cfg(not(unix))]
  true
}

fn cache_key(
  asset: &crate::CatalogAsset,
  browser: &BrowserIdentity,
  renderer_identity: &str,
) -> String {
  let mut hash = Sha256::new();
  hash.update(b"reactant-output-cache\0");
  self::field(&mut hash, &asset.canonical_request);
  for dependency in &asset.dependencies {
    match dependency.kind {
      battlement_reactant_asset_syntax::DependencyKind::Image => hash.update(b"image\0"),
      battlement_reactant_asset_syntax::DependencyKind::Font => hash.update(b"font\0"),
    }
    self::field(&mut hash, dependency.path.as_bytes());
    self::field(&mut hash, &dependency.identity);
  }
  hash.update([asset.raster_scale]);
  for value in [
    &browser.executable_sha256,
    &browser.product,
    &browser.protocol_version,
    &browser.revision,
    &browser.user_agent,
    &browser.javascript_version,
    renderer_identity,
  ] {
    self::field(&mut hash, value.as_bytes());
  }
  self::hex(&hash.finalize())
}

fn field(hash: &mut Sha256, bytes: &[u8]) {
  hash.update((bytes.len() as u64).to_be_bytes());
  hash.update(bytes);
}

fn normalized(path: &Path) -> String {
  path.to_string_lossy().replace('\\', "/")
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
  use battlement_reactant_asset_syntax::DependencyKind;

  use crate::{CatalogAsset, DependencyIdentity, incremental::FileFingerprint};

  use super::{BrowserIdentity, cache_key};

  #[test]
  fn browser_renderer_dependency_and_scale_are_cache_key_inputs() {
    let asset = CatalogAsset {
      canonical_request: b"request".to_vec(),
      address: "address".to_owned(),
      guid: "guid".to_owned(),
      request_identity: [1; 32],
      raster_scale: 1,
      dependencies: vec![DependencyIdentity {
        kind: DependencyKind::Image,
        path: "Assets/image.png".to_owned(),
        identity: [2; 32],
      }],
      source_symbols: vec!["source".to_owned()],
    };
    let browser = BrowserIdentity {
      executable_path: "/browser".to_owned(),
      executable_fingerprint: FileFingerprint {
        path: "/browser".to_owned(),
        file_id: "1".to_owned(),
        byte_length: 1,
        modified_nanoseconds: 1,
      },
      executable_sha256: "executable".to_owned(),
      product: "Chrome/1".to_owned(),
      protocol_version: "1".to_owned(),
      revision: "revision".to_owned(),
      user_agent: "agent".to_owned(),
      javascript_version: "javascript".to_owned(),
    };
    let original = cache_key(&asset, &browser, "renderer");

    let mut changed_browser = browser.clone();
    changed_browser.product = "Chrome/2".to_owned();
    let mut changed_dependency = asset.clone();
    changed_dependency.dependencies[0].identity = [3; 32];
    let mut changed_scale = asset.clone();
    changed_scale.raster_scale = 2;

    assert_ne!(cache_key(&asset, &changed_browser, "renderer"), original);
    assert_ne!(cache_key(&asset, &browser, "other-renderer"), original);
    assert_ne!(
      cache_key(&changed_dependency, &browser, "renderer"),
      original
    );
    assert_ne!(cache_key(&changed_scale, &browser, "renderer"), original);
  }
}
