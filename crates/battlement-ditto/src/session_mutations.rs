use std::{
  collections::{BTreeMap, BTreeSet},
  fs::{self, File, OpenOptions},
  io::Write,
  path::{Path, PathBuf},
  sync::Arc,
};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::wire::{
  common::ErrorCode,
  job::Job,
  lifecycle::{
    ArtifactAck, DittoContext, DittoEventRecord, JobComplete, JobCompleteAck, JobFailed,
    JobFailedAck, LogBatchAck, NextAction, ScenarioComplete, ScenarioDecision, decode_ndjson,
  },
};

/// Finalizes player payloads after their transport prerequisites are durable.
pub trait PlayerSessionHandler: Send + Sync + 'static {
  /// Commits one scenario and returns its stored continuation decision.
  fn scenario_complete(&self, complete: &ScenarioComplete) -> Result<ScenarioDecision>;

  /// Finalizes one successful player job.
  fn job_complete(&self, complete: &JobComplete) -> Result<JobCompleteAck>;

  /// Finalizes one player-reported infrastructure failure.
  fn job_failed(&self, failed: &JobFailed, error_id: &str) -> Result<JobFailedAck>;
}

pub(crate) struct ContinueSessionHandler;

impl PlayerSessionHandler for ContinueSessionHandler {
  fn scenario_complete(&self, _: &ScenarioComplete) -> Result<ScenarioDecision> {
    Ok(ScenarioDecision {
      action: NextAction::Continue,
      completed_failures: 0,
      error_id: None,
      error_code: None,
      message: None,
    })
  }

  fn job_complete(&self, complete: &JobComplete) -> Result<JobCompleteAck> {
    Ok(JobCompleteAck {
      job_id: complete.job_id.clone(),
    })
  }

  fn job_failed(&self, failed: &JobFailed, error_id: &str) -> Result<JobFailedAck> {
    Ok(JobFailedAck {
      job_id: failed.job_id.clone(),
      error_id: error_id.to_owned(),
    })
  }
}

pub(crate) struct MutationState {
  job: Job,
  player_session_id: String,
  directory: PathBuf,
  handler: Arc<dyn PlayerSessionHandler>,
  expected_sequence: Option<u64>,
  logs: BTreeMap<u64, Stored>,
  artifacts: BTreeMap<String, StoredArtifact>,
  scenarios: BTreeMap<String, Stored>,
  observed_errors: BTreeSet<String>,
  terminal: Option<Terminal>,
}

pub(crate) struct MutationReply {
  pub status: u16,
  pub body: Vec<u8>,
  pub used_error_id: bool,
}

pub(crate) struct MutationError {
  pub status: u16,
  pub code: ErrorCode,
  pub message: String,
  pub expected_sequence: Option<u64>,
  pub terminal: bool,
}

struct Stored {
  request: Vec<u8>,
  response: Vec<u8>,
}

struct StoredArtifact {
  identity: String,
  response: Vec<u8>,
}

enum Terminal {
  Complete(Stored),
  Failed(Stored),
}

impl MutationState {
  pub fn new(
    job: Job,
    player_session_id: String,
    directory: PathBuf,
    handler: Arc<dyn PlayerSessionHandler>,
  ) -> Result<Self> {
    fs::create_dir_all(directory.join("logs"))?;
    fs::create_dir_all(directory.join("artifacts"))?;
    fs::create_dir_all(directory.join(".requests"))?;
    sync_directory(&directory)?;
    Ok(Self {
      job,
      player_session_id,
      directory,
      handler,
      expected_sequence: None,
      logs: BTreeMap::new(),
      artifacts: BTreeMap::new(),
      scenarios: BTreeMap::new(),
      observed_errors: BTreeSet::new(),
      terminal: None,
    })
  }

  pub fn accept_startup(
    &mut self,
    first_sequence: Option<u64>,
    request: &[u8],
    response: &[u8],
  ) -> Result<()> {
    persist(&self.directory, "started", request, response)?;
    self.expected_sequence = first_sequence;
    Ok(())
  }

