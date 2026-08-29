//! Conditional publication and retained cleanup of baseline namespaces.

use std::{
  collections::{BTreeMap, BTreeSet},
  env,
};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{
  baseline_manifest::{BaselineManifest, validate_namespace, validate_sha256},
  wire::baseline_state::{BaselineStoreState, BaselineTombstone},
};

const LEASE_SECONDS: i64 = 60;
const RETENTION_DAYS: i64 = 7;

/// Minimal conditional object operations needed for safe namespace mutation.
pub trait ConditionalObjectStore: Send + Sync {
  /// Reads an object and its current entity tag.
  fn get(&self, key: &str) -> Result<Option<StoredObject>>;
  /// Confirms that an object still has the supplied entity tag.
  fn confirm(&self, key: &str, etag: &str) -> Result<ConditionalMutation>;
  /// Creates an object only when its key is absent.
  fn put_if_absent(&self, key: &str, bytes: &[u8]) -> Result<ConditionalMutation>;
  /// Replaces an object only when its entity tag matches.
  fn put_if_match(&self, key: &str, etag: &str, bytes: &[u8]) -> Result<ConditionalMutation>;
  /// Deletes an object only when its entity tag matches.
  fn delete_if_match(&self, key: &str, etag: &str) -> Result<ConditionalMutation>;
  /// Deletes an object, treating absence as success.
  fn delete(&self, key: &str) -> Result<()>;
}

/// One conditionally readable object and its opaque entity tag.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredObject {
  pub bytes: Vec<u8>,
  pub etag: String,
}

/// The result of an object precondition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConditionalMutation {
  Applied,
  PreconditionFailed,
}

/// Environment variable names for bucket-scoped R2 write credentials.
pub struct R2CredentialNames<'a> {
  /// Name of the variable containing the Cloudflare account identifier.
  pub account_id: &'a str,
  /// Name of the variable containing the bucket name.
  pub bucket: &'a str,
  /// Name of the variable containing the bucket-scoped access key ID.
  pub access_key_id: &'a str,
  /// Name of the variable containing the bucket-scoped secret access key.
  pub secret_access_key: &'a str,
}

/// Secret R2 values loaded only by write-capable commands.
pub struct R2Credentials {
  account_id: String,
  bucket: String,
  access_key_id: String,
  secret_access_key: String,
}

/// Publication state plus nonfatal lease-release diagnostics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationResult {
  pub state: BaselineStoreState,
  pub warnings: Vec<String>,
}

/// A dry-run or applied retained-object cleanup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CleanupResult {
  pub eligible_sha256: Vec<String>,
  pub applied: bool,
  pub state: BaselineStoreState,
  pub warnings: Vec<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WriteLease {
  owner: String,
  expires_at: String,
}

struct Lease<'a> {
  store: &'a dyn ConditionalObjectStore,
  key: String,
  owner: String,
  etag: String,
}

impl R2Credentials {
  /// Loads only the explicitly named environment variables.
  pub fn from_environment(names: R2CredentialNames<'_>) -> Result<Self> {
    Ok(Self {
      account_id: required_environment(names.account_id)?,
      bucket: required_environment(names.bucket)?,
      access_key_id: required_environment(names.access_key_id)?,
      secret_access_key: required_environment(names.secret_access_key)?,
    })
  }

  /// Returns the R2 account identifier.
  pub fn account_id(&self) -> &str {
    &self.account_id
  }

  /// Returns the configured bucket.
  pub fn bucket(&self) -> &str {
    &self.bucket
  }

  pub(crate) fn access_key_id(&self) -> &str {
    &self.access_key_id
  }

  pub(crate) fn secret_access_key(&self) -> &str {
    &self.secret_access_key
  }
}

