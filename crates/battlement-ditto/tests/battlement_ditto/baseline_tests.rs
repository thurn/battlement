use std::{
  collections::{BTreeMap, BTreeSet},
  fs,
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
  },
  thread,
  time::Duration,
};

use anyhow::{Result, bail};
use battlement_ditto::{
  baseline_manifest::{BaselineEntry, BaselineManifest, ManifestSnapshot},
  baseline_store::{BaselineStore, FilesystemBaselineStore, ReachedBaseline, hydrate_reached},
  baseline_update::{
    BaselineProposal, BaselineUpdateRequest, ScenarioUpdate, ScenarioUpdateStatus, apply,
  },
  wire::result::BaselineWriteStatus,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn lock_serialization_is_canonical_and_absence_is_explicit() {
  let temporary = TempDir::new().unwrap();
  let path = temporary.path().join("ditto.lock");
  assert_eq!(ManifestSnapshot::read(&path).unwrap().sha256, None);

  let manifest = BaselineManifest {
    suite: "game".into(),
    namespace: "team/game".into(),
    baselines: vec![
      entry("z", "second", "end", "bb"),
      entry("a", "first", "start", "aa"),
    ],
  };
  let bytes = manifest.canonical_bytes().unwrap();
  assert_eq!(
    String::from_utf8(bytes.clone()).unwrap(),
    format!(
      concat!(
        "suite = \"game\"\nnamespace = \"team/game\"\n\n",
        "[[baselines]]\nprofile = \"a\"\nscenario = \"first\"\ncheckpoint = \"start\"\n",
        "sha256 = \"{a}\"\nwidth = 2\nheight = 1\nsize_bytes = 24\nsource = \"{SOURCE}\"\n\n",
        "[[baselines]]\nprofile = \"z\"\nscenario = \"second\"\ncheckpoint = \"end\"\n",
        "sha256 = \"{b}\"\nwidth = 2\nheight = 1\nsize_bytes = 24\nsource = \"{SOURCE}\"\n",
      ),
      a = hash_fill("aa"),
      b = hash_fill("bb"),
      SOURCE = SOURCE,
    )
  );
  fs::write(&path, &bytes).unwrap();
  let snapshot = ManifestSnapshot::read(&path).unwrap();
  assert_eq!(snapshot.manifest.unwrap().canonical_bytes().unwrap(), bytes);
  assert_eq!(snapshot.sha256.unwrap(), sha256(&bytes));
  assert!(BaselineManifest::parse(b"suite='x'\nnamespace='x'\nunknown=true\n").is_err());
}

#[test]
fn filesystem_store_hydrates_external_objects_only_when_reached() {
  let temporary = TempDir::new().unwrap();
  let store_root = temporary.path().join("outside-repository");
  let cache = temporary.path().join("cache");
  let png = png(3, 2, 7);
  let hash = sha256(&png);
  let source = temporary.path().join("actual.png");
  fs::write(&source, &png).unwrap();
  let store = FilesystemBaselineStore::new(store_root.clone());
  store.put("team/game", &hash, &source).unwrap();
  assert_eq!(
    store.object_path("team/game", &hash).unwrap(),
    store_root.join(format!("team/game/objects/{}/{hash}.png", &hash[..2]))
  );

  let calls = CountingStore::new(store);
  assert_eq!(
    hydrate_reached(&calls, None, &cache, "macos", "game", "shot").unwrap(),
    ReachedBaseline::Missing
  );
  assert_eq!(calls.calls(), 0);
  let manifest = BaselineManifest {
    suite: "game".into(),
    namespace: "team/game".into(),
    baselines: vec![BaselineEntry {
      profile: "macos".into(),
      scenario: "game".into(),
      checkpoint: "shot".into(),
      sha256: hash.clone(),
      width: 3,
      height: 2,
      size_bytes: png.len() as u64,
      source: SOURCE.into(),
    }],
  };
  let loaded = hydrate_reached(&calls, Some(&manifest), &cache, "macos", "game", "shot").unwrap();
  let ReachedBaseline::Hydrated { path, .. } = loaded else {
    panic!("expected hydration")
  };
  assert_eq!(fs::read(path).unwrap(), png);
  assert_eq!(calls.calls(), 1);
}

#[test]
fn full_update_publishes_eligible_captures_and_prunes_only_authored_absences() {
  let fixture = Fixture::new();
  fixture.write_manifest(vec![
    entry("macos", "removed", "old", "11"),
    entry("macos", "failed", "kept", "22"),
    entry("ios", "removed", "old", "33"),
  ]);
  let starting = ManifestSnapshot::read(&fixture.lock).unwrap().sha256;
  let eligible = fixture.proposal("eligible", "new", 1);
  let failed = fixture.proposal("failed", "kept", 2);
  let skipped = fixture.proposal("skipped", "kept", 3);
  let authored = authored(&[
    ("eligible", &["new"]),
    ("failed", &["kept"]),
    ("skipped", &["kept"]),
  ]);
  let scenarios = vec![
    scenario(
      "eligible",
      ScenarioUpdateStatus::Eligible,
      vec![eligible.clone()],
    ),
    scenario("failed", ScenarioUpdateStatus::Failed, vec![failed]),
    scenario(
      "skipped",
      ScenarioUpdateStatus::RuntimeSkipped,
      vec![skipped],
    ),
  ];
  let result = apply(
    &fixture.store,
    request(&fixture, starting, false, &authored, &scenarios),
  )
  .unwrap();

  assert_eq!(result.writes.len(), 1);
  assert_eq!(result.writes[0].status, BaselineWriteStatus::Published);
  let manifest = ManifestSnapshot::read(&fixture.lock)
    .unwrap()
    .manifest
    .unwrap();
  assert_eq!(
    manifest.find("macos", "eligible", "new").unwrap().sha256,
    eligible.sha256
  );
  assert!(manifest.find("macos", "removed", "old").is_none());
  assert_eq!(
    manifest.find("macos", "failed", "kept").unwrap().sha256,
    hash_fill("22")
  );
  assert!(manifest.find("macos", "skipped", "kept").is_none());
  assert!(manifest.find("ios", "removed", "old").is_some());
  assert_eq!(
    result.lock_sha256,
    ManifestSnapshot::read(&fixture.lock)
      .unwrap()
      .sha256
      .unwrap()
  );
}

#[test]
fn filtered_update_preserves_unselected_scenarios_and_prunes_selected_checkpoints() {
  let fixture = Fixture::new();
  fixture.write_manifest(vec![
    entry("macos", "selected", "removed", "11"),
    entry("macos", "other", "removed", "22"),
  ]);
  let authored = authored(&[("selected", &["current"]), ("other", &[])]);
  let scenarios = vec![scenario(
    "selected",
    ScenarioUpdateStatus::Eligible,
    Vec::new(),
  )];
  apply(
    &fixture.store,
    request(
      &fixture,
      ManifestSnapshot::read(&fixture.lock).unwrap().sha256,
      true,
      &authored,
      &scenarios,
    ),
  )
  .unwrap();
  let manifest = ManifestSnapshot::read(&fixture.lock)
    .unwrap()
    .manifest
    .unwrap();
  assert!(manifest.find("macos", "selected", "removed").is_none());
  assert!(manifest.find("macos", "other", "removed").is_some());
}

#[test]
fn a_matching_capture_is_not_uploaded_or_reported_as_an_update() {
  let fixture = Fixture::new();
  let proposal = fixture.proposal("game", "shot", 4);
  fixture.write_manifest(vec![BaselineEntry {
    profile: "macos".into(),
    scenario: "game".into(),
    checkpoint: "shot".into(),
    sha256: proposal.sha256,
    width: proposal.width,
    height: proposal.height,
    size_bytes: proposal.size_bytes,
    source: proposal.source,
  }]);
  let original = fs::read(&fixture.lock).unwrap();
  let authored = authored(&[("game", &["shot"])]);
  let scenarios = vec![scenario(
    "game",
    ScenarioUpdateStatus::Eligible,
    vec![
      BaselineProposal::from_png(
        "game".into(),
        "shot".into(),
        fixture.temporary.path().join("game-shot-4.png"),
        SOURCE.into(),
      )
      .unwrap(),
    ],
  )];
  let calls = CountingStore::new(fixture.store.clone());
  let result = apply(
    &calls,
    request(
      &fixture,
      ManifestSnapshot::read(&fixture.lock).unwrap().sha256,
      false,
      &authored,
      &scenarios,
    ),
  )
  .unwrap();
  assert!(result.writes.is_empty());
  assert_eq!(calls.put_calls(), 0);
  assert_eq!(fs::read(&fixture.lock).unwrap(), original);
}

#[test]
fn stale_and_partial_updates_leave_the_manifest_byte_for_byte_unchanged() {
  let fixture = Fixture::new();
  fixture.write_manifest(Vec::new());
  let original = fs::read(&fixture.lock).unwrap();
  let authored = authored(&[("game", &["one", "two"])]);
  let scenarios = vec![scenario(
    "game",
    ScenarioUpdateStatus::Eligible,
    vec![
      fixture.proposal("game", "one", 1),
      fixture.proposal("game", "two", 2),
    ],
  )];
  let failing = FailingStore {
    inner: fixture.store.clone(),
    calls: AtomicUsize::new(0),
    fail_at: 1,
  };
  let failure = apply(
    &failing,
    request(
      &fixture,
      ManifestSnapshot::read(&fixture.lock).unwrap().sha256,
      false,
      &authored,
      &scenarios,
    ),
  )
  .unwrap_err();
  assert_eq!(fs::read(&fixture.lock).unwrap(), original);
  assert_eq!(
    failure.writes[0].status,
    BaselineWriteStatus::UploadedUnreferenced
  );
  assert_eq!(failure.writes[1].status, BaselineWriteStatus::Proposed);

  let stale = apply(
    &fixture.store,
    request(
      &fixture,
      Some(hash_fill("ff")),
      false,
      &authored,
      &scenarios,
    ),
  )
  .unwrap_err();
  assert!(stale.reason.contains("starting ditto.lock digest"));
  assert_eq!(fs::read(&fixture.lock).unwrap(), original);
}

#[test]
fn a_manifest_write_failure_reports_uploaded_unreferenced_objects() {
  let fixture = Fixture::new();
  let authored = authored(&[("game", &["shot"])]);
  let scenarios = vec![scenario(
    "game",
    ScenarioUpdateStatus::Eligible,
    vec![fixture.proposal("game", "shot", 1)],
  )];
  let store = BreakManifestStore {
    inner: fixture.store.clone(),
    lock: fixture.lock.clone(),
  };
  let failure = apply(
    &store,
    request(&fixture, None, false, &authored, &scenarios),
  )
  .unwrap_err();
  assert!(failure.reason.contains("replace ditto.lock"));
  assert_eq!(
    failure.writes[0].status,
    BaselineWriteStatus::UploadedUnreferenced
  );
  assert!(fixture.lock.is_dir());
}

#[test]
fn concurrent_updates_serialize_and_the_loser_observes_a_stale_lock() {
  let fixture = Fixture::new();
  let (started_tx, started_rx) = mpsc::channel();
  let store = Arc::new(DelayedStore {
    inner: fixture.store.clone(),
    calls: AtomicUsize::new(0),
    started: Mutex::new(Some(started_tx)),
  });
  let lock = fixture.lock.clone();
  let first_store = Arc::clone(&store);
  let first = thread::spawn(move || run_initial_update(&lock, first_store.as_ref(), 1));
  started_rx.recv().unwrap();
  let lock = fixture.lock.clone();
  let second_store = Arc::clone(&store);
  let second = thread::spawn(move || run_initial_update(&lock, second_store.as_ref(), 2));
  assert!(first.join().unwrap().is_ok());
  let failure = second.join().unwrap().unwrap_err();
  assert!(
    failure
      .reason
      .contains("changed while the update was running")
  );
  assert_eq!(store.calls.load(Ordering::SeqCst), 1);
}

#[derive(Clone)]
struct CountingStore {
  inner: FilesystemBaselineStore,
  hydrate_calls: Arc<AtomicUsize>,
  put_calls: Arc<AtomicUsize>,
}

impl CountingStore {
  fn new(inner: FilesystemBaselineStore) -> Self {
    Self {
      inner,
      hydrate_calls: Arc::new(AtomicUsize::new(0)),
      put_calls: Arc::new(AtomicUsize::new(0)),
    }
  }

  fn calls(&self) -> usize {
    self.hydrate_calls.load(Ordering::SeqCst)
  }

  fn put_calls(&self) -> usize {
    self.put_calls.load(Ordering::SeqCst)
  }
}

impl BaselineStore for CountingStore {
  fn hydrate(&self, namespace: &str, sha256: &str, cache_root: &Path) -> Result<PathBuf> {
    self.hydrate_calls.fetch_add(1, Ordering::SeqCst);
    self.inner.hydrate(namespace, sha256, cache_root)
  }

  fn put(&self, namespace: &str, sha256: &str, source: &Path) -> Result<()> {
    self.put_calls.fetch_add(1, Ordering::SeqCst);
    self.inner.put(namespace, sha256, source)
  }
}

struct FailingStore {
  inner: FilesystemBaselineStore,
  calls: AtomicUsize,
  fail_at: usize,
}

impl BaselineStore for FailingStore {
  fn hydrate(&self, namespace: &str, sha256: &str, cache_root: &Path) -> Result<PathBuf> {
    self.inner.hydrate(namespace, sha256, cache_root)
  }

  fn put(&self, namespace: &str, sha256: &str, source: &Path) -> Result<()> {
    if self.calls.fetch_add(1, Ordering::SeqCst) == self.fail_at {
      bail!("injected upload failure");
    }
    self.inner.put(namespace, sha256, source)
  }
}

struct BreakManifestStore {
  inner: FilesystemBaselineStore,
  lock: PathBuf,
}

impl BaselineStore for BreakManifestStore {
  fn hydrate(&self, namespace: &str, sha256: &str, cache_root: &Path) -> Result<PathBuf> {
    self.inner.hydrate(namespace, sha256, cache_root)
  }

  fn put(&self, namespace: &str, sha256: &str, source: &Path) -> Result<()> {
    self.inner.put(namespace, sha256, source)?;
    fs::create_dir(&self.lock)?;
    Ok(())
  }
}

struct DelayedStore {
  inner: FilesystemBaselineStore,
  calls: AtomicUsize,
  started: Mutex<Option<mpsc::Sender<()>>>,
}

impl BaselineStore for DelayedStore {
  fn hydrate(&self, namespace: &str, sha256: &str, cache_root: &Path) -> Result<PathBuf> {
    self.inner.hydrate(namespace, sha256, cache_root)
  }

  fn put(&self, namespace: &str, sha256: &str, source: &Path) -> Result<()> {
    self.calls.fetch_add(1, Ordering::SeqCst);
    if let Some(sender) = self.started.lock().unwrap().take() {
      sender.send(()).unwrap();
      thread::sleep(Duration::from_millis(150));
    }
    self.inner.put(namespace, sha256, source)
  }
}

struct Fixture {
  temporary: TempDir,
  lock: PathBuf,
  store: FilesystemBaselineStore,
}

impl Fixture {
  fn new() -> Self {
    let temporary = TempDir::new().unwrap();
    Self {
      lock: temporary.path().join("ditto.lock"),
      store: FilesystemBaselineStore::new(temporary.path().join("store")),
      temporary,
    }
  }

  fn write_manifest(&self, baselines: Vec<BaselineEntry>) {
    fs::write(
      &self.lock,
      BaselineManifest {
        suite: "game".into(),
        namespace: "team/game".into(),
        baselines,
      }
      .canonical_bytes()
      .unwrap(),
    )
    .unwrap();
  }

  fn proposal(&self, scenario: &str, checkpoint: &str, marker: u8) -> BaselineProposal {
    let path = self
      .temporary
      .path()
      .join(format!("{scenario}-{checkpoint}-{marker}.png"));
    fs::write(&path, png(2, 1, marker)).unwrap();
    BaselineProposal::from_png(scenario.into(), checkpoint.into(), path, SOURCE.into()).unwrap()
  }
}

fn request<'a>(
  fixture: &'a Fixture,
  starting_lock_sha256: Option<String>,
  filtered: bool,
  authored_checkpoints: &'a BTreeMap<String, BTreeSet<String>>,
  scenarios: &'a [ScenarioUpdate],
) -> BaselineUpdateRequest<'a> {
  BaselineUpdateRequest {
    lock_path: &fixture.lock,
    starting_lock_sha256,
    suite: "game",
    namespace: "team/game",
    profile: "macos",
    filtered,
    authored_checkpoints,
    scenarios,
  }
}