  pub fn logs(&mut self, first: u64, body: &[u8]) -> Result<MutationReply, MutationError> {
    self.require_active()?;
    if let Some(stored) = self.logs.get(&first) {
      return if stored.request == body {
        Ok(MutationReply {
          status: 200,
          body: stored.response.clone(),
          used_error_id: false,
        })
      } else {
        Err(MutationError {
          status: 409,
          code: ErrorCode::TransportLogConflict,
          message: "conflicting log replay".to_owned(),
          expected_sequence: None,
          terminal: false,
        })
      };
    }
    let Some(expected) = self.expected_sequence else {
      return Err(conflict("job did not establish a first log sequence", None));
    };
    if first != expected {
      return Err(MutationError {
        status: 409,
        code: ErrorCode::TransportLogGap,
        message: "log sequence gap".to_owned(),
        expected_sequence: Some(expected),
        terminal: false,
      });
    }
    let records = decode_ndjson(body, &self.job, &self.player_session_id, first)
      .map_err(|error| bad_request(error.to_string()))?;
    let next = first
      .checked_add(records.len() as u64)
      .ok_or_else(|| bad_request("log sequence overflow".to_owned()))?;
    let response = json(&LogBatchAck {
      player_session_id: self.player_session_id.clone(),
      next_sequence: next,
    });
    self
      .persist_log(first, body, &response)
      .map_err(storage_error)?;
    for record in records {
      if let DittoEventRecord::Context(context) = record
        && let DittoContext::ErrorObserved { error_ref, .. } = context.body
      {
        self.observed_errors.insert(error_ref);
      }
    }
    self.logs.insert(
      first,
      Stored {
        request: body.to_vec(),
        response: response.clone(),
      },
    );
    self.expected_sequence = Some(next);
    Ok(MutationReply {
      status: 200,
      body: response,
      used_error_id: false,
    })
  }

  pub fn artifact(
    &mut self,
    artifact_id: &str,
    body: &[u8],
    declared_hash: &str,
    width: u32,
    height: u32,
  ) -> Result<MutationReply, MutationError> {
    self.require_active()?;
    let actual_hash = hex_hash(body);
    if actual_hash != declared_hash {
      return Err(bad_request(
        "artifact SHA-256 does not match its body".to_owned(),
      ));
    }
    let (decoded_width, decoded_height) = png_dimensions(body)
      .ok_or_else(|| bad_request("artifact is not a supported PNG".to_owned()))?;
    if (decoded_width, decoded_height) != (width, height) {
      return Err(bad_request(
        "artifact dimensions do not match its headers".to_owned(),
      ));
    }
    let identity = format!("{actual_hash}:{width}:{height}");
    if let Some(stored) = self.artifacts.get(artifact_id) {
      return if stored.identity == identity {
        Ok(MutationReply {
          status: 200,
          body: stored.response.clone(),
          used_error_id: false,
        })
      } else {
        Err(MutationError {
          status: 409,
          code: ErrorCode::TransportArtifactConflict,
          message: "conflicting artifact replay".to_owned(),
          expected_sequence: None,
          terminal: false,
        })
      };
    }
    let response = json(&ArtifactAck {
      artifact_id: artifact_id.to_owned(),
      sha256: actual_hash,
    });
    write_atomic(
      &self
        .directory
        .join("artifacts")
        .join(format!("{artifact_id}.png")),
      body,
    )
    .and_then(|()| {
      persist(
        &self.directory,
        &format!("artifact-{artifact_id}"),
        identity.as_bytes(),
        &response,
      )
    })
    .map_err(storage_error)?;
    self.artifacts.insert(
      artifact_id.to_owned(),
      StoredArtifact {
        identity,
        response: response.clone(),
      },
    );
    Ok(MutationReply {
      status: 201,
      body: response,
      used_error_id: false,
    })
  }

