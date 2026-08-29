use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};

use crate::host::{Host, OperatingSystem};

const ODIFF_VERSION: &str = "4.5.0";

/// Host tools and filesystem roots needed by one Ditto invocation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveryRequest {
  pub unity_version: String,
  pub apple_tools_required: bool,
  pub ffmpeg_required: bool,
  pub cache_root: Option<PathBuf>,
}

/// One resolved or missing host tool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tool {
  pub name: String,
  pub path: Option<PathBuf>,
  pub version: Option<String>,
  pub expected_version: Option<String>,
  pub required: bool,
  pub pinned: bool,
  pub alternatives: Vec<PathBuf>,
  pub problem: Option<String>,
}

/// Conventional user-level roots shared by Ditto commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheRoots {
  pub root: PathBuf,
  pub runs: PathBuf,
  pub builds: PathBuf,
  pub baselines: PathBuf,
  pub tools: PathBuf,
  pub resource_slots: PathBuf,
}

/// Complete resolved host discovery.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostDiscovery {
  pub unity: Tool,
  pub apple: Vec<Tool>,
  pub odiff: Tool,
  pub ffmpeg: Tool,
  pub caches: CacheRoots,
}

impl HostDiscovery {
  /// Resolves supported tools and cache paths using an injectable host.
  pub fn inspect(host: &impl Host, request: &DiscoveryRequest) -> Result<Self> {
    ensure!(
      !request.unity_version.is_empty(),
      "Unity version must not be empty"
    );
    let caches = cache_roots(host, request.cache_root.as_deref());
    Ok(Self {
      unity: unity(host, &request.unity_version),
      apple: apple_tools(host, request.apple_tools_required),
      odiff: odiff(host, &caches),
      ffmpeg: named_executable(
        host,
        "FFmpeg",
        &["DITTO_FFMPEG_PATH", "BATTLEMENT_FFMPEG"],
        "ffmpeg",
        &["-version"],
        request.ffmpeg_required,
      ),
      caches,
    })
  }
}

impl Tool {
  /// Returns whether the tool exists and satisfies its required version.
  pub fn ready(&self) -> bool {
    self.path.is_some() && self.problem.is_none()
  }
}

