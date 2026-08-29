//! Durable, atomic baseline acceptance from the local review application.

use std::{
  fs,
  path::{Path, PathBuf},
};

use anyhow::{Context, Error as AnyError, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
  baseline_store::{BaselineStore, write_atomic},
  baseline_update::{self, BaselineProposal, BaselineUpdateRequest, BaselineUpdateResult},
  config::model::Suite,
  review_acceptance_result, storage_commands,
  wire::{
    common::{ErrorCode, ErrorSource},
    lifecycle::HttpError,
    result::{BaselineWriteResult, ErrorOccurrence, RunResult, RunStatus},
    review::{ReviewAcceptance, ReviewAcceptanceResult, ReviewSelection},
    run_storage::{ActiveRun, RunStore},
  },
};

/// One HTTP-ready terminal acceptance response and optional replacement run.
pub struct AcceptanceReply {
  pub status: u16,
  pub body: Vec<u8>,
  pub replacement: Option<(PathBuf, RunResult)>,
}

/// Mutable authoring state for one local review session.
pub struct ReviewAcceptanceService {
  suite: Suite,
  store: RunStore,
  requests: PathBuf,
  reviewed: RunResult,
  reviewed_directory: PathBuf,
  baseline_store: Box<dyn BaselineStore>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoredReply {
  request_sha256: String,
  status: u16,
  body: Vec<u8>,
  comparison_run_id: Option<String>,
}

impl ReviewAcceptanceService {
  /// Opens write-capable review state after validating configured credentials.
  pub fn open(
    suite: Suite,
    runs_root: impl Into<PathBuf>,
    reviewed: RunResult,
    reviewed_directory: PathBuf,
  ) -> Result<Self> {
    let runs_root = runs_root.into();
    let baseline_store = storage_commands::write_store(&suite)?;
    ensure!(
      reviewed.suite.as_deref() == Some(suite.name.as_str()),
      "reviewed run belongs to another suite"
    );
    Ok(Self {
      requests: runs_root.join(".review-acceptance"),
      store: RunStore::open(runs_root)?,
      suite,
      reviewed,
      reviewed_directory,
      baseline_store,
    })
  }

  /// Applies or replays one exact JSON request body.
  pub fn accept(&mut self, bytes: &[u8]) -> AcceptanceReply {
    self
      .accept_inner(bytes)
      .unwrap_or_else(|error| AcceptanceReply {
        status: 500,
        body: json(&http_error(
          ErrorCode::DurabilityFailed,
          &format!("{error:#}"),
          None,
        )),
        replacement: None,
      })
  }

  fn accept_inner(&mut self, bytes: &[u8]) -> Result<AcceptanceReply> {
    let request: ReviewAcceptance = match serde_json::from_slice(bytes) {
      Ok(request) => request,
      Err(error) => {
        return Ok(error_reply(
          400,
          ErrorCode::ConfigurationInvalid,
          &error.to_string(),
          None,
        ));
      }
    };
    if Uuid::parse_str(&request.request_id).is_err() {
      return Ok(error_reply(
        400,
        ErrorCode::ConfigurationInvalid,
        "request_id must be a UUID",
        None,
      ));
    }
    if Uuid::parse_str(&request.run_id).is_err() {
      return Ok(error_reply(
        400,
        ErrorCode::ConfigurationInvalid,
        "run_id must be a UUID",
        None,
      ));
    }
    let request_sha256 = format!("{:x}", Sha256::digest(bytes));
    let record_path = self.record_path(&request);
    if let Some(mut reply) = replay(&record_path, &request_sha256, &mut self.store)? {
      if let Some((directory, result)) = &reply.replacement {
        let same_result = self.reviewed.run_id == result.run_id;
        if self.reviewed.run_id == request.run_id || same_result {
          self.reviewed = result.clone();
          self.reviewed_directory = directory.clone();
        } else {
          reply.replacement = None;
        }
      }
      return Ok(reply);
    }

    let now = review_acceptance_result::unix_time()?;
    let mut attempt = self.begin_attempt(now)?;
    if let Err(error) = self.store.materialize_derived(
      &attempt,
      &self.reviewed.run_id,
      &self.reviewed.artifacts,
      now,
    ) {
      let reply = self.finish_failure(attempt, AcceptanceFailure::durability(error), now)?;
      persist_reply(&record_path, &request_sha256, &reply)?;
      return Ok(reply);
    }
    let outcome = self.apply_request(&request, &mut attempt, now);
    let reply = match outcome {
      Ok(applied) => self.finish_success(&request, attempt, applied, now)?,
      Err(failure) => self.finish_failure(attempt, failure, now)?,
    };
    persist_reply(&record_path, &request_sha256, &reply)?;
    Ok(reply)
  }