  pub fn scenario(
    &mut self,
    scenario_id: &str,
    body: &[u8],
  ) -> Result<MutationReply, MutationError> {
    self.require_active()?;
    if let Some(stored) = self.scenarios.get(scenario_id) {
      return replay(stored, body, "conflicting scenario completion");
    }
    let complete: ScenarioComplete = serde_json::from_slice(body)
      .map_err(|error| bad_request(format!("malformed scenario completion: {error}")))?;
    if complete.scenario_id != scenario_id {
      return Err(conflict(
        "scenario completion belongs to another route",
        None,
      ));
    }
    complete
      .validate(
        &self.job,
        &self.observed_errors.iter().cloned().collect::<Vec<_>>(),
      )
      .map_err(|error| bad_request(error.to_string()))?;
    self.require_log(complete.last_log_sequence)?;
    if !complete
      .artifacts
      .iter()
      .all(|artifact| self.artifacts.contains_key(&artifact.artifact_id))
    {
      return Err(conflict(
        "scenario references an artifact that was not uploaded",
        None,
      ));
    }
    let decision = self
      .handler
      .scenario_complete(&complete)
      .map_err(storage_error)?;
    decision.validate().map_err(storage_error)?;
    let response = json(&decision);
    persist(
      &self.directory,
      &format!("scenario-{scenario_id}"),
      body,
      &response,
    )
    .map_err(storage_error)?;
    self.scenarios.insert(
      scenario_id.to_owned(),
      Stored {
        request: body.to_vec(),
        response: response.clone(),
      },
    );
    Ok(MutationReply {
      status: 200,
      body: response,
      used_error_id: false,
    })
  }

  pub fn complete(&mut self, body: &[u8]) -> Result<MutationReply, MutationError> {
    if let Some(terminal) = &self.terminal {
      return match terminal {
        Terminal::Complete(stored) => replay(stored, body, "conflicting job completion"),
        Terminal::Failed(_) => Err(conflict("job failure is already terminal", None)),
      };
    }
    let complete: JobComplete = serde_json::from_slice(body)
      .map_err(|error| bad_request(format!("malformed job completion: {error}")))?;
    complete
      .validate(&self.job)
      .map_err(|error| bad_request(error.to_string()))?;
    self.require_log(complete.last_log_sequence)?;
    if !complete
      .executed_scenario_ids
      .iter()
      .all(|scenario| self.scenarios.contains_key(scenario))
    {
      return Err(conflict(
        "job completion references an uncommitted scenario",
        None,
      ));
    }
    let acknowledgement = self
      .handler
      .job_complete(&complete)
      .map_err(storage_error)?;
    acknowledgement
      .validate(&self.job.job_id)
      .map_err(storage_error)?;
    let response = json(&acknowledgement);
    persist(&self.directory, "job-complete", body, &response).map_err(storage_error)?;
    self.terminal = Some(Terminal::Complete(Stored {
      request: body.to_vec(),
      response: response.clone(),
    }));
    Ok(MutationReply {
      status: 200,
      body: response,
      used_error_id: false,
    })
  }

  pub fn failed(
    &mut self,
    body: &[u8],
    error_id: &str,
    uses_new_error_id: bool,
  ) -> Result<MutationReply, MutationError> {
    if let Some(terminal) = &self.terminal {
      return match terminal {
        Terminal::Failed(stored) => replay(stored, body, "conflicting job failure"),
        Terminal::Complete(_) => Err(conflict("job completion is already terminal", None)),
      };
    }
    let failed: JobFailed = serde_json::from_slice(body)
      .map_err(|error| bad_request(format!("malformed job failure: {error}")))?;
    failed
      .validate(&self.job)
      .map_err(|error| bad_request(error.to_string()))?;
    if let Some(sequence) = failed.last_log_sequence {
      self.require_log(sequence)?;
    }
    if !failed
      .executed_scenario_ids
      .iter()
      .all(|scenario| self.scenarios.contains_key(scenario))
    {
      return Err(conflict(
        "job failure references an uncommitted scenario",
        None,
      ));
    }
    let acknowledgement = self
      .handler
      .job_failed(&failed, error_id)
      .map_err(storage_error)?;
    acknowledgement
      .validate(&self.job.job_id)
      .map_err(storage_error)?;
    let response = json(&acknowledgement);
    persist(&self.directory, "job-failed", body, &response).map_err(storage_error)?;
    self.terminal = Some(Terminal::Failed(Stored {
      request: body.to_vec(),
      response: response.clone(),
    }));
    Ok(MutationReply {
      status: 200,
      body: response,
      used_error_id: uses_new_error_id,
    })
  }