fn run_initial_update(
  lock: &Path,
  delayed: &dyn BaselineStore,
  marker: u8,
) -> std::result::Result<
  battlement_ditto::baseline_update::BaselineUpdateResult,
  battlement_ditto::baseline_update::BaselineUpdateFailure,
> {
  let directory = lock.parent().unwrap();
  let actual = directory.join(format!("concurrent-{marker}.png"));
  fs::write(&actual, png(2, 1, marker)).unwrap();
  let proposal =
    BaselineProposal::from_png("game".into(), "shot".into(), actual, SOURCE.into()).unwrap();
  let authored = authored(&[("game", &["shot"])]);
  let scenarios = vec![scenario(
    "game",
    ScenarioUpdateStatus::Eligible,
    vec![proposal],
  )];
  apply(
    delayed,
    BaselineUpdateRequest {
      lock_path: lock,
      starting_lock_sha256: None,
      suite: "game",
      namespace: "team/game",
      profile: "macos",
      filtered: false,
      authored_checkpoints: &authored,
      scenarios: &scenarios,
    },
  )
}

fn scenario(
  name: &str,
  status: ScenarioUpdateStatus,
  proposals: Vec<BaselineProposal>,
) -> ScenarioUpdate {
  ScenarioUpdate {
    name: name.into(),
    status,
    proposals,
  }
}

fn authored(entries: &[(&str, &[&str])]) -> BTreeMap<String, BTreeSet<String>> {
  entries
    .iter()
    .map(|(scenario, checkpoints)| {
      (
        (*scenario).to_owned(),
        checkpoints
          .iter()
          .map(|value| (*value).to_owned())
          .collect(),
      )
    })
    .collect()
}

fn entry(profile: &str, scenario: &str, checkpoint: &str, hash_prefix: &str) -> BaselineEntry {
  BaselineEntry {
    profile: profile.into(),
    scenario: scenario.into(),
    checkpoint: checkpoint.into(),
    sha256: hash_fill(hash_prefix),
    width: 2,
    height: 1,
    size_bytes: 24,
    source: SOURCE.into(),
  }
}

fn hash_fill(prefix: &str) -> String {
  prefix.repeat(32)
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