  fn apply_request(
    &self,
    request: &ReviewAcceptance,
    _attempt: &mut ActiveRun,
    _now: u64,
  ) -> Result<BaselineUpdateResult, AcceptanceFailure> {
    request
      .validate(&self.reviewed)
      .map_err(AcceptanceFailure::invalid)?;
    review_acceptance_result::validate_current_suite(&self.suite, request)
      .map_err(AcceptanceFailure::invalid)?;
    let proposals = request
      .selections
      .iter()
      .map(|selection| self.proposal(selection))
      .collect::<Result<Vec<_>>>()
      .map_err(AcceptanceFailure::invalid)?;
    let authored = review_acceptance_result::authored_checkpoints(&self.suite);
    let scenarios = review_acceptance_result::group_proposals(proposals);
    let baseline = self
      .suite
      .baseline
      .as_ref()
      .context("suite has no baseline store")
      .map_err(AcceptanceFailure::invalid)?;
    baseline_update::apply(
      self.baseline_store.as_ref(),
      BaselineUpdateRequest {
        lock_path: &storage_commands::lock_path(&self.suite),
        starting_lock_sha256: request.lock_sha256.clone(),
        suite: &self.suite.name,
        namespace: storage_commands::namespace(baseline),
        profile: self
          .reviewed
          .profile
          .as_deref()
          .expect("validated run has a profile"),
        filtered: true,
        authored_checkpoints: &authored,
        scenarios: &scenarios,
      },
    )
    .map_err(AcceptanceFailure::update)
  }

  fn proposal(&self, selection: &ReviewSelection) -> Result<BaselineProposal> {
    let actual = review_acceptance_result::selected_actual(&self.reviewed, selection)?;
    let proposal = BaselineProposal::from_png(
      selection.scenario.clone(),
      selection.checkpoint.clone(),
      self.reviewed_directory.join(&actual.path),
      self.reviewed.build.as_ref().map_or_else(
        || actual.sha256.clone(),
        |build| build.source_fingerprint.clone(),
      ),
    )?;
    ensure!(
      proposal.sha256 == selection.actual_sha256,
      "selected actual image changed after the reviewed run"
    );
    ensure!(
      (proposal.width, proposal.height) == (selection.width, selection.height),
      "selected actual dimensions changed after the reviewed run"
    );
    Ok(proposal)
  }

  fn begin_attempt(&mut self, now: u64) -> Result<ActiveRun> {
    let result = review_acceptance_result::attempt_result(&self.reviewed, now)?;
    let mut progress = Vec::new();
    let active = self.store.begin(result, &mut progress, now)?;
    self
      .store
      .index_identity(&active, &self.suite.repository, &self.suite.name, now)?;
    Ok(active)
  }

  fn finish_success(
    &mut self,
    request: &ReviewAcceptance,
    mut attempt: ActiveRun,
    applied: BaselineUpdateResult,
    now: u64,
  ) -> Result<AcceptanceReply> {
    let mut result = review_acceptance_result::derived_result(
      &self.suite,
      &self.reviewed,
      request,
      &applied,
      attempt.path(),
      now,
    )?;
    self.store.finalize(&mut attempt, result.clone(), now)?;
    result = self.store.peek_result(&result.run_id)?;
    let directory = self.store.run_directory(&result.run_id)?;
    self.reviewed = result.clone();
    self.reviewed_directory = directory.clone();
    let response = ReviewAcceptanceResult {
      comparison_run_id: result.run_id.clone(),
      lock_sha256: applied.lock_sha256,
    };
    response.validate()?;
    Ok(AcceptanceReply {
      status: 200,
      body: json(&response),
      replacement: Some((directory, result)),
    })
  }