  fn require_active(&self) -> Result<(), MutationError> {
    if self.terminal.is_none() {
      Ok(())
    } else {
      Err(MutationError {
        status: 410,
        code: ErrorCode::TransportRequestFailed,
        message: "player job has ended".to_owned(),
        expected_sequence: None,
        terminal: false,
      })
    }
  }

  fn require_log(&self, last: u64) -> Result<(), MutationError> {
    if self.expected_sequence == last.checked_add(1) {
      Ok(())
    } else {
      Err(conflict(
        "completion log sequence is not durable",
        self.expected_sequence,
      ))
    }
  }

  fn persist_log(&self, first: u64, body: &[u8], response: &[u8]) -> Result<()> {
    let path = self.directory.join("logs/events.jsonl");
    let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
    file.write_all(body)?;
    file.sync_all()?;
    persist(&self.directory, &format!("logs-{first}"), body, response)
  }
}

fn replay(stored: &Stored, body: &[u8], message: &str) -> Result<MutationReply, MutationError> {
  if stored.request == body {
    Ok(MutationReply {
      status: 200,
      body: stored.response.clone(),
      used_error_id: false,
    })
  } else {
    Err(conflict(message, None))
  }
}

fn bad_request(message: String) -> MutationError {
  MutationError {
    status: 400,
    code: ErrorCode::TransportRequestFailed,
    message,
    expected_sequence: None,
    terminal: false,
  }
}

fn conflict(message: &str, expected_sequence: Option<u64>) -> MutationError {
  MutationError {
    status: 409,
    code: expected_sequence.map_or(ErrorCode::TransportRequestFailed, |_| {
      ErrorCode::TransportLogGap
    }),
    message: message.to_owned(),
    expected_sequence,
    terminal: false,
  }
}

fn storage_error(error: anyhow::Error) -> MutationError {
  MutationError {
    status: 500,
    code: ErrorCode::DurabilityFailed,
    message: format!("durable session storage failed: {error}"),
    expected_sequence: None,
    terminal: true,
  }
}

fn json<T: serde::Serialize>(value: &T) -> Vec<u8> {
  serde_json::to_vec(value).expect("validated response must serialize")
}

fn hex_hash(bytes: &[u8]) -> String {
  Sha256::digest(bytes)
    .iter()
    .map(|byte| format!("{byte:02x}"))
    .collect()
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
  if bytes.len() < 24 || &bytes[..16] != b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR" {
    return None;
  }
  Some((
    u32::from_be_bytes(bytes[16..20].try_into().ok()?),
    u32::from_be_bytes(bytes[20..24].try_into().ok()?),
  ))
}

fn persist(directory: &Path, key: &str, request: &[u8], response: &[u8]) -> Result<()> {
  let requests = directory.join(".requests");
  write_atomic(&requests.join(format!("{key}.request")), request)?;
  write_atomic(&requests.join(format!("{key}.response")), response)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
  let parent = path.parent().context("durable path has no parent")?;
  let temporary = parent.join(format!(
    ".{}.{}.tmp",
    path.file_name().unwrap().to_string_lossy(),
    Uuid::new_v4()
  ));
  let mut file = OpenOptions::new()
    .create_new(true)
    .write(true)
    .open(&temporary)?;
  let result = (|| {
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    sync_directory(parent)
  })();
  if result.is_err() {
    let _ = fs::remove_file(temporary);
  }
  result
}

fn sync_directory(path: &Path) -> Result<()> {
  File::open(path)?.sync_all()?;
  Ok(())
}
