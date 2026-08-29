use std::{collections::BTreeMap, fs, sync::Mutex};

use anyhow::Result;
use battlement_ditto::{
  baseline_manifest::{BaselineEntry, BaselineManifest},
  baseline_publication::{
    ConditionalMutation, ConditionalObjectStore, R2CredentialNames, R2Credentials, StoredObject,
    clean_storage, publish, upload_immutable,
  },
  filesystem_publication_store::FilesystemPublicationStore,
};
use sha2::{Digest, Sha256};
use tempfile::tempdir;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

const NAMESPACE: &str = "samples/ui";
const STATE_KEY: &str = "samples/ui/metadata/state.json";
const LEASE_KEY: &str = "samples/ui/metadata/write-lease.json";

#[derive(Default)]
struct FakeR2 {
  inner: Mutex<FakeR2State>,
}

#[derive(Default)]
struct FakeR2State {
  objects: BTreeMap<String, StoredObject>,
  journal: Vec<String>,
  fail_next_state_confirm: bool,
  lose_next_lease_refresh: bool,
}

impl FakeR2 {
  fn insert(&self, key: &str, bytes: &[u8]) {
    self
      .inner
      .lock()
      .unwrap()
      .objects
      .insert(key.to_owned(), stored(bytes));
  }

  fn remove(&self, key: &str) {
    self.inner.lock().unwrap().objects.remove(key);
  }

  fn object(&self, key: &str) -> Option<StoredObject> {
    self.inner.lock().unwrap().objects.get(key).cloned()
  }

  fn journal(&self) -> Vec<String> {
    self.inner.lock().unwrap().journal.clone()
  }

  fn clear_journal(&self) {
    self.inner.lock().unwrap().journal.clear();
  }

  fn fail_next_state_confirm(&self) {
    self.inner.lock().unwrap().fail_next_state_confirm = true;
  }

  fn lose_next_lease_refresh(&self) {
    self.inner.lock().unwrap().lose_next_lease_refresh = true;
  }
}

impl ConditionalObjectStore for FakeR2 {
  fn get(&self, key: &str) -> Result<Option<StoredObject>> {
    let mut inner = self.inner.lock().unwrap();
    inner.journal.push(format!("GET {key}"));
    Ok(inner.objects.get(key).cloned())
  }

  fn confirm(&self, key: &str, etag: &str) -> Result<ConditionalMutation> {
    let mut inner = self.inner.lock().unwrap();
    inner.journal.push(format!("HEAD {key} If-Match"));
    if key == STATE_KEY && inner.fail_next_state_confirm {
      inner.fail_next_state_confirm = false;
      return Ok(ConditionalMutation::PreconditionFailed);
    }
    Ok(match inner.objects.get(key) {
      Some(object) if object.etag == etag => ConditionalMutation::Applied,
      _ => ConditionalMutation::PreconditionFailed,
    })
  }

  fn put_if_absent(&self, key: &str, bytes: &[u8]) -> Result<ConditionalMutation> {
    let mut inner = self.inner.lock().unwrap();
    inner.journal.push(format!("PUT {key} If-None-Match"));
    if inner.objects.contains_key(key) {
      return Ok(ConditionalMutation::PreconditionFailed);
    }
    inner.objects.insert(key.to_owned(), stored(bytes));
    Ok(ConditionalMutation::Applied)
  }

  fn put_if_match(&self, key: &str, etag: &str, bytes: &[u8]) -> Result<ConditionalMutation> {
    let mut inner = self.inner.lock().unwrap();
    inner.journal.push(format!("PUT {key} If-Match"));
    if key == LEASE_KEY && inner.lose_next_lease_refresh {
      inner.lose_next_lease_refresh = false;
      inner.objects.remove(key);
      return Ok(ConditionalMutation::PreconditionFailed);
    }
    let matches = inner
      .objects
      .get(key)
      .is_some_and(|object| object.etag == etag);
    if !matches {
      return Ok(ConditionalMutation::PreconditionFailed);
    }
    inner.objects.insert(key.to_owned(), stored(bytes));
    Ok(ConditionalMutation::Applied)
  }

  fn delete_if_match(&self, key: &str, etag: &str) -> Result<ConditionalMutation> {
    let mut inner = self.inner.lock().unwrap();
    inner.journal.push(format!("DELETE {key} If-Match"));
    let matches = inner
      .objects
      .get(key)
      .is_some_and(|object| object.etag == etag);
    if !matches {
      return Ok(ConditionalMutation::PreconditionFailed);
    }
    inner.objects.remove(key);
    Ok(ConditionalMutation::Applied)
  }