/// Uploads one immutable content-addressed PNG without changing namespace state.
pub fn upload_immutable(
  store: &dyn ConditionalObjectStore,
  namespace: &str,
  sha256: &str,
  bytes: &[u8],
) -> Result<()> {
  validate_namespace(namespace)?;
  validate_sha256("baseline sha256", sha256)?;
  ensure!(
    format!("{:x}", Sha256::digest(bytes)) == sha256,
    "proposed baseline hash mismatch"
  );
  let key = object_key(namespace, sha256);
  if store.put_if_absent(&key, bytes)? == ConditionalMutation::Applied {
    return Ok(());
  }
  let existing = store.get(&key)?.context("immutable baseline disappeared")?;
  ensure!(
    format!("{:x}", Sha256::digest(&existing.bytes)) == sha256,
    "immutable baseline object conflicts with its digest"
  );
  Ok(())
}

/// Publishes one canonical namespace state after verifying every live object.
pub fn publish(
  store: &dyn ConditionalObjectStore,
  manifest: &BaselineManifest,
  lock_sha256: &str,
  now: OffsetDateTime,
) -> Result<PublicationResult> {
  manifest.canonical_bytes()?;
  validate_sha256("lock sha256", lock_sha256)?;
  let mut lease = Lease::acquire(store, &manifest.namespace, now)?;
  let result = publish_inner(&mut lease, manifest, lock_sha256, now);
  finish_publication(result, lease.release())
}

/// Plans or applies deletion of objects retained for at least seven days.
pub fn clean_storage(
  store: &dyn ConditionalObjectStore,
  namespace: &str,
  now: OffsetDateTime,
  apply: bool,
) -> Result<CleanupResult> {
  validate_namespace(namespace)?;
  if !apply {
    let state = read_state(store, namespace)?
      .context("baseline state is missing")?
      .0;
    return Ok(cleanup_result(state, now, false));
  }
  let mut lease = Lease::acquire(store, namespace, now)?;
  let result = clean_inner(&mut lease, namespace, now);
  finish_cleanup(result, lease.release())
}

fn publish_inner(
  lease: &mut Lease<'_>,
  manifest: &BaselineManifest,
  lock_sha256: &str,
  now: OffsetDateTime,
) -> Result<PublicationResult> {
  let previous = read_state(lease.store, &manifest.namespace)?;
  let live: BTreeSet<_> = manifest
    .baselines
    .iter()
    .map(|entry| entry.sha256.clone())
    .collect();
  for sha256 in &live {
    let object = lease
      .store
      .get(&object_key(&manifest.namespace, sha256))?
      .with_context(|| format!("published baseline object {sha256} is missing"))?;
    ensure!(
      format!("{:x}", Sha256::digest(&object.bytes)) == *sha256,
      "published baseline object {sha256} has the wrong hash"
    );
    lease.refresh()?;
  }
  let state = next_publication(
    previous.as_ref().map(|value| &value.0),
    live,
    lock_sha256,
    now,
  )?;
  lease.refresh()?;
  write_state(
    lease.store,
    &manifest.namespace,
    &state,
    previous.as_ref().map(|value| value.1.as_str()),
  )?;
  Ok(PublicationResult {
    state,
    warnings: Vec::new(),
  })
}

fn clean_inner(
  lease: &mut Lease<'_>,
  namespace: &str,
  now: OffsetDateTime,
) -> Result<CleanupResult> {
  let (previous, etag) =
    read_state(lease.store, namespace)?.context("baseline state is missing")?;
  let mut result = cleanup_result(previous.clone(), now, true);
  ensure!(
    lease.store.confirm(&state_key(namespace), &etag)? == ConditionalMutation::Applied,
    "baseline state ETag changed before cleanup"
  );
  for sha256 in &result.eligible_sha256 {
    lease.refresh()?;
    ensure!(
      lease.store.confirm(&state_key(namespace), &etag)? == ConditionalMutation::Applied,
      "baseline state ETag changed before cleanup"
    );
    lease.store.delete(&object_key(namespace, sha256))?;
  }
  result.state.generation = result
    .state
    .generation
    .checked_add(1)
    .context("baseline state generation overflow")?;
  result.state.published_at = timestamp(now)?;
  result.state.cleanup_applied_at = Some(result.state.published_at.clone());
  result
    .state
    .tombstones
    .retain(|value| !result.eligible_sha256.contains(&value.sha256));
  result.state.validate(Some(&previous))?;
  lease.refresh()?;
  write_state(lease.store, namespace, &result.state, Some(&etag))?;
  Ok(result)
}

