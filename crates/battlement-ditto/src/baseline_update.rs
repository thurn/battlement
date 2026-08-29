//! All-or-nothing baseline manifest updates.

use std::{
  collections::{BTreeMap, BTreeSet},
  error::Error as StdError,
  fmt::{Display, Formatter, Result as FmtResult},
  fs::{self, File, OpenOptions},
  path::{Path, PathBuf},
  result::Result as StdResult,
};

use anyhow::{Context, Error as AnyError, Result, ensure};
use fs2::FileExt;
use sha2::{Digest, Sha256};

use crate::{
  baseline_manifest::{
    BaselineEntry, BaselineManifest, ManifestSnapshot, digest, validate_namespace, validate_sha256,
  },
  baseline_store::{BaselineStore, write_atomic},
  wire::result::{BaselineWriteResult, BaselineWriteStatus},
};

/// Whether one selected scenario may contribute update proposals.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioUpdateStatus {
  Eligible,
  Failed,
  RuntimeSkipped,
}

/// One reached screenshot proposed for acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselineProposal {
  pub scenario: String,
  pub checkpoint: String,
  pub actual: PathBuf,
  pub sha256: String,
  pub width: u32,
  pub height: u32,
  pub size_bytes: u64,
  pub source: String,
}

/// The reached captures and terminal eligibility of one selected scenario.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioUpdate {
  pub name: String,
  pub status: ScenarioUpdateStatus,
  pub proposals: Vec<BaselineProposal>,
}

/// Complete authoring and starting-state inputs for one atomic update.
pub struct BaselineUpdateRequest<'a> {
  pub lock_path: &'a Path,
  pub starting_lock_sha256: Option<String>,
  pub suite: &'a str,
  pub namespace: &'a str,
  pub profile: &'a str,
  pub filtered: bool,
  pub authored_checkpoints: &'a BTreeMap<String, BTreeSet<String>>,
  pub scenarios: &'a [ScenarioUpdate],
}

/// Evidence from a successful manifest publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselineUpdateResult {
  pub lock_sha256: String,
  pub writes: Vec<BaselineWriteResult>,
}

/// A failed transaction together with every proposal's durable state.
#[derive(Debug)]
pub struct BaselineUpdateFailure {
  pub reason: String,
  pub writes: Vec<BaselineWriteResult>,
}

impl Display for BaselineUpdateFailure {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
    formatter.write_str(&self.reason)
  }
}

impl StdError for BaselineUpdateFailure {}

impl BaselineProposal {
  /// Builds verified metadata from exact PNG bytes.
  pub fn from_png(
    scenario: String,
    checkpoint: String,
    actual: PathBuf,
    source: String,
  ) -> Result<Self> {
    validate_sha256("baseline source", &source)?;
    let bytes = fs::read(&actual).context("read proposed baseline PNG")?;
    let (width, height) = png_dimensions(&bytes)?;
    Ok(Self {
      scenario,
      checkpoint,
      actual,
      sha256: format!("{:x}", Sha256::digest(&bytes)),
      width,
      height,
      size_bytes: bytes.len().try_into()?,
      source,
    })
  }

  fn verify(&self) -> Result<()> {
    ensure!(
      !self.scenario.trim().is_empty(),
      "proposal scenario must not be empty"
    );
    ensure!(
      !self.checkpoint.trim().is_empty(),
      "proposal checkpoint must not be empty"
    );
    validate_sha256("baseline sha256", &self.sha256)?;
    validate_sha256("baseline source", &self.source)?;
    let bytes = fs::read(&self.actual).context("read proposed baseline PNG")?;
    let (width, height) = png_dimensions(&bytes)?;
    ensure!(
      self.sha256 == format!("{:x}", Sha256::digest(&bytes)),
      "proposed baseline hash changed"
    );
    ensure!(
      self.width == width && self.height == height,
      "proposed baseline dimensions changed"
    );
    ensure!(
      self.size_bytes == bytes.len() as u64,
      "proposed baseline size changed"
    );
    Ok(())
  }
}

/// Publishes eligible objects and replaces the manifest exactly once.
pub fn apply(
  store: &dyn BaselineStore,
  request: BaselineUpdateRequest<'_>,
) -> StdResult<BaselineUpdateResult, BaselineUpdateFailure> {
  apply_inner(store, request).map_err(|(error, writes)| BaselineUpdateFailure {
    reason: format!("{error:#}"),
    writes,
  })
}

fn apply_inner(
  store: &dyn BaselineStore,
  request: BaselineUpdateRequest<'_>,
) -> StdResult<BaselineUpdateResult, (AnyError, Vec<BaselineWriteResult>)> {
  let mut writes = Vec::new();
  let result = (|| -> Result<BaselineUpdateResult> {
    validate_request(&request)?;
    let starting = ManifestSnapshot::read(request.lock_path)?;
    ensure!(
      starting.sha256 == request.starting_lock_sha256,
      "starting ditto.lock digest does not match"
    );
    let mut manifest = starting.manifest.unwrap_or(BaselineManifest {
      suite: request.suite.to_owned(),
      namespace: request.namespace.to_owned(),
      baselines: Vec::new(),
    });
    ensure!(
      manifest.suite == request.suite,
      "ditto.lock suite does not match"
    );
    ensure!(
      manifest.namespace == request.namespace,
      "ditto.lock namespace does not match"
    );
    prune(&mut manifest, &request);
    collect_proposals(&mut manifest, &request, &mut writes)?;

    let _lease = suite_lease(request.lock_path)?;
    ensure!(
      ManifestSnapshot::read(request.lock_path)?.sha256 == request.starting_lock_sha256,
      "ditto.lock changed while the update was running"
    );
    for proposal in eligible_proposals(&request) {
      let Some(write_index) = writes.iter().position(|write| {
        write.scenario == proposal.scenario && write.checkpoint == proposal.checkpoint
      }) else {
        continue;
      };
      store
        .put(request.namespace, &proposal.sha256, &proposal.actual)
        .context("publish baseline object")?;
      writes[write_index].status = BaselineWriteStatus::UploadedUnreferenced;
    }
    let bytes = manifest.canonical_bytes()?;
    write_atomic(request.lock_path, &bytes).context("replace ditto.lock")?;
    for write in &mut writes {
      write.status = BaselineWriteStatus::Published;
    }
    Ok(BaselineUpdateResult {
      lock_sha256: digest(&bytes),
      writes: writes.clone(),
    })
  })();
  result.map_err(|error| (error, writes))
}