  fn delete(&self, key: &str) -> Result<()> {
    let mut inner = self.inner.lock().unwrap();
    inner.journal.push(format!("DELETE {key}"));
    inner.objects.remove(key);
    Ok(())
  }
}

#[test]
fn branch_acceptance_uploads_only_an_immutable_object() {
  let store = FakeR2::default();
  let bytes = b"accepted png";
  let sha256 = digest(bytes);

  upload_immutable(&store, NAMESPACE, &sha256, bytes).unwrap();

  assert!(store.object(&object_key(&sha256)).is_some());
  assert!(store.object(STATE_KEY).is_none());
  assert_eq!(
    store.journal(),
    vec![format!("PUT {} If-None-Match", object_key(&sha256))]
  );
}

#[test]
fn publication_tombstones_replacements_and_restoration_resets_retention() {
  let store = FakeR2::default();
  let first = upload(&store, b"first png");
  let second = upload(&store, b"second png");
  let start = at("2026-08-01T12:00:00Z");

  let initial = publish(&store, &manifest(&first), &digest(b"lock one"), start).unwrap();
  let replaced = publish(
    &store,
    &manifest(&second),
    &digest(b"lock two"),
    start + Duration::days(1),
  )
  .unwrap();
  let restored = publish(
    &store,
    &manifest(&first),
    &digest(b"lock three"),
    start + Duration::days(2),
  )
  .unwrap();

  assert_eq!(initial.state.generation, 1);
  assert_eq!(initial.state.published_at, "2026-08-01T12:00:00Z");
  assert_eq!(replaced.state.generation, 2);
  assert_eq!(replaced.state.tombstones[0].sha256, first);
  assert_eq!(restored.state.generation, 3);
  assert_eq!(restored.state.live_sha256, vec![first.clone()]);
  assert_eq!(restored.state.tombstones[0].sha256, second);
  assert!(
    !store
      .journal()
      .iter()
      .any(|entry| entry == &format!("DELETE {}", object_key(&first)))
  );
}

#[test]
fn active_remote_lease_rejects_a_competing_publisher_without_mutation() {
  let store = FakeR2::default();
  let sha256 = upload(&store, b"accepted png");
  store.insert(
    LEASE_KEY,
    br#"{"owner":"other","expires_at":"2026-08-01T12:01:00Z"}"#,
  );
  store.clear_journal();

  let error = publish(
    &store,
    &manifest(&sha256),
    &digest(b"lock"),
    at("2026-08-01T12:00:00Z"),
  )
  .unwrap_err();

  assert!(error.to_string().contains("lease is held"));
  assert_eq!(store.journal(), vec![format!("GET {LEASE_KEY}")]);
  assert!(store.object(STATE_KEY).is_none());
}

#[test]
fn credential_errors_name_only_the_configured_environment_variable() {
  let missing = format!("DITTO_MISSING_{}", uuid::Uuid::new_v4());
  let error = R2Credentials::from_environment(R2CredentialNames {
    account_id: &missing,
    bucket: "unused",
    access_key_id: "unused",
    secret_access_key: "unused",
  })
  .err()
  .unwrap();

  assert!(error.to_string().contains(&missing));
}

#[test]
fn lost_lease_aborts_before_state_replacement() {
  let store = FakeR2::default();
  let first = upload(&store, b"first png");
  let second = upload(&store, b"second png");
  let now = at("2026-08-01T12:00:00Z");
  publish(&store, &manifest(&first), &digest(b"lock one"), now).unwrap();
  let before = store.object(STATE_KEY).unwrap().bytes;
  store.lose_next_lease_refresh();

  let error = publish(
    &store,
    &manifest(&second),
    &digest(b"lock two"),
    now + Duration::days(1),
  )
  .unwrap_err();

  assert!(error.to_string().contains("lease was lost"));
  assert_eq!(store.object(STATE_KEY).unwrap().bytes, before);
  assert!(
    !store
      .journal()
      .iter()
      .any(|entry| entry.starts_with("DELETE samples/ui/objects"))
  );
}

#[test]
fn etag_race_aborts_cleanup_before_any_object_delete() {
  let (store, old, _, now) = retained_store();
  let before = store.object(STATE_KEY).unwrap().bytes;
  store.fail_next_state_confirm();

  let error = clean_storage(&store, NAMESPACE, now, true).unwrap_err();

  assert!(error.to_string().contains("ETag changed"));
  assert!(store.object(&object_key(&old)).is_some());
  assert_eq!(store.object(STATE_KEY).unwrap().bytes, before);
}