fn next_publication(
  previous: Option<&BaselineStoreState>,
  live: BTreeSet<String>,
  lock_sha256: &str,
  now: OffsetDateTime,
) -> Result<BaselineStoreState> {
  let published_at = timestamp(now)?;
  let mut tombstones: BTreeMap<_, _> = previous
    .map(|state| {
      state
        .tombstones
        .iter()
        .map(|value| (value.sha256.clone(), value.removed_at.clone()))
        .collect()
    })
    .unwrap_or_default();
  if let Some(previous) = previous {
    for sha256 in &previous.live_sha256 {
      if !live.contains(sha256) {
        tombstones
          .entry(sha256.clone())
          .or_insert_with(|| published_at.clone());
      }
    }
  }
  for sha256 in &live {
    tombstones.remove(sha256);
  }
  let generation = match previous {
    Some(state) => state
      .generation
      .checked_add(1)
      .context("baseline state generation overflow")?,
    None => 1,
  };
  let state = BaselineStoreState {
    generation,
    lock_sha256: lock_sha256.to_owned(),
    published_at,
    live_sha256: live.into_iter().collect(),
    tombstones: tombstones
      .into_iter()
      .map(|(sha256, removed_at)| BaselineTombstone { sha256, removed_at })
      .collect(),
    cleanup_applied_at: previous.and_then(|state| state.cleanup_applied_at.clone()),
  };
  state.validate(previous)?;
  Ok(state)
}

fn cleanup_result(state: BaselineStoreState, now: OffsetDateTime, applied: bool) -> CleanupResult {
  let cutoff = now - Duration::days(RETENTION_DAYS);
  let live: BTreeSet<_> = state.live_sha256.iter().collect();
  let eligible_sha256 = state
    .tombstones
    .iter()
    .filter(|value| !live.contains(&value.sha256))
    .filter(|value| parse_timestamp(&value.removed_at).is_ok_and(|removed_at| removed_at <= cutoff))
    .map(|value| value.sha256.clone())
    .collect();
  CleanupResult {
    eligible_sha256,
    applied,
    state,
    warnings: Vec::new(),
  }
}

fn read_state(
  store: &dyn ConditionalObjectStore,
  namespace: &str,
) -> Result<Option<(BaselineStoreState, String)>> {
  store
    .get(&state_key(namespace))?
    .map(|object| {
      let state: BaselineStoreState = serde_json::from_slice(&object.bytes)?;
      state.validate_shape()?;
      Ok((state, object.etag))
    })
    .transpose()
}

fn write_state(
  store: &dyn ConditionalObjectStore,
  namespace: &str,
  state: &BaselineStoreState,
  etag: Option<&str>,
) -> Result<()> {
  let bytes = state.to_canonical_json()?;
  let mutation = match etag {
    Some(etag) => store.put_if_match(&state_key(namespace), etag, &bytes)?,
    None => store.put_if_absent(&state_key(namespace), &bytes)?,
  };
  ensure!(
    mutation == ConditionalMutation::Applied,
    "baseline state ETag changed"
  );
  Ok(())
}

