use std::{
  collections::BTreeMap,
  fs,
  path::Path,
  sync::{
    Arc, Barrier,
    atomic::{AtomicUsize, Ordering},
  },
  thread,
};

use battlement_tooling::{
  build_cache::{
    BUILD_LOG_FILE, BuildAccess, BuildCache, BuildFailure, CacheEvent, CleanupScope,
    SOURCE_MANIFEST_FILE,
  },
  build_identity::{
    AppleToolchain, BuildIdentity, BuildIdentityRequest, BuildTarget, CaptureAdapter, NativeInput,
    RustToolchain,
  },
  fingerprint::SourceManifest,
};
use tempfile::TempDir;

const HASH_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const HASH_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
const REPOSITORY: &str = "/fixture/repository";

#[test]
fn concurrent_callers_publish_once_and_reuse_complete_entry() {
  let temporary = TempDir::new().unwrap();
  let cache = Arc::new(BuildCache::open(temporary.path(), 1024 * 1024).unwrap());
  let identity = Arc::new(identity(HASH_A, "one"));
  let barrier = Arc::new(Barrier::new(6));
  let builders = Arc::new(AtomicUsize::new(0));
  let mut threads = Vec::new();
  for index in 0..6 {
    let cache = Arc::clone(&cache);
    let identity = Arc::clone(&identity);
    let barrier = Arc::clone(&barrier);
    let builders = Arc::clone(&builders);
    threads.push(thread::spawn(move || {
      barrier.wait();
      match cache
        .acquire(REPOSITORY, "suite-a", &identity, 10 + index)
        .unwrap()
      {
        BuildAccess::Build(pending) => {
          builders.fetch_add(1, Ordering::SeqCst);
          populate(&pending, b"player bytes");
          pending.publish(Path::new("player.app"), 20).unwrap().build
        }
        BuildAccess::Reused(build) => build,
      }
    }));
  }
  let builds = threads
    .into_iter()
    .map(|thread| thread.join().unwrap())
    .collect::<Vec<_>>();

  assert_eq!(builders.load(Ordering::SeqCst), 1);
  for build in &builds {
    assert_eq!(build.metadata().identity, *identity);
    assert_eq!(
      fs::read(build.player_path().join("player.bin")).unwrap(),
      b"player bytes"
    );
    assert!(build.path().join(BUILD_LOG_FILE).is_file());
    assert!(build.path().join(SOURCE_MANIFEST_FILE).is_file());
  }
  drop(builds);
  let journal = cache.journal().unwrap();
  assert_eq!(
    journal
      .iter()
      .filter(|entry| entry.event == CacheEvent::Created)
      .count(),
    1
  );
  assert_eq!(
    journal
      .iter()
      .filter(|entry| entry.event == CacheEvent::Reused)
      .count(),
    5
  );
}

#[test]
fn interrupted_and_failed_builds_never_become_reusable() {
  let temporary = TempDir::new().unwrap();
  let cache = BuildCache::open(temporary.path(), 1024 * 1024).unwrap();
  let current = identity(HASH_A, "current");
  let interrupted = expect_pending(cache.acquire(REPOSITORY, "suite-a", &current, 1).unwrap());
  fs::write(interrupted.path().join("partial-player"), b"partial").unwrap();
  drop(interrupted);
  assert!(!entry_path(temporary.path(), &current).exists());

  let failed = expect_pending(cache.acquire(REPOSITORY, "suite-a", &current, 2).unwrap());
  fs::write(failed.path().join(BUILD_LOG_FILE), b"compiler output\n").unwrap();
  let failure_path = failed
    .fail(&BuildFailure {
      phase: "rust".to_owned(),
      error_ids: vec!["E0308".to_owned()],
      message: "compilation failed".to_owned(),
      failed_at_unix_s: 3,
    })
    .unwrap();
  assert_eq!(
    fs::read(failure_path.join(BUILD_LOG_FILE)).unwrap(),
    b"compiler output\n"
  );
  assert!(failure_path.join("failure.json").is_file());
  assert!(!failure_path.join("partial-player").exists());
  assert!(!entry_path(temporary.path(), &current).exists());
  assert!(matches!(
    cache.acquire(REPOSITORY, "suite-a", &current, 4).unwrap(),
    BuildAccess::Build(_)
  ));
  assert_eq!(cache.journal().unwrap()[0].event, CacheEvent::Failed);
}

