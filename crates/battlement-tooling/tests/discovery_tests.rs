use std::{
  collections::{BTreeMap, BTreeSet},
  env,
  io::{BufRead, BufReader},
  path::{Path, PathBuf},
  process::{Command, Stdio},
};

use anyhow::{Result, bail};
use battlement_tooling::{
  discovery::{DiscoveryRequest, HostDiscovery},
  doctor::{CheckCategory, CheckStatus, DoctorReport, DoctorRequest},
  host::{FilesystemOperation, Host, OperatingSystem},
  unity_lease::UnityEditorLease,
};
use tempfile::TempDir;

const UNITY_VERSION: &str = "6000.5.8f1";

#[test]
fn healthy_read_only_host_separates_optional_and_write_checks() {
  let mut host = FakeHost::macos();
  host
    .environment
    .insert("UNITY_EDITOR".to_owned(), "/tools/unity".to_owned());
  host
    .environment
    .insert("DITTO_ODIFF_PATH".to_owned(), "/tools/odiff".to_owned());
  host
    .files
    .extend([PathBuf::from("/tools/unity"), PathBuf::from("/tools/odiff")]);
  host.command("/tools/unity", &["-version"], UNITY_VERSION);
  host.command("/tools/odiff", &["--version"], "ODiff 4.5.0");

  let report = DoctorReport::inspect(
    &host,
    &DoctorRequest {
      discovery: request(),
      write_required: false,
      minimum_available_bytes: Some(1024),
      secret_environment_names: vec![],
    },
  )
  .unwrap();

  assert!(report.healthy());
  assert!(report.discovery.unity.ready());
  assert!(report.discovery.odiff.ready());
  assert!(report.checks.iter().any(|check| {
    check.category == CheckCategory::Optional
      && check.name == "FFmpeg"
      && check.status == CheckStatus::Warning
  }));
  assert_eq!(
    report
      .checks
      .iter()
      .filter(|check| check.category == CheckCategory::ReadOnly)
      .count(),
    4
  );
  assert!(
    report
      .checks
      .iter()
      .filter(|check| check.category == CheckCategory::Write)
      .all(|check| check.status != CheckStatus::Failed)
  );
}

#[test]
fn doctor_reports_mismatches_alternatives_permissions_and_redacts_secrets() {
  let mut host = FakeHost::macos();
  host
    .environment
    .insert("UNITY_EDITOR".to_owned(), "/secret/unity".to_owned());
  host
    .environment
    .insert("R2_SECRET".to_owned(), "secret".to_owned());
  host.files.insert(PathBuf::from("/secret/unity"));
  host.command("/secret/unity", &["-version"], "6000.4.0f1 secret");
  host.directories.insert(
    PathBuf::from("/Applications/Unity/Hub/Editor"),
    vec![PathBuf::from("/Applications/Unity/Hub/Editor/6000.4.0f1")],
  );
  host
    .executables
    .insert("odiff".to_owned(), PathBuf::from("/usr/bin/odiff"));
  host.failures.insert(
    (PathBuf::from("/cache/builds"), FilesystemOperation::Read),
    "permission denied with secret".to_owned(),
  );
  host.failures.insert(
    (PathBuf::from("/cache/runs"), FilesystemOperation::Write),
    "read-only secret".to_owned(),
  );

  let mut discovery = request();
  discovery.apple_tools_required = true;
  let report = DoctorReport::inspect(
    &host,
    &DoctorRequest {
      discovery,
      write_required: true,
      minimum_available_bytes: Some(20_000),
      secret_environment_names: vec!["R2_SECRET".to_owned()],
    },
  )
  .unwrap();

  assert!(!report.healthy());
  let unity = report
    .checks
    .iter()
    .find(|check| check.name == "Unity")
    .unwrap();
  assert_eq!(unity.status, CheckStatus::Failed);
  assert!(unity.detail.contains("expected version 6000.5.8f1"));
  assert!(unity.detail.contains("installed alternatives"));
  let odiff = report
    .checks
    .iter()
    .find(|check| check.name == "ODiff")
    .unwrap();
  assert!(odiff.detail.contains("/usr/bin/odiff"));
  assert!(report.checks.iter().any(|check| {
    check.category == CheckCategory::ReadOnly && check.status == CheckStatus::Failed
  }));
  assert!(report.checks.iter().any(|check| {
    check.category == CheckCategory::Write && check.status == CheckStatus::Failed
  }));
  assert!(
    report
      .checks
      .iter()
      .all(|check| !check.detail.contains("secret"))
  );
  assert_eq!(
    report.discovery.unity.path,
    Some(PathBuf::from("/<redacted>/unity"))
  );
}

#[test]
fn discovery_uses_host_native_cache_and_pinned_odiff_paths() {
  let host = FakeHost {
    operating_system: OperatingSystem::Linux,
    architecture: "arm64".to_owned(),
    home: PathBuf::from("/home/tester"),
    environment: BTreeMap::from([("XDG_CACHE_HOME".to_owned(), "/var/cache/tester".to_owned())]),
    ..FakeHost::default()
  };
  let mut request = request();
  request.cache_root = None;
  let discovery = HostDiscovery::inspect(&host, &request).unwrap();
  assert_eq!(
    discovery.caches.root,
    PathBuf::from("/var/cache/tester/Battlement/ditto")
  );
  assert_eq!(discovery.odiff.expected_version.as_deref(), Some("4.5.0"));
  assert!(
    discovery
      .odiff
      .problem
      .as_deref()
      .is_some_and(|problem| problem == "not found")
  );
}