impl<'a> Lease<'a> {
  fn acquire(
    store: &'a dyn ConditionalObjectStore,
    namespace: &str,
    now: OffsetDateTime,
  ) -> Result<Self> {
    validate_namespace(namespace)?;
    let key = lease_key(namespace);
    let current = store.get(&key)?;
    if let Some(current) = &current {
      let lease: WriteLease = serde_json::from_slice(&current.bytes)?;
      ensure!(
        parse_timestamp(&lease.expires_at)? <= now,
        "baseline mutation lease is held"
      );
    }
    let owner = Uuid::new_v4().to_string();
    let bytes = lease_bytes(&owner, now)?;
    let mutation = match &current {
      Some(current) => store.put_if_match(&key, &current.etag, &bytes)?,
      None => store.put_if_absent(&key, &bytes)?,
    };
    ensure!(
      mutation == ConditionalMutation::Applied,
      "baseline mutation lease changed"
    );
    let etag = owned_lease(store, &key, &owner)?.etag;
    Ok(Self {
      store,
      key,
      owner,
      etag,
    })
  }

  fn refresh(&mut self) -> Result<()> {
    let bytes = lease_bytes(&self.owner, OffsetDateTime::now_utc())?;
    ensure!(
      self.store.put_if_match(&self.key, &self.etag, &bytes)? == ConditionalMutation::Applied,
      "baseline mutation lease was lost"
    );
    self.etag = owned_lease(self.store, &self.key, &self.owner)?.etag;
    Ok(())
  }

  fn release(self) -> Result<()> {
    ensure!(
      self.store.delete_if_match(&self.key, &self.etag)? == ConditionalMutation::Applied,
      "baseline mutation lease was lost before release"
    );
    Ok(())
  }
}

fn owned_lease(store: &dyn ConditionalObjectStore, key: &str, owner: &str) -> Result<StoredObject> {
  let object = store
    .get(key)?
    .context("baseline mutation lease disappeared")?;
  let lease: WriteLease = serde_json::from_slice(&object.bytes)?;
  ensure!(
    lease.owner == owner,
    "baseline mutation lease owner changed"
  );
  Ok(object)
}

fn finish_publication(
  result: Result<PublicationResult>,
  release: Result<()>,
) -> Result<PublicationResult> {
  match (result, release) {
    (Ok(mut result), Err(error)) => {
      result
        .warnings
        .push(format!("release baseline lease: {error:#}"));
      Ok(result)
    }
    (Ok(result), Ok(())) => Ok(result),
    (Err(error), _) => Err(error),
  }
}

fn finish_cleanup(result: Result<CleanupResult>, release: Result<()>) -> Result<CleanupResult> {
  match (result, release) {
    (Ok(mut result), Err(error)) => {
      result
        .warnings
        .push(format!("release baseline lease: {error:#}"));
      Ok(result)
    }
    (Ok(result), Ok(())) => Ok(result),
    (Err(error), _) => Err(error),
  }
}

fn required_environment(name: &str) -> Result<String> {
  ensure!(
    !name.is_empty(),
    "R2 credential environment variable name is empty"
  );
  env::var(name)
    .with_context(|| format!("required R2 credential environment variable {name} is missing"))
}

fn object_key(namespace: &str, sha256: &str) -> String {
  format!("{namespace}/objects/{}/{sha256}.png", &sha256[..2])
}

fn state_key(namespace: &str) -> String {
  format!("{namespace}/metadata/state.json")
}

fn lease_key(namespace: &str) -> String {
  format!("{namespace}/metadata/write-lease.json")
}

fn lease_bytes(owner: &str, now: OffsetDateTime) -> Result<Vec<u8>> {
  Ok(serde_json::to_vec(&WriteLease {
    owner: owner.to_owned(),
    expires_at: timestamp(now + Duration::seconds(LEASE_SECONDS))?,
  })?)
}

fn timestamp(value: OffsetDateTime) -> Result<String> {
  Ok(value.replace_nanosecond(0)?.format(&Rfc3339)?)
}

fn parse_timestamp(value: &str) -> Result<OffsetDateTime> {
  OffsetDateTime::parse(value, &Rfc3339).context("parse baseline timestamp")
}