#[test]
fn publication_requires_player_manifest_and_full_log() {
  let temporary = TempDir::new().unwrap();
  let cache = BuildCache::open(temporary.path(), 1024 * 1024).unwrap();
  let identity = identity(HASH_A, "incomplete");
  let pending = expect_pending(cache.acquire(REPOSITORY, "suite-a", &identity, 1).unwrap());
  fs::create_dir(pending.path().join("player.app")).unwrap();
  fs::write(pending.path().join("player.app/player.bin"), b"partial").unwrap();
  fs::write(pending.path().join(BUILD_LOG_FILE), b"log\n").unwrap();
  assert!(
    pending
      .publish(Path::new("player.app"), 1)
      .unwrap_err()
      .to_string()
      .contains(SOURCE_MANIFEST_FILE)
  );
  assert!(!entry_path(temporary.path(), &identity).exists());
  assert!(matches!(
    cache.acquire(REPOSITORY, "suite-a", &identity, 2).unwrap(),
    BuildAccess::Build(_)
  ));
}

#[test]
fn an_oversize_entry_completes_and_reports_itself_while_active() {
  let temporary = TempDir::new().unwrap();
  let cache = BuildCache::open(temporary.path(), 1).unwrap();
  let identity = identity(HASH_A, "oversize");
  let pending = expect_pending(cache.acquire(REPOSITORY, "suite-a", &identity, 1).unwrap());
  populate(&pending, &[b'x'; 256]);
  let published = pending.publish(Path::new("player.app"), 1).unwrap();

  assert!(published.maintenance.evicted.is_empty());
  assert_eq!(published.maintenance.active.len(), 1);
  assert_eq!(published.maintenance.active[0], identity.fingerprint);
  assert_eq!(published.maintenance.oversize.len(), 1);
  assert_eq!(
    published.maintenance.oversize[0].fingerprint,
    identity.fingerprint
  );
  assert!(published.build.path().is_dir());
}

#[test]
fn failed_current_source_never_falls_back_to_an_older_build() {
  let temporary = TempDir::new().unwrap();
  let cache = BuildCache::open(temporary.path(), 1024 * 1024).unwrap();
  let older = identity(HASH_A, "older");
  let older_pending = expect_pending(cache.acquire(REPOSITORY, "suite-a", &older, 1).unwrap());
  populate(&older_pending, b"old player");
  drop(
    older_pending
      .publish(Path::new("player.app"), 1)
      .unwrap()
      .build,
  );

  let current = identity(HASH_B, "current");
  let current_pending = expect_pending(cache.acquire(REPOSITORY, "suite-a", &current, 2).unwrap());
  fs::write(
    current_pending.path().join(BUILD_LOG_FILE),
    b"failed current\n",
  )
  .unwrap();
  current_pending
    .fail(&BuildFailure {
      phase: "unity".to_owned(),
      error_ids: vec!["CS1002".to_owned()],
      message: "Unity compilation failed".to_owned(),
      failed_at_unix_s: 2,
    })
    .unwrap();

  assert!(matches!(
    cache.acquire(REPOSITORY, "suite-a", &current, 3).unwrap(),
    BuildAccess::Build(_)
  ));
  assert!(matches!(
    cache.acquire(REPOSITORY, "suite-a", &older, 3).unwrap(),
    BuildAccess::Reused(_)
  ));
}

#[test]
fn lru_pressure_skips_active_entries_and_reports_oversize_builds() {
  let temporary = TempDir::new().unwrap();
  let generous = BuildCache::open(temporary.path(), 1024 * 1024).unwrap();
  let first = identity(HASH_A, "first");
  let pending = expect_pending(generous.acquire(REPOSITORY, "suite-a", &first, 1).unwrap());
  populate(&pending, &[b'a'; 256]);
  drop(pending.publish(Path::new("player.app"), 1).unwrap().build);

  let second = identity(HASH_B, "second");
  let pending = expect_pending(generous.acquire(REPOSITORY, "suite-b", &second, 2).unwrap());
  populate(&pending, &[b'b'; 256]);
  let active = pending.publish(Path::new("player.app"), 2).unwrap().build;

  let constrained = BuildCache::open(temporary.path(), 1).unwrap();
  let report = constrained.enforce_limit(3).unwrap();
  assert_eq!(report.evicted.len(), 1);
  assert_eq!(report.evicted[0].fingerprint, first.fingerprint);
  assert_eq!(report.active.len(), 1);
  assert_eq!(report.active[0], second.fingerprint);
  assert_eq!(report.oversize[0].fingerprint, second.fingerprint);
  assert!(entry_path(temporary.path(), &second).is_dir());

  drop(active);
  let report = constrained.enforce_limit(4).unwrap();
  assert_eq!(report.evicted[0].fingerprint, second.fingerprint);
  assert_eq!(report.remaining_bytes, 0);
}

