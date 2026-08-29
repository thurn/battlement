use std::{
  fs::{self, File, OpenOptions},
  io::Write,
  path::Path,
  time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use battlement_tooling::{
  build_cache::{BuildCache, CleanupScope, DEFAULT_BUILD_CACHE_BYTES},
  discovery::{CacheRoots, DiscoveryRequest, HostDiscovery},
  doctor::{CheckCategory, CheckStatus, DoctorCheck, DoctorReport, DoctorRequest},
  host::{FilesystemOperation, Host, SystemHost},
};
use fs2::FileExt;

use crate::{
  cli::{CleanCommand, DoctorOptions},
  config::{
    self,
    model::{Baseline, StepKind, Suite, Target},
  },
  selection, storage_commands,
  wire::run_storage::{RunCleanupScope, RunStore},
};

pub(crate) fn doctor(
  config_path: Option<&Path>,
  options: DoctorOptions,
  stdout: &mut dyn Write,
) -> Result<u8> {
  let suite = config::load(config_path)?;
  let selected = selection::resolve(
    &suite,
    &selection::Options {
      profile: options.profile,
      allow_empty: true,
      ..selection::Options::default()
    },
  )?;
  let secrets = credential_names(&suite);
  let mut report = DoctorReport::inspect(
    &SystemHost,
    &DoctorRequest {
      discovery: discovery_request(&suite, selected.profile.target())?,
      write_required: false,
      minimum_available_bytes: None,
      secret_environment_names: secrets,
    },
  )?;
  report
    .checks
    .extend(baseline_checks(&suite, &report.discovery.caches));
  for check in &report.checks {
    writeln!(
      stdout,
      "{} {} {}: {}",
      category(check.category),
      status(check.status),
      check.name,
      check.detail
    )?;
  }
  Ok(if report.healthy() { 0 } else { 2 })
}

pub(crate) fn clean(
  config_path: Option<&Path>,
  command: CleanCommand,
  stdout: &mut dyn Write,
) -> Result<u8> {
  if let CleanCommand::Storage { apply } = command {
    return storage_commands::clean_storage(config_path, apply, stdout);
  }
  let suite = config::load(config_path)?;
  let roots = cache_roots(&suite)?;
  match command {
    CleanCommand::Runs { global } => clean_runs(&suite, &roots, global, stdout),
    CleanCommand::Builds { global } => clean_builds(&suite, &roots, global, stdout),
    CleanCommand::Baselines => clean_baselines(&suite, &roots, stdout),
    CleanCommand::Storage { .. } => unreachable!(),
  }
}

pub(crate) fn cache_roots(suite: &Suite) -> Result<CacheRoots> {
  Ok(HostDiscovery::inspect(&SystemHost, &discovery_request(suite, Target::Macos)?)?.caches)
}

fn clean_runs(
  suite: &Suite,
  roots: &CacheRoots,
  global: bool,
  stdout: &mut dyn Write,
) -> Result<u8> {
  let mut store = RunStore::open(&roots.runs)?;
  let scope = if global {
    RunCleanupScope::Global
  } else {
    RunCleanupScope::Suite {
      repository: suite
        .repository
        .canonicalize()?
        .to_string_lossy()
        .into_owned(),
      suite: suite.name.clone(),
    }
  };
  let now = unix_time()?;
  let preview = store.cleanup_preview(&scope, now)?;
  print_plan(
    stdout,
    "runs",
    preview
      .inactive
      .iter()
      .map(|entry| (&entry.run_id, entry.artifact_bytes)),
    &preview.active,
  )?;
  stdout.flush()?;
  store.cleanup_planned(&preview, now)?;
  Ok(0)
}

fn clean_builds(
  suite: &Suite,
  roots: &CacheRoots,
  global: bool,
  stdout: &mut dyn Write,
) -> Result<u8> {
  let cache = BuildCache::open(&roots.builds, DEFAULT_BUILD_CACHE_BYTES)?;
  let scope = if global {
    CleanupScope::Global
  } else {
    CleanupScope::Suite {
      repository: suite
        .repository
        .canonicalize()?
        .to_string_lossy()
        .into_owned(),
      suite: suite.name.clone(),
    }
  };
  let preview = cache.cleanup_preview(&scope)?;
  print_plan(
    stdout,
    "builds",
    preview
      .inactive
      .iter()
      .map(|entry| (&entry.fingerprint, entry.bytes)),
    &preview.active,
  )?;
  stdout.flush()?;
  cache.cleanup_planned(&preview, unix_time()?)?;
  Ok(0)
}

fn clean_baselines(suite: &Suite, roots: &CacheRoots, stdout: &mut dyn Write) -> Result<u8> {
  let namespace = match suite
    .baseline
    .as_ref()
    .context("suite has no baseline store")?
  {
    Baseline::Filesystem { namespace, .. } | Baseline::R2 { namespace, .. } => namespace,
  };
  let root = roots.baselines.join(namespace);
  let mut planned = Vec::new();
  let mut active = Vec::new();
  let mut leases = Vec::new();
  collect_baselines(&root, &mut planned, &mut active, &mut leases)?;
  print_plan(
    stdout,
    "baselines",
    planned.iter().map(|(path, bytes, _)| (path, *bytes)),
    &active,
  )?;
  stdout.flush()?;
  for (path, _, lock) in planned {
    if Path::new(&path).is_file() {
      fs::remove_file(path)?;
    }
    if Path::new(&lock).is_file() {
      fs::remove_file(lock)?;
    }
  }
  drop(leases);
  remove_empty_directories(&root)?;
  Ok(0)
}

fn collect_baselines(
  directory: &Path,
  planned: &mut Vec<(String, u64, String)>,
  active: &mut Vec<String>,
  leases: &mut Vec<File>,
) -> Result<()> {
  if !directory.is_dir() {
    return Ok(());
  }
  for entry in fs::read_dir(directory)? {
    let entry = entry?;
    if entry.file_type()?.is_dir() {
      collect_baselines(&entry.path(), planned, active, leases)?;
      continue;
    }
    let path = entry.path();
    if path.extension().is_none_or(|extension| extension != "png") {
      continue;
    }
    let lock_path = path.with_extension("download.lock");
    let lease = OpenOptions::new()
      .create(true)
      .read(true)
      .write(true)
      .truncate(false)
      .open(&lock_path)?;
    if lease.try_lock_exclusive().is_ok() {
      planned.push((
        path.to_string_lossy().into_owned(),
        entry.metadata()?.len(),
        lock_path.to_string_lossy().into_owned(),
      ));
      leases.push(lease);
    } else {
      active.push(path.to_string_lossy().into_owned());
    }
  }
  Ok(())
}

fn remove_empty_directories(directory: &Path) -> Result<bool> {
  if !directory.is_dir() {
    return Ok(false);
  }
  for entry in fs::read_dir(directory)? {
    let entry = entry?;
    if entry.file_type()?.is_dir() {
      remove_empty_directories(&entry.path())?;
    }
  }
  if fs::read_dir(directory)?.next().is_none() {
    fs::remove_dir(directory)?;
    return Ok(true);
  }
  Ok(false)
}

fn print_plan<'a>(
  stdout: &mut dyn Write,
  label: &str,
  entries: impl Iterator<Item = (&'a String, u64)>,
  active: &[String],
) -> Result<()> {
  let entries = entries.collect::<Vec<_>>();
  let bytes = entries.iter().map(|(_, bytes)| bytes).sum::<u64>();
  writeln!(
    stdout,
    "clean {label}: {} entries, {bytes} bytes",
    entries.len()
  )?;
  for (name, _) in entries {
    writeln!(stdout, "  {name}")?;
  }
  for name in active {
    writeln!(stdout, "  active: {name}")?;
  }
  Ok(())
}

pub(crate) fn discovery_request(suite: &Suite, target: Target) -> Result<DiscoveryRequest> {
  Ok(DiscoveryRequest {
    unity_version: unity_version(&suite.player.unity_project)?,
    apple_tools_required: target != Target::Webgl,
    ffmpeg_required: suite.scenarios.iter().any(|scenario| {
      scenario
        .steps
        .iter()
        .any(|step| matches!(step.action, StepKind::Video(_)))
    }),
    cache_root: None,
  })
}

pub(crate) fn unity_version(project: &Path) -> Result<String> {
  let contents = fs::read_to_string(project.join("ProjectSettings/ProjectVersion.txt"))?;
  contents
    .lines()
    .find_map(|line| line.strip_prefix("m_EditorVersion: "))
    .map(str::to_owned)
    .context("Unity ProjectVersion.txt omits m_EditorVersion")
}

fn credential_names(suite: &Suite) -> Vec<String> {
  match &suite.baseline {
    Some(Baseline::R2 {
      account_id_env,
      bucket_env,
      access_key_id_env,
      secret_access_key_env,
      ..
    }) => vec![
      account_id_env.clone(),
      bucket_env.clone(),
      access_key_id_env.clone(),
      secret_access_key_env.clone(),
    ],
    _ => Vec::new(),
  }
}

fn baseline_checks(suite: &Suite, roots: &CacheRoots) -> Vec<DoctorCheck> {
  let Some(baseline) = &suite.baseline else {
    return vec![DoctorCheck {
      category: CheckCategory::Optional,
      name: "baseline store".to_owned(),
      status: CheckStatus::Warning,
      detail: "not configured".to_owned(),
    }];
  };
  let read = (|| -> Result<String> {
    let (manifest, _) = storage_commands::manifest(suite)?;
    if let Some(entry) = manifest.baselines.first() {
      storage_commands::read_store(suite)?.hydrate(
        &manifest.namespace,
        &entry.sha256,
        &roots.baselines,
      )?;
      Ok(format!("verified object {}", entry.sha256))
    } else {
      Ok("reachable; manifest contains no objects".to_owned())
    }
  })();
  let write = match baseline {
    Baseline::Filesystem { root, .. } => SystemHost
      .check_directory(root, FilesystemOperation::Write)
      .map(|()| root.display().to_string()),
    Baseline::R2 {
      account_id_env,
      bucket_env,
      access_key_id_env,
      secret_access_key_env,
      ..
    } => [
      account_id_env,
      bucket_env,
      access_key_id_env,
      secret_access_key_env,
    ]
    .into_iter()
    .find(|name| std::env::var_os(name).is_none())
    .map_or_else(
      || Ok("configured".to_owned()),
      |name| anyhow::bail!("environment variable {name} is not set"),
    ),
  };
  vec![
    doctor_check(CheckCategory::ReadOnly, "baseline read", read, true),
    doctor_check(CheckCategory::Write, "baseline write", write, false),
  ]
}

fn doctor_check(
  category: CheckCategory,
  name: &str,
  result: Result<String>,
  required: bool,
) -> DoctorCheck {
  match result {
    Ok(detail) => DoctorCheck {
      category,
      name: name.to_owned(),
      status: CheckStatus::Passed,
      detail,
    },
    Err(error) => DoctorCheck {
      category,
      name: name.to_owned(),
      status: if required {
        CheckStatus::Failed
      } else {
        CheckStatus::Warning
      },
      detail: format!("{error:#}"),
    },
  }
}

fn category(value: CheckCategory) -> &'static str {
  match value {
    CheckCategory::Required => "required",
    CheckCategory::Optional => "optional",
    CheckCategory::ReadOnly => "read",
    CheckCategory::Write => "write",
  }
}

fn status(value: CheckStatus) -> &'static str {
  match value {
    CheckStatus::Passed => "pass",
    CheckStatus::Warning => "warning",
    CheckStatus::Failed => "fail",
  }
}

fn unix_time() -> Result<u64> {
  Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