fn validate_request(request: &BaselineUpdateRequest<'_>) -> Result<()> {
  ensure!(!request.suite.trim().is_empty(), "suite must not be empty");
  validate_namespace(request.namespace)?;
  ensure!(
    !request.profile.trim().is_empty(),
    "profile must not be empty"
  );
  let mut selected = BTreeSet::new();
  let mut proposal_identities = BTreeSet::new();
  for scenario in request.scenarios {
    ensure!(
      selected.insert(&scenario.name),
      "duplicate selected scenario"
    );
    ensure!(
      request.authored_checkpoints.contains_key(&scenario.name),
      "selected scenario is absent from the full suite"
    );
    for proposal in &scenario.proposals {
      proposal.verify()?;
      ensure!(
        proposal.scenario == scenario.name,
        "proposal scenario does not match"
      );
      ensure!(
        request.authored_checkpoints[&scenario.name].contains(&proposal.checkpoint),
        "proposal checkpoint is absent from the full suite"
      );
      ensure!(
        proposal_identities.insert((&proposal.scenario, &proposal.checkpoint)),
        "duplicate baseline proposal"
      );
    }
  }
  if !request.filtered {
    ensure!(
      selected.len() == request.authored_checkpoints.len(),
      "unfiltered update must include every authored scenario"
    );
  }
  Ok(())
}

fn prune(manifest: &mut BaselineManifest, request: &BaselineUpdateRequest<'_>) {
  let selected: BTreeSet<_> = request
    .scenarios
    .iter()
    .map(|scenario| scenario.name.as_str())
    .collect();
  manifest.baselines.retain(|entry| {
    if entry.profile != request.profile {
      return true;
    }
    if request.filtered && !selected.contains(entry.scenario.as_str()) {
      return true;
    }
    request
      .authored_checkpoints
      .get(&entry.scenario)
      .is_some_and(|checkpoints| checkpoints.contains(&entry.checkpoint))
  });
}

fn collect_proposals(
  manifest: &mut BaselineManifest,
  request: &BaselineUpdateRequest<'_>,
  writes: &mut Vec<BaselineWriteResult>,
) -> Result<()> {
  for proposal in eligible_proposals(request) {
    if manifest
      .find(request.profile, &proposal.scenario, &proposal.checkpoint)
      .is_some_and(|entry| entry.sha256 == proposal.sha256)
    {
      continue;
    }
    let entry = BaselineEntry {
      profile: request.profile.to_owned(),
      scenario: proposal.scenario.clone(),
      checkpoint: proposal.checkpoint.clone(),
      sha256: proposal.sha256.clone(),
      width: proposal.width,
      height: proposal.height,
      size_bytes: proposal.size_bytes,
      source: proposal.source.clone(),
    };
    manifest.baselines.retain(|current| {
      current.profile != entry.profile
        || current.scenario != entry.scenario
        || current.checkpoint != entry.checkpoint
    });
    manifest.baselines.push(entry);
    writes.push(BaselineWriteResult {
      sha256: proposal.sha256.clone(),
      profile: request.profile.to_owned(),
      scenario: proposal.scenario.clone(),
      checkpoint: proposal.checkpoint.clone(),
      status: BaselineWriteStatus::Proposed,
    });
  }
  Ok(())
}

fn eligible_proposals<'a>(
  request: &'a BaselineUpdateRequest<'_>,
) -> impl Iterator<Item = &'a BaselineProposal> {
  request
    .scenarios
    .iter()
    .filter_map(|scenario| {
      (scenario.status == ScenarioUpdateStatus::Eligible).then_some(&scenario.proposals)
    })
    .flatten()
}

fn suite_lease(lock_path: &Path) -> Result<File> {
  let parent = lock_path.parent().context("ditto.lock has no parent")?;
  fs::create_dir_all(parent)?;
  let lease_path = parent.join(".ditto.lock.lease");
  let lease = OpenOptions::new()
    .create(true)
    .read(true)
    .write(true)
    .truncate(false)
    .open(lease_path)?;
  lease.lock_exclusive()?;
  Ok(lease)
}

fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32)> {
  ensure!(
    bytes.len() >= 24 && &bytes[..16] == b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR",
    "proposed baseline is not a PNG"
  );
  let width = u32::from_be_bytes(bytes[16..20].try_into()?);
  let height = u32::from_be_bytes(bytes[20..24].try_into()?);
  ensure!(
    width > 0 && height > 0,
    "proposed baseline dimensions must be positive"
  );
  Ok((width, height))
}