  fn finish_failure(
    &mut self,
    mut attempt: ActiveRun,
    failure: AcceptanceFailure,
    now: u64,
  ) -> Result<AcceptanceReply> {
    let run_id = attempt.run_id().to_owned();
    let mut result =
      review_acceptance_result::failure_result(&self.reviewed, &run_id, failure.writes, now)?;
    result.status = RunStatus::InfrastructureError;
    result.exit_code = 2;
    let error_id = format!("E{:04}", result.errors.len() + 1);
    result.errors.push(ErrorOccurrence {
      id: error_id,
      code: failure.code,
      source: ErrorSource::Ditto,
      message: bounded_message(&failure.message),
      job_id: None,
      player_session_id: None,
      scenario_id: None,
      step_index: None,
      log_sequence: None,
    });
    self.store.finalize(&mut attempt, result, now)?;
    Ok(error_reply(
      failure.status,
      failure.code,
      &failure.message,
      Some(run_id),
    ))
  }

  fn record_path(&self, request: &ReviewAcceptance) -> PathBuf {
    self
      .requests
      .join(&request.run_id)
      .join(format!("{}.json", request.request_id))
  }
}

struct AcceptanceFailure {
  status: u16,
  code: ErrorCode,
  message: String,
  writes: Vec<BaselineWriteResult>,
}

impl AcceptanceFailure {
  fn invalid(error: AnyError) -> Self {
    let message = format!("{error:#}");
    let stale = message.contains("lock digest is stale") || message.contains("starting ditto.lock");
    Self {
      status: if stale { 409 } else { 422 },
      code: if stale {
        ErrorCode::BaselineLockStale
      } else {
        ErrorCode::ConfigurationInvalid
      },
      message,
      writes: Vec::new(),
    }
  }

  fn update(failure: baseline_update::BaselineUpdateFailure) -> Self {
    let stale = failure.reason.contains("starting ditto.lock digest")
      || failure.reason.contains("ditto.lock changed while");
    let publish = failure.reason.contains("publish baseline object");
    Self {
      status: if stale { 409 } else { 500 },
      code: if stale {
        ErrorCode::BaselineLockStale
      } else if publish {
        ErrorCode::BaselinePublishFailed
      } else {
        ErrorCode::BaselineManifestWriteFailed
      },
      message: failure.reason,
      writes: failure.writes,
    }
  }

  fn durability(error: AnyError) -> Self {
    Self {
      status: 500,
      code: ErrorCode::DurabilityFailed,
      message: format!("{error:#}"),
      writes: Vec::new(),
    }
  }
}

fn replay(
  path: &Path,
  request_sha256: &str,
  store: &mut RunStore,
) -> Result<Option<AcceptanceReply>> {
  if !path.is_file() {
    return Ok(None);
  }
  let stored: StoredReply = serde_json::from_slice(&fs::read(path)?)?;
  if stored.request_sha256 != request_sha256 {
    return Ok(Some(error_reply(
      409,
      ErrorCode::BaselineStoreConflict,
      "request_id was reused with different request bytes",
      None,
    )));
  }
  let replacement = stored
    .comparison_run_id
    .as_deref()
    .map(|run_id| {
      let result = store.peek_result(run_id)?;
      let directory = store.run_directory(run_id)?;
      Ok::<_, AnyError>((directory, result))
    })
    .transpose()?;
  Ok(Some(AcceptanceReply {
    status: stored.status,
    body: stored.body,
    replacement,
  }))
}

fn persist_reply(path: &Path, request_sha256: &str, reply: &AcceptanceReply) -> Result<()> {
  write_atomic(
    path,
    &serde_json::to_vec(&StoredReply {
      request_sha256: request_sha256.to_owned(),
      status: reply.status,
      body: reply.body.clone(),
      comparison_run_id: reply
        .replacement
        .as_ref()
        .map(|(_, result)| result.run_id.clone()),
    })?,
  )
}

fn error_reply(
  status: u16,
  code: ErrorCode,
  message: &str,
  related_run_id: Option<String>,
) -> AcceptanceReply {
  AcceptanceReply {
    status,
    body: json(&http_error(code, message, related_run_id)),
    replacement: None,
  }
}

fn http_error(code: ErrorCode, message: &str, related_run_id: Option<String>) -> HttpError {
  HttpError {
    error_id: "E0001".to_owned(),
    code,
    message: bounded_message(message),
    expected_sequence: None,
    related_run_id,
  }
}

fn bounded_message(message: &str) -> String {
  message.chars().take(4096).collect()
}

fn json(value: &impl Serialize) -> Vec<u8> {
  serde_json::to_vec(value).expect("review response serializes")
}