#[test]
fn suite_cleanup_is_scoped_and_global_cleanup_removes_the_rest() {
  let temporary = TempDir::new().unwrap();
  let cache = BuildCache::open(temporary.path(), 1024 * 1024).unwrap();
  let suite_a = identity(HASH_A, "suite-a");
  let pending = expect_pending(cache.acquire(REPOSITORY, "suite-a", &suite_a, 1).unwrap());
  populate(&pending, b"suite a");
  drop(pending.publish(Path::new("player.app"), 1).unwrap().build);
  let suite_b = identity(HASH_B, "suite-b");
  let pending = expect_pending(cache.acquire(REPOSITORY, "suite-b", &suite_b, 2).unwrap());
  populate(&pending, b"suite b");
  drop(pending.publish(Path::new("player.app"), 2).unwrap().build);

  let report = cache
    .cleanup(
      &CleanupScope::Suite {
        repository: REPOSITORY.to_owned(),
        suite: "suite-a".to_owned(),
      },
      3,
    )
    .unwrap();
  assert_eq!(report.evicted.len(), 1);
  assert_eq!(report.evicted[0].suite, "suite-a");
  assert!(!entry_path(temporary.path(), &suite_a).exists());
  assert!(entry_path(temporary.path(), &suite_b).exists());

  let report = cache.cleanup(&CleanupScope::Global, 4).unwrap();
  assert_eq!(report.evicted.len(), 1);
  assert_eq!(report.evicted[0].suite, "suite-b");
  assert_eq!(report.remaining_bytes, 0);
  assert_eq!(
    cache
      .journal()
      .unwrap()
      .iter()
      .map(|entry| entry.event)
      .collect::<Vec<_>>(),
    [
      CacheEvent::Created,
      CacheEvent::Created,
      CacheEvent::Evicted,
      CacheEvent::Evicted,
    ]
  );
}

#[test]
fn cleanup_preview_counts_only_inactive_builds_without_mutation() {
  let temporary = TempDir::new().unwrap();
  let cache = BuildCache::open(temporary.path(), 1024 * 1024).unwrap();
  let first = identity(HASH_A, "first");
  let pending = expect_pending(cache.acquire(REPOSITORY, "suite-a", &first, 1).unwrap());
  populate(&pending, b"first");
  drop(pending.publish(Path::new("player.app"), 1).unwrap().build);
  let second = identity(HASH_B, "second");
  let pending = expect_pending(cache.acquire(REPOSITORY, "suite-a", &second, 2).unwrap());
  populate(&pending, b"second");
  let active = pending.publish(Path::new("player.app"), 2).unwrap().build;

  let preview = cache
    .cleanup_preview(&CleanupScope::Suite {
      repository: REPOSITORY.to_owned(),
      suite: "suite-a".to_owned(),
    })
    .unwrap();
  assert_eq!(preview.inactive.len(), 1);
  assert_eq!(preview.inactive[0].fingerprint, first.fingerprint);
  assert_eq!(preview.active.len(), 1);
  assert_eq!(preview.active[0], second.fingerprint);
  assert!(entry_path(temporary.path(), &first).is_dir());
  assert!(entry_path(temporary.path(), &second).is_dir());
  drop(active);
}

