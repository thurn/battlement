use std::{iter, path::Path};

use anyhow::Result;

use crate::{
  discovery::{DiscoveryRequest, HostDiscovery, Tool},
  host::{FilesystemOperation, Host},
};

/// A stable grouping in human and machine doctor output.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckCategory {
  Required,
  Optional,
  ReadOnly,
  Write,
}

/// Outcome of one independent host check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckStatus {
  Passed,
  Warning,
  Failed,
}

/// One credential-safe doctor finding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorCheck {
  pub category: CheckCategory,
  pub name: String,
  pub status: CheckStatus,
  pub detail: String,
}

/// Inputs determining which optional and write checks are mandatory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorRequest {
  pub discovery: DiscoveryRequest,
  pub write_required: bool,
  pub minimum_available_bytes: Option<u64>,
  pub secret_environment_names: Vec<String>,
}

/// Complete categorized result for `ditto doctor`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorReport {
  pub discovery: HostDiscovery,
  pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
  /// Runs independent tool, cache-read, cache-write, and capacity checks.
  pub fn inspect(host: &impl Host, request: &DoctorRequest) -> Result<Self> {
    let mut discovery = HostDiscovery::inspect(host, &request.discovery)?;
    let secrets = request
      .secret_environment_names
      .iter()
      .filter_map(|name| host.environment(name))
      .filter(|value| !value.is_empty())
      .collect::<Vec<_>>();
    let mut checks = Vec::new();
    for tool in iter::once(&discovery.unity)
      .chain(discovery.apple.iter())
      .chain(iter::once(&discovery.odiff))
      .chain(iter::once(&discovery.ffmpeg))
    {
      checks.push(tool_check(tool, &secrets));
    }
    for (name, path) in cache_paths(&discovery) {
      checks.push(directory_check(
        host,
        CheckCategory::ReadOnly,
        name,
        path,
        FilesystemOperation::Read,
        true,
        &secrets,
      ));
      checks.push(directory_check(
        host,
        CheckCategory::Write,
        name,
        path,
        FilesystemOperation::Write,
        request.write_required,
        &secrets,
      ));
    }
    if let Some(required) = request.minimum_available_bytes {
      checks.push(capacity_check(
        host,
        &discovery.caches.root,
        required,
        request.write_required,
        &secrets,
      ));
    }
    redact_discovery(&mut discovery, &secrets);
    Ok(Self { discovery, checks })
  }

  /// Returns whether every mandatory check passed.
  pub fn healthy(&self) -> bool {
    self
      .checks
      .iter()
      .all(|check| check.status != CheckStatus::Failed)
  }
}

fn tool_check(tool: &Tool, secrets: &[String]) -> DoctorCheck {
  let category = if tool.required {
    CheckCategory::Required
  } else {
    CheckCategory::Optional
  };
  let status = if tool.ready() {
    CheckStatus::Passed
  } else if tool.required {
    CheckStatus::Failed
  } else {
    CheckStatus::Warning
  };
  let mut detail = tool
    .problem
    .clone()
    .or_else(|| tool.version.clone())
    .unwrap_or_else(|| "available".to_owned());
  if let Some(path) = &tool.path {
    detail.push_str(&format!(" at {}", path.display()));
  }
  if !tool.alternatives.is_empty() {
    detail.push_str("; installed alternatives: ");
    detail.push_str(
      &tool
        .alternatives
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", "),
    );
  }
  DoctorCheck {
    category,
    name: tool.name.clone(),
    status,
    detail: redact(&detail, secrets),
  }
}

fn cache_paths(discovery: &HostDiscovery) -> [(&str, &Path); 4] {
  [
    ("run cache", &discovery.caches.runs),
    ("build cache", &discovery.caches.builds),
    ("baseline cache", &discovery.caches.baselines),
    ("tool cache", &discovery.caches.tools),
  ]
}

fn directory_check(
  host: &impl Host,
  category: CheckCategory,
  name: &str,
  path: &Path,
  operation: FilesystemOperation,
  required: bool,
  secrets: &[String],
) -> DoctorCheck {
  match host.check_directory(path, operation) {
    Ok(()) => DoctorCheck {
      category,
      name: name.to_owned(),
      status: CheckStatus::Passed,
      detail: redact(&path.display().to_string(), secrets),
    },
    Err(error) => DoctorCheck {
      category,
      name: name.to_owned(),
      status: if required {
        CheckStatus::Failed
      } else {
        CheckStatus::Warning
      },
      detail: redact(&format!("{}: {error:#}", path.display()), secrets),
    },
  }
}

fn capacity_check(
  host: &impl Host,
  path: &Path,
  required: u64,
  mandatory: bool,
  secrets: &[String],
) -> DoctorCheck {
  let (status, detail) = match host.available_bytes(path) {
    Ok(available) if available >= required => (
      CheckStatus::Passed,
      format!("{available} bytes available; {required} required"),
    ),
    Ok(available) => (
      if mandatory {
        CheckStatus::Failed
      } else {
        CheckStatus::Warning
      },
      format!("{available} bytes available; {required} required"),
    ),
    Err(error) => (
      if mandatory {
        CheckStatus::Failed
      } else {
        CheckStatus::Warning
      },
      format!("capacity check failed: {error:#}"),
    ),
  };
  DoctorCheck {
    category: CheckCategory::Write,
    name: "recording capacity".to_owned(),
    status,
    detail: redact(&detail, secrets),
  }
}

fn redact(value: &str, secrets: &[String]) -> String {
  secrets.iter().fold(value.to_owned(), |redacted, secret| {
    redacted.replace(secret, "<redacted>")
  })
}

fn redact_discovery(discovery: &mut HostDiscovery, secrets: &[String]) {
  for tool in iter::once(&mut discovery.unity)
    .chain(discovery.apple.iter_mut())
    .chain(iter::once(&mut discovery.odiff))
    .chain(iter::once(&mut discovery.ffmpeg))
  {
    tool.path = tool
      .path
      .as_ref()
      .map(|path| redact(&path.display().to_string(), secrets).into());
    tool.version = tool.version.as_ref().map(|value| redact(value, secrets));
    tool.problem = tool.problem.as_ref().map(|value| redact(value, secrets));
    tool.alternatives = tool
      .alternatives
      .iter()
      .map(|path| redact(&path.display().to_string(), secrets).into())
      .collect();
  }
  discovery.caches.root = redact_path(&discovery.caches.root, secrets);
  discovery.caches.runs = redact_path(&discovery.caches.runs, secrets);
  discovery.caches.builds = redact_path(&discovery.caches.builds, secrets);
  discovery.caches.baselines = redact_path(&discovery.caches.baselines, secrets);
  discovery.caches.tools = redact_path(&discovery.caches.tools, secrets);
  discovery.caches.resource_slots = redact_path(&discovery.caches.resource_slots, secrets);
}

fn redact_path(path: &Path, secrets: &[String]) -> std::path::PathBuf {
  redact(&path.display().to_string(), secrets).into()
}