fn cache_roots(host: &impl Host, configured: Option<&Path>) -> CacheRoots {
  let root = configured.map_or_else(
    || {
      host
        .environment("DITTO_CACHE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| default_cache_root(host))
    },
    Path::to_path_buf,
  );
  let resource_slots = host
    .environment("BATTLEMENT_RESOURCE_SLOTS")
    .map(PathBuf::from)
    .unwrap_or_else(|| user_cache_root(host).join("Battlement/resource-slots"));
  CacheRoots {
    runs: root.join("runs"),
    builds: root.join("builds"),
    baselines: root.join("baselines"),
    tools: root.join("tools"),
    root,
    resource_slots,
  }
}

fn default_cache_root(host: &impl Host) -> PathBuf {
  user_cache_root(host).join("Battlement/ditto")
}

fn user_cache_root(host: &impl Host) -> PathBuf {
  match host.operating_system() {
    OperatingSystem::Macos => host.home_directory().join("Library/Caches"),
    OperatingSystem::Linux => host
      .environment("XDG_CACHE_HOME")
      .map(PathBuf::from)
      .unwrap_or_else(|| host.home_directory().join(".cache")),
    OperatingSystem::Windows => host
      .environment("LOCALAPPDATA")
      .map(PathBuf::from)
      .unwrap_or_else(|| host.home_directory().join("AppData/Local")),
    OperatingSystem::Unsupported => host.home_directory().join(".cache"),
  }
}

fn unity(host: &impl Host, expected: &str) -> Tool {
  let (path, alternatives, explicit) = if let Some(configured) = host.environment("UNITY_EDITOR") {
    (
      PathBuf::from(configured),
      unity_alternatives(host, expected),
      true,
    )
  } else {
    (
      unity_path(host, expected),
      unity_alternatives(host, expected),
      false,
    )
  };
  if !host.is_file(&path) {
    return missing_tool("Unity", true, Some(expected), alternatives);
  }
  let version = if explicit {
    command_version(host, &path, &["-version"])
  } else {
    Ok(expected.to_owned())
  };
  versioned_tool(
    "Unity",
    path,
    version,
    Some(expected),
    true,
    true,
    alternatives,
  )
}

fn unity_path(host: &impl Host, version: &str) -> PathBuf {
  match host.operating_system() {
    OperatingSystem::Macos => PathBuf::from(format!(
      "/Applications/Unity/Hub/Editor/{version}/Unity.app/Contents/MacOS/Unity"
    )),
    OperatingSystem::Linux => host
      .home_directory()
      .join(format!("Unity/Hub/Editor/{version}/Editor/Unity")),
    OperatingSystem::Windows => PathBuf::from(
      host
        .environment("PROGRAMFILES")
        .unwrap_or_else(|| "C:/Program Files".to_owned()),
    )
    .join(format!("Unity/Hub/Editor/{version}/Editor/Unity.exe")),
    OperatingSystem::Unsupported => PathBuf::from("Unity"),
  }
}

fn unity_alternatives(host: &impl Host, expected: &str) -> Vec<PathBuf> {
  let root = match host.operating_system() {
    OperatingSystem::Macos => PathBuf::from("/Applications/Unity/Hub/Editor"),
    OperatingSystem::Linux => host.home_directory().join("Unity/Hub/Editor"),
    OperatingSystem::Windows => PathBuf::from(
      host
        .environment("PROGRAMFILES")
        .unwrap_or_else(|| "C:/Program Files".to_owned()),
    )
    .join("Unity/Hub/Editor"),
    OperatingSystem::Unsupported => return Vec::new(),
  };
  host
    .child_directories(&root)
    .unwrap_or_default()
    .into_iter()
    .filter(|path| path.file_name().is_none_or(|name| name != expected))
    .collect()
}

fn apple_tools(host: &impl Host, required: bool) -> Vec<Tool> {
  let mut tools = [
    ("xcrun", vec!["--version"]),
    ("xcodebuild", vec!["-version"]),
  ]
  .into_iter()
  .map(|(name, arguments)| named_executable(host, name, &[], name, &arguments, required))
  .collect::<Vec<_>>();
  let simctl = host.find_executable("xcrun");
  tools.push(match simctl {
    Some(path) if host.is_file(&path) => versioned_tool(
      "simctl",
      path.clone(),
      command_version(host, &path, &["simctl", "help"]),
      None,
      required,
      false,
      Vec::new(),
    ),
    _ => missing_tool("simctl", required, None, Vec::new()),
  });
  tools
}

fn odiff(host: &impl Host, caches: &CacheRoots) -> Tool {
  let override_path = host.environment("DITTO_ODIFF_PATH").map(PathBuf::from);
  let path = override_path.clone().unwrap_or_else(|| {
    caches.tools.join(format!(
      "odiff/{ODIFF_VERSION}/odiff-macos-{}",
      odiff_architecture(&host.architecture())
    ))
  });
  let alternatives = host.find_executable("odiff").into_iter().collect();
  if !host.is_file(&path) {
    return missing_tool("ODiff", true, Some(ODIFF_VERSION), alternatives);
  }
  versioned_tool(
    "ODiff",
    path.clone(),
    command_version(host, &path, &["--version"]),
    override_path.is_none().then_some(ODIFF_VERSION),
    true,
    override_path.is_none(),
    alternatives,
  )
}

fn named_executable(
  host: &impl Host,
  label: &str,
  environment_names: &[&str],
  executable_name: &str,
  version_arguments: &[&str],
  required: bool,
) -> Tool {
  let configured = environment_names
    .iter()
    .find_map(|name| host.environment(name))
    .map(PathBuf::from);
  let path = configured.or_else(|| host.find_executable(executable_name));
  let Some(path) = path else {
    return missing_tool(label, required, None, Vec::new());
  };
  if !host.is_file(&path) {
    return missing_tool(label, required, None, Vec::new());
  }
  versioned_tool(
    label,
    path.clone(),
    command_version(host, &path, version_arguments),
    None,
    required,
    false,
    Vec::new(),
  )
}

fn versioned_tool(
  name: &str,
  path: PathBuf,
  version: Result<String>,
  expected: Option<&str>,
  required: bool,
  pinned: bool,
  alternatives: Vec<PathBuf>,
) -> Tool {
  let (version, mut problem) = match version {
    Ok(version) => (Some(version), None),
    Err(error) => (None, Some(format!("version check failed: {error:#}"))),
  };
  if let (Some(version), Some(expected)) = (&version, expected) {
    if !version.contains(expected) {
      problem = Some(format!("expected version {expected}, found {version}"));
    }
  }
  Tool {
    name: name.to_owned(),
    path: Some(path),
    version,
    expected_version: expected.map(str::to_owned),
    required,
    pinned,
    alternatives,
    problem,
  }
}

fn missing_tool(
  name: &str,
  required: bool,
  expected: Option<&str>,
  alternatives: Vec<PathBuf>,
) -> Tool {
  Tool {
    name: name.to_owned(),
    path: None,
    version: None,
    expected_version: expected.map(str::to_owned),
    required,
    pinned: false,
    alternatives,
    problem: Some("not found".to_owned()),
  }
}

fn command_version(host: &impl Host, path: &Path, arguments: &[&str]) -> Result<String> {
  Ok(
    host
      .command_output(path, arguments)?
      .lines()
      .next()
      .unwrap_or_default()
      .to_owned(),
  )
}

fn odiff_architecture(architecture: &str) -> &str {
  match architecture {
    "aarch64" | "arm64" => "arm64",
    _ => "x64",
  }
}