#[test]
fn planned_cleanup_never_expands_to_later_or_newly_inactive_builds() {
  let temporary = TempDir::new().unwrap();
  let cache = BuildCache::open(temporary.path(), 1024 * 1024).unwrap();
  let first = identity(HASH_A, "first");
  let pending = expect_pending(cache.acquire(REPOSITORY, "suite-a", &first, 1).unwrap());
  populate(&pending, b"first");
  drop(pending.publish(Path::new("player.app"), 1).unwrap().build);
  let preview = cache
    .cleanup_preview(&CleanupScope::Suite {
      repository: REPOSITORY.to_owned(),
      suite: "suite-a".to_owned(),
    })
    .unwrap();

  let second = identity(HASH_B, "second");
  let pending = expect_pending(cache.acquire(REPOSITORY, "suite-a", &second, 2).unwrap());
  populate(&pending, b"second");
  drop(pending.publish(Path::new("player.app"), 2).unwrap().build);
  let active = match cache.acquire(REPOSITORY, "suite-a", &first, 3).unwrap() {
    BuildAccess::Reused(build) => build,
    BuildAccess::Build(_) => panic!("planned build disappeared"),
  };

  let report = cache.cleanup_planned(&preview, 4).unwrap();
  assert!(report.evicted.is_empty());
  assert_eq!(report.active, std::slice::from_ref(&first.fingerprint));
  assert!(entry_path(temporary.path(), &first).is_dir());
  assert!(entry_path(temporary.path(), &second).is_dir());
  drop(active);
}

#[test]
fn suite_cleanup_includes_repository_identity() {
  let temporary = TempDir::new().unwrap();
  let cache = BuildCache::open(temporary.path(), 1024 * 1024).unwrap();
  let first = identity(HASH_A, "first");
  let pending = expect_pending(cache.acquire("/repo/a", "shared", &first, 1).unwrap());
  populate(&pending, b"first");
  drop(pending.publish(Path::new("player.app"), 1).unwrap().build);
  let second = identity(HASH_B, "second");
  let pending = expect_pending(cache.acquire("/repo/b", "shared", &second, 2).unwrap());
  populate(&pending, b"second");
  drop(pending.publish(Path::new("player.app"), 2).unwrap().build);

  let preview = cache
    .cleanup_preview(&CleanupScope::Suite {
      repository: "/repo/a".to_owned(),
      suite: "shared".to_owned(),
    })
    .unwrap();
  assert_eq!(preview.inactive.len(), 1);
  assert_eq!(preview.inactive[0].fingerprint, first.fingerprint);
  cache.cleanup_planned(&preview, 3).unwrap();
  assert!(!entry_path(temporary.path(), &first).exists());
  assert!(entry_path(temporary.path(), &second).is_dir());
}

fn identity(source: &str, build_number: &str) -> BuildIdentity {
  BuildIdentity::derive(&BuildIdentityRequest {
    source_fingerprint: source.to_owned(),
    target: BuildTarget::Macos,
    unity_version: "6000.5.8f1".to_owned(),
    rust: RustToolchain {
      rustc_version: "rustc 1.91.0".to_owned(),
      cargo_version: "cargo 1.91.0".to_owned(),
      target: "aarch64-apple-darwin".to_owned(),
    },
    apple: Some(AppleToolchain {
      xcode_version: "Xcode 26.0".to_owned(),
      sdk_version: "macosx26.0".to_owned(),
    }),
    diagnostics: true,
    capture_adapter: CaptureAdapter {
      name: "async-readback".to_owned(),
      version: "1".to_owned(),
    },
    native_inputs: vec![NativeInput {
      name: "rules".to_owned(),
      sha256: source.to_owned(),
    }],
    options: BTreeMap::from([("build-number".to_owned(), build_number.to_owned())]),
  })
  .unwrap()
}

fn expect_pending(access: BuildAccess) -> battlement_tooling::build_cache::PendingBuild {
  match access {
    BuildAccess::Build(pending) => pending,
    BuildAccess::Reused(_) => panic!("expected an unpublished build"),
  }
}

fn populate(pending: &battlement_tooling::build_cache::PendingBuild, bytes: &[u8]) {
  fs::create_dir(pending.path().join("player.app")).unwrap();
  fs::write(pending.path().join("player.app/player.bin"), bytes).unwrap();
  fs::write(pending.path().join(BUILD_LOG_FILE), b"complete build log\n").unwrap();
  SourceManifest {
    fingerprint: pending.identity().source_fingerprint.clone(),
    entries: Vec::new(),
  }
  .write(&pending.path().join(SOURCE_MANIFEST_FILE))
  .unwrap();
}

fn entry_path(root: &Path, identity: &BuildIdentity) -> std::path::PathBuf {
  root.join("entries").join(&identity.fingerprint)
}