#[test]
fn dry_run_has_no_mutating_requests() {
  let (store, old, _, now) = retained_store();
  store.clear_journal();

  let result = clean_storage(&store, NAMESPACE, now, false).unwrap();

  assert_eq!(result.eligible_sha256, vec![old]);
  assert!(!result.applied);
  assert_eq!(store.journal(), vec![format!("GET {STATE_KEY}")]);
}

#[test]
fn cleanup_deletes_only_old_nonlive_objects_and_treats_missing_as_deleted() {
  let (store, old, recent, now) = retained_store();
  store.remove(&object_key(&old));
  store.clear_journal();

  let result = clean_storage(&store, NAMESPACE, now, true).unwrap();

  assert_eq!(result.eligible_sha256, vec![old.clone()]);
  assert!(result.applied);
  assert_eq!(result.state.generation, 4);
  assert_eq!(result.state.tombstones[0].sha256, recent.clone());
  assert!(store.object(&object_key(&recent)).is_some());
  assert!(
    store
      .journal()
      .contains(&format!("DELETE {}", object_key(&old)))
  );
}

#[test]
fn filesystem_publication_uses_canonical_state_and_an_advisory_lock() {
  let directory = tempdir().unwrap();
  let store = FilesystemPublicationStore::new(directory.path().to_owned());
  let bytes = b"filesystem png";
  let sha256 = digest(bytes);
  upload_immutable(&store, NAMESPACE, &sha256, bytes).unwrap();

  let result = publish(
    &store,
    &manifest(&sha256),
    &digest(b"filesystem lock"),
    at("2026-08-01T12:00:00Z"),
  )
  .unwrap();

  assert_eq!(
    fs::read(directory.path().join(STATE_KEY)).unwrap(),
    result.state.to_canonical_json().unwrap()
  );
  assert!(
    directory
      .path()
      .join("samples/ui/metadata/write.lock")
      .is_file()
  );
  assert!(!directory.path().join(LEASE_KEY).exists());
}

fn retained_store() -> (FakeR2, String, String, OffsetDateTime) {
  let store = FakeR2::default();
  let live = upload(&store, b"live png");
  let old = upload(&store, b"old png");
  let recent = upload(&store, b"recent png");
  let start = at("2026-08-01T12:00:00Z");
  publish(
    &store,
    &manifest_many(&[&live, &old, &recent]),
    &digest(b"lock one"),
    start,
  )
  .unwrap();
  publish(
    &store,
    &manifest_many(&[&live, &recent]),
    &digest(b"lock two"),
    start + Duration::days(1),
  )
  .unwrap();
  publish(
    &store,
    &manifest(&live),
    &digest(b"lock three"),
    start + Duration::days(7),
  )
  .unwrap();
  (store, old, recent, start + Duration::days(9))
}

fn upload(store: &FakeR2, bytes: &[u8]) -> String {
  let sha256 = digest(bytes);
  store.insert(&object_key(&sha256), bytes);
  sha256
}

fn manifest(sha256: &str) -> BaselineManifest {
  manifest_many(&[sha256])
}

fn manifest_many(hashes: &[&str]) -> BaselineManifest {
  BaselineManifest {
    suite: "ui".to_owned(),
    namespace: NAMESPACE.to_owned(),
    baselines: hashes
      .iter()
      .enumerate()
      .map(|(index, sha256)| BaselineEntry {
        profile: "canonical".to_owned(),
        scenario: format!("scenario-{index}"),
        checkpoint: "settled".to_owned(),
        sha256: (*sha256).to_owned(),
        width: 1,
        height: 1,
        size_bytes: 1,
        source: digest(format!("source-{index}").as_bytes()),
      })
      .collect(),
  }
}

fn object_key(sha256: &str) -> String {
  format!("{NAMESPACE}/objects/{}/{sha256}.png", &sha256[..2])
}

fn digest(bytes: &[u8]) -> String {
  format!("{:x}", Sha256::digest(bytes))
}

fn at(value: &str) -> OffsetDateTime {
  OffsetDateTime::parse(value, &Rfc3339).unwrap()
}

fn stored(bytes: &[u8]) -> StoredObject {
  StoredObject {
    bytes: bytes.to_vec(),
    etag: digest(bytes),
  }
}