#[test]
fn non_apple_silicon_macos_hosts_are_rejected() {
  let host = FakeHost {
    operating_system: OperatingSystem::Macos,
    architecture: "unsupported".to_owned(),
    ..FakeHost::default()
  };

  let error = HostDiscovery::inspect(&host, &request()).unwrap_err();

  assert!(error.to_string().contains("requires Apple silicon macOS"));
}

#[test]
fn rust_leases_use_both_legacy_slots_without_overbooking() {
  let temporary = TempDir::new().unwrap();
  let first = UnityEditorLease::try_acquire(temporary.path())
    .unwrap()
    .unwrap();
  let second = UnityEditorLease::try_acquire(temporary.path())
    .unwrap()
    .unwrap();
  assert_eq!((first.slot(), second.slot()), (0, 1));
  assert!(
    UnityEditorLease::try_acquire(temporary.path())
      .unwrap()
      .is_none()
  );
  drop(first);
  assert_eq!(
    UnityEditorLease::try_acquire(temporary.path())
      .unwrap()
      .unwrap()
      .slot(),
    0
  );
}

#[test]
fn rust_and_python_clients_exclude_each_other() {
  let temporary = TempDir::new().unwrap();
  let scripts = PathBuf::from(
    env::var_os("CARGO_MANIFEST_DIR").expect("Cargo must provide the active manifest directory"),
  )
  .join("../../scripts");
  let code = r#"
import os
from pathlib import Path
import time
from resource_slots import SlotLease

root = Path(os.environ["BATTLEMENT_RESOURCE_SLOTS"])
first = SlotLease(root, "unity-editor", 2).acquire()
second = SlotLease(root, "unity-editor", 2).acquire()
print("ready", flush=True)
time.sleep(30)
"#;
  let mut child =
    Command::new(env::var_os("BATTLEMENT_PYTHON").unwrap_or_else(|| "python3".into()))
      .args(["-u", "-c", code])
      .env("PYTHONPATH", scripts)
      .env("BATTLEMENT_RESOURCE_SLOTS", temporary.path())
      .stdout(Stdio::piped())
      .spawn()
      .unwrap();
  let mut ready = String::new();
  BufReader::new(child.stdout.take().unwrap())
    .read_line(&mut ready)
    .unwrap();
  assert_eq!(ready.trim(), "ready");
  assert!(
    UnityEditorLease::try_acquire(temporary.path())
      .unwrap()
      .is_none()
  );
  child.kill().unwrap();
  child.wait().unwrap();
  assert!(
    UnityEditorLease::try_acquire(temporary.path())
      .unwrap()
      .is_some()
  );
}

fn request() -> DiscoveryRequest {
  DiscoveryRequest {
    unity_version: UNITY_VERSION.to_owned(),
    apple_tools_required: false,
    ffmpeg_required: false,
    cache_root: Some(PathBuf::from("/cache")),
  }
}

struct FakeHost {
  operating_system: OperatingSystem,
  architecture: String,
  home: PathBuf,
  environment: BTreeMap<String, String>,
  executables: BTreeMap<String, PathBuf>,
  files: BTreeSet<PathBuf>,
  directories: BTreeMap<PathBuf, Vec<PathBuf>>,
  commands: BTreeMap<(PathBuf, Vec<String>), std::result::Result<String, String>>,
  failures: BTreeMap<(PathBuf, FilesystemOperation), String>,
  available_bytes: u64,
}

impl Default for FakeHost {
  fn default() -> Self {
    Self {
      operating_system: OperatingSystem::Unsupported,
      architecture: String::new(),
      home: PathBuf::new(),
      environment: BTreeMap::new(),
      executables: BTreeMap::new(),
      files: BTreeSet::new(),
      directories: BTreeMap::new(),
      commands: BTreeMap::new(),
      failures: BTreeMap::new(),
      available_bytes: 0,
    }
  }
}

impl FakeHost {
  fn macos() -> Self {
    Self {
      operating_system: OperatingSystem::Macos,
      architecture: "arm64".to_owned(),
      home: PathBuf::from("/Users/tester"),
      available_bytes: 10_000,
      ..Self::default()
    }
  }

  fn command(&mut self, executable: &str, arguments: &[&str], output: &str) {
    self.commands.insert(
      (
        PathBuf::from(executable),
        arguments
          .iter()
          .map(|argument| (*argument).to_owned())
          .collect(),
      ),
      Ok(output.to_owned()),
    );
  }
}

impl Host for FakeHost {
  fn operating_system(&self) -> OperatingSystem {
    self.operating_system
  }

  fn architecture(&self) -> String {
    self.architecture.clone()
  }

  fn environment(&self, name: &str) -> Option<String> {
    self.environment.get(name).cloned()
  }

  fn home_directory(&self) -> PathBuf {
    self.home.clone()
  }

  fn find_executable(&self, name: &str) -> Option<PathBuf> {
    self.executables.get(name).cloned()
  }

  fn is_file(&self, path: &Path) -> bool {
    self.files.contains(path)
  }

  fn child_directories(&self, path: &Path) -> Result<Vec<PathBuf>> {
    Ok(self.directories.get(path).cloned().unwrap_or_default())
  }

  fn command_output(&self, executable: &Path, arguments: &[&str]) -> Result<String> {
    match self.commands.get(&(
      executable.to_owned(),
      arguments
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect(),
    )) {
      Some(Ok(output)) => Ok(output.clone()),
      Some(Err(error)) => bail!(error.clone()),
      None => bail!("command response not configured"),
    }
  }

  fn check_directory(&self, path: &Path, operation: FilesystemOperation) -> Result<()> {
    if let Some(error) = self.failures.get(&(path.to_owned(), operation)) {
      bail!(error.clone());
    }
    Ok(())
  }

  fn available_bytes(&self, _path: &Path) -> Result<u64> {
    Ok(self.available_bytes)
  }
}
