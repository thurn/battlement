use std::{
  collections::{BTreeMap, BTreeSet},
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
  thread,
  time::Duration,
};

use anyhow::{Result, bail};
use battlement_ditto::{
  baseline_manifest::{BaselineEntry, BaselineManifest},
  baseline_store::{BaselineStore, ReachedBaseline, hydrate_reached},
  r2_baseline_store::{FetchSelection, PublicObjectClient, R2BaselineStore, fetch},
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use tiny_http::{Response, Server};

const SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn public_http_hydration_supports_a_later_offline_run() {
  let temporary = TempDir::new().unwrap();
  let bytes = png(5, 4, 1);
  let hash = sha256(&bytes);
  let manifest = manifest(vec![entry("macos", "game", "shot", &hash, bytes.len())]);
  let server = Server::http("127.0.0.1:0").unwrap();
  let base_url = format!("http://{}", server.server_addr());
  let expected_path = format!("/team/game/objects/{}/{hash}.png", &hash[..2]);
  let response_bytes = bytes.clone();
  let responder = thread::spawn(move || {
    let request = server.recv().unwrap();
    assert_eq!(request.url(), expected_path);
    request
      .respond(Response::from_data(response_bytes))
      .unwrap();
  });
  let online = R2BaselineStore::new(base_url, Duration::from_secs(2));
  let reached = hydrate_reached(
    &online,
    Some(&manifest),
    temporary.path(),
    "macos",
    "game",
    "shot",
  )
  .unwrap();
  let ReachedBaseline::Hydrated { path, .. } = reached else {
    panic!("expected baseline")
  };
  assert_eq!(std::fs::read(&path).unwrap(), bytes);
  responder.join().unwrap();

  let deny = Arc::new(DenyClient::default());
  let offline = R2BaselineStore::with_client("https://offline.invalid".into(), deny.clone());
  hydrate_reached(
    &offline,
    Some(&manifest),
    temporary.path(),
    "macos",
    "game",
    "shot",
  )
  .unwrap();
  assert_eq!(deny.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn corrupt_downloads_never_enter_the_cache_and_true_misses_are_infrastructure_errors() {
  let temporary = TempDir::new().unwrap();
  let expected = png(2, 2, 1);
  let hash = sha256(&expected);
  let wrong = Arc::new(StaticClient::new(png(2, 2, 2)));
  let store = R2BaselineStore::with_client("https://public.example/baselines".into(), wrong);
  let error = store
    .hydrate("team/game", &hash, temporary.path())
    .unwrap_err();
  assert!(error.to_string().contains("wrong hash"));
  assert!(!cache_path(temporary.path(), &hash).exists());

  let missing = R2BaselineStore::with_client(
    "https://public.example/baselines".into(),
    Arc::new(DenyClient::default()),
  );
  let error = missing
    .hydrate("team/game", &hash, temporary.path())
    .unwrap_err();
  let diagnostic = format!("{error:#}");
  assert!(diagnostic.contains(&hash));
  assert!(diagnostic.contains("https://public.example/baselines/team/game/objects"));
}

#[test]
fn concurrent_same_hash_hydration_downloads_once() {
  let temporary = TempDir::new().unwrap();
  let bytes = png(3, 3, 8);
  let hash = sha256(&bytes);
  let client = Arc::new(DelayedClient::new(BTreeMap::from([(
    hash.clone(),
    bytes.clone(),
  )])));
  let store = Arc::new(R2BaselineStore::with_client(
    "https://public.example".into(),
    client.clone(),
  ));
  thread::scope(|scope| {
    for _ in 0..4 {
      let store = Arc::clone(&store);
      let hash = hash.clone();
      let cache = temporary.path().to_owned();
      scope.spawn(move || {
        store.hydrate("team/game", &hash, &cache).unwrap();
      });
    }
  });
  assert_eq!(client.calls.load(Ordering::SeqCst), 1);
  assert_eq!(
    std::fs::read(cache_path(temporary.path(), &hash)).unwrap(),
    bytes
  );
}

#[test]
fn fetch_selection_is_exact_and_fetch_all_has_bounded_parallelism() {
  let temporary = TempDir::new().unwrap();
  let objects: BTreeMap<_, _> = (1..=4)
    .map(|marker| {
      let bytes = png(2, 2, marker);
      (sha256(&bytes), bytes)
    })
    .collect();
  let hashes: Vec<_> = objects.keys().cloned().collect();
  let manifest = manifest(vec![
    entry(
      "macos",
      "alpha",
      "one",
      &hashes[0],
      objects[&hashes[0]].len(),
    ),
    entry(
      "macos",
      "alpha",
      "duplicate",
      &hashes[0],
      objects[&hashes[0]].len(),
    ),
    entry(
      "macos",
      "beta",
      "two",
      &hashes[1],
      objects[&hashes[1]].len(),
    ),
    entry(
      "ios",
      "alpha",
      "three",
      &hashes[2],
      objects[&hashes[2]].len(),
    ),
    entry(
      "ios",
      "gamma",
      "four",
      &hashes[3],
      objects[&hashes[3]].len(),
    ),
  ]);
  let client = Arc::new(DelayedClient::new(objects.clone()));
  let store = R2BaselineStore::with_client("https://public.example".into(), client.clone());
  let paths = fetch(
    &store,
    &manifest,
    &temporary.path().join("all"),
    FetchSelection::All,
    2,
  )
  .unwrap();
  assert_eq!(paths.len(), 4);
  assert_eq!(client.calls.load(Ordering::SeqCst), 4);
  assert!(client.maximum.load(Ordering::SeqCst) <= 2);
  assert!(client.maximum.load(Ordering::SeqCst) > 1);

  let selected_client = Arc::new(DelayedClient::new(objects));
  let selected_store =
    R2BaselineStore::with_client("https://public.example".into(), selected_client.clone());
  let selected = BTreeSet::from(["alpha".to_owned()]);
  let paths = fetch(
    &selected_store,
    &manifest,
    &temporary.path().join("selected"),
    FetchSelection::Selected {
      profile: "macos",
      scenarios: &selected,
    },
    8,
  )
  .unwrap();
  assert_eq!(paths.len(), 1);
  assert_eq!(selected_client.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn an_unreached_checkpoint_performs_no_public_request() {
  let temporary = TempDir::new().unwrap();
  let client = Arc::new(DenyClient::default());
  let store = R2BaselineStore::with_client("https://public.example".into(), client.clone());
  assert_eq!(
    hydrate_reached(&store, None, temporary.path(), "macos", "game", "shot").unwrap(),
    ReachedBaseline::Missing
  );
  assert_eq!(client.calls.load(Ordering::SeqCst), 0);
}

#[derive(Default)]
struct DenyClient {
  calls: AtomicUsize,
}

impl PublicObjectClient for DenyClient {
  fn get(&self, _url: &str) -> Result<Vec<u8>> {
    self.calls.fetch_add(1, Ordering::SeqCst);
    bail!("network unavailable")
  }
}

struct StaticClient {
  bytes: Vec<u8>,
}

impl StaticClient {
  fn new(bytes: Vec<u8>) -> Self {
    Self { bytes }
  }
}

impl PublicObjectClient for StaticClient {
  fn get(&self, _url: &str) -> Result<Vec<u8>> {
    Ok(self.bytes.clone())
  }
}

struct DelayedClient {
  objects: BTreeMap<String, Vec<u8>>,
  calls: AtomicUsize,
  active: AtomicUsize,
  maximum: AtomicUsize,
  urls: Mutex<Vec<String>>,
}

impl DelayedClient {
  fn new(objects: BTreeMap<String, Vec<u8>>) -> Self {
    Self {
      objects,
      calls: AtomicUsize::new(0),
      active: AtomicUsize::new(0),
      maximum: AtomicUsize::new(0),
      urls: Mutex::new(Vec::new()),
    }
  }
}

impl PublicObjectClient for DelayedClient {
  fn get(&self, url: &str) -> Result<Vec<u8>> {
    self.calls.fetch_add(1, Ordering::SeqCst);
    self.urls.lock().unwrap().push(url.to_owned());
    let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
    self.maximum.fetch_max(active, Ordering::SeqCst);
    thread::sleep(Duration::from_millis(50));
    self.active.fetch_sub(1, Ordering::SeqCst);
    let hash = url.rsplit('/').next().unwrap().trim_end_matches(".png");
    Ok(self.objects[hash].clone())
  }
}

fn manifest(baselines: Vec<BaselineEntry>) -> BaselineManifest {
  BaselineManifest {
    suite: "game".into(),
    namespace: "team/game".into(),
    baselines,
  }
}

fn entry(
  profile: &str,
  scenario: &str,
  checkpoint: &str,
  hash: &str,
  size: usize,
) -> BaselineEntry {
  BaselineEntry {
    profile: profile.into(),
    scenario: scenario.into(),
    checkpoint: checkpoint.into(),
    sha256: hash.into(),
    width: 2,
    height: 2,
    size_bytes: size as u64,
    source: SOURCE.into(),
  }
}

fn cache_path(root: &Path, hash: &str) -> PathBuf {
  root.join(format!("team/game/objects/{}/{hash}.png", &hash[..2]))
}

fn sha256(bytes: &[u8]) -> String {
  format!("{:x}", Sha256::digest(bytes))
}

fn png(width: u32, height: u32, marker: u8) -> Vec<u8> {
  let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
  bytes.extend(width.to_be_bytes());
  bytes.extend(height.to_be_bytes());
  bytes.push(marker);
  bytes
}
