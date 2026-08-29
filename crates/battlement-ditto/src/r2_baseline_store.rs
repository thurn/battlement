//! Credential-free R2 baseline hydration and prefetching.

use std::{
  collections::BTreeSet,
  fs::{self, OpenOptions},
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  thread,
  time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use fs2::FileExt;
use sha2::{Digest, Sha256};
use ureq::Agent;

use crate::{
  baseline_manifest::BaselineManifest,
  baseline_store::{BaselineStore, object_path, verify_file, write_atomic},
};

const MAXIMUM_PNG_BYTES: u64 = 64 * 1024 * 1024;

/// Performs an unauthenticated public object read.
pub trait PublicObjectClient: Send + Sync {
  fn get(&self, url: &str) -> Result<Vec<u8>>;
}

/// Fetches public objects with a bounded blocking HTTP client.
pub struct UreqPublicObjectClient {
  agent: Agent,
}

/// A read-only R2 store backed by a public base URL and a durable local cache.
pub struct R2BaselineStore {
  public_base_url: String,
  client: Arc<dyn PublicObjectClient>,
}

/// Which manifest objects an explicit fetch should hydrate.
pub enum FetchSelection<'a> {
  Selected {
    profile: &'a str,
    scenarios: &'a BTreeSet<String>,
  },
  All,
}

impl UreqPublicObjectClient {
  /// Creates a reusable client with one total deadline per request.
  pub fn new(timeout: Duration) -> Self {
    let config = Agent::config_builder()
      .timeout_global(Some(timeout))
      .build();
    Self {
      agent: config.into(),
    }
  }
}

impl PublicObjectClient for UreqPublicObjectClient {
  fn get(&self, url: &str) -> Result<Vec<u8>> {
    self
      .agent
      .get(url)
      .call()
      .with_context(|| format!("download public baseline {url}"))?
      .body_mut()
      .with_config()
      .limit(MAXIMUM_PNG_BYTES)
      .read_to_vec()
      .context("read public baseline response")
  }
}

impl R2BaselineStore {
  /// Uses the configured public URL without reading any write credentials.
  pub fn new(public_base_url: String, timeout: Duration) -> Self {
    Self::with_client(
      public_base_url,
      Arc::new(UreqPublicObjectClient::new(timeout)),
    )
  }

  /// Uses an explicit public client, primarily for deterministic host testing.
  pub fn with_client(public_base_url: String, client: Arc<dyn PublicObjectClient>) -> Self {
    Self {
      public_base_url: public_base_url.trim_end_matches('/').to_owned(),
      client,
    }
  }

  fn object_url(&self, namespace: &str, sha256: &str) -> Result<String> {
    let path = object_path(Path::new(""), namespace, sha256)?;
    Ok(format!(
      "{}/{}",
      self.public_base_url,
      path.to_string_lossy()
    ))
  }
}

impl BaselineStore for R2BaselineStore {
  fn hydrate(&self, namespace: &str, sha256: &str, cache_root: &Path) -> Result<PathBuf> {
    let destination = object_path(cache_root, namespace, sha256)?;
    if verified_cache(&destination, sha256)? {
      return Ok(destination);
    }
    let lease = OpenOptions::new()
      .create(true)
      .read(true)
      .write(true)
      .truncate(false)
      .open(destination.with_extension("download.lock"))
      .context("open baseline download lease")?;
    lease.lock_exclusive()?;
    if verified_cache(&destination, sha256)? {
      return Ok(destination);
    }
    let url = self.object_url(namespace, sha256)?;
    let bytes = self
      .client
      .get(&url)
      .with_context(|| format!("baseline {sha256} is unavailable from {url}"))?;
    ensure!(
      format!("{:x}", Sha256::digest(&bytes)) == sha256,
      "downloaded baseline {sha256} from {url} has the wrong hash"
    );
    write_atomic(&destination, &bytes).context("cache verified public baseline")?;
    Ok(destination)
  }

  fn put(&self, _namespace: &str, _sha256: &str, _source: &Path) -> Result<()> {
    bail!("R2 writes require the publishing store")
  }
}

/// Hydrates selected scenarios or the entire manifest with bounded parallelism.
pub fn fetch(
  store: &dyn BaselineStore,
  manifest: &BaselineManifest,
  cache_root: &Path,
  selection: FetchSelection<'_>,
  parallelism: usize,
) -> Result<Vec<PathBuf>> {
  ensure!(parallelism > 0, "fetch parallelism must be positive");
  let hashes: Vec<_> = manifest
    .baselines
    .iter()
    .filter(|entry| match &selection {
      FetchSelection::All => true,
      FetchSelection::Selected { profile, scenarios } => {
        entry.profile == *profile && scenarios.contains(&entry.scenario)
      }
    })
    .map(|entry| entry.sha256.clone())
    .collect::<BTreeSet<_>>()
    .into_iter()
    .collect();
  let next = AtomicUsize::new(0);
  let results = Mutex::new((0..hashes.len()).map(|_| None).collect::<Vec<_>>());
  thread::scope(|scope| {
    for _ in 0..parallelism.min(hashes.len()) {
      scope.spawn(|| {
        loop {
          let index = next.fetch_add(1, Ordering::Relaxed);
          let Some(hash) = hashes.get(index) else { break };
          results.lock().unwrap()[index] =
            Some(store.hydrate(&manifest.namespace, hash, cache_root));
        }
      });
    }
  });
  results
    .into_inner()
    .unwrap()
    .into_iter()
    .map(|result| result.context("fetch worker omitted a result")?)
    .collect()
}

fn verified_cache(path: &Path, sha256: &str) -> Result<bool> {
  if !path.exists() {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }
    return Ok(false);
  }
  if verify_file(path, sha256).is_ok() {
    return Ok(true);
  }
  fs::remove_file(path).context("remove corrupt cached baseline")?;
  Ok(false)
}
