use std::{
  env,
  ffi::OsStr,
  fs,
  path::{Path, PathBuf},
  process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};

pub(crate) fn rules_package(manifest: &Path) -> Result<String> {
  let contents = fs::read_to_string(manifest)
    .with_context(|| format!("failed to read {}", manifest.display()))?;
  let name = contents
    .lines()
    .skip_while(|line| line.trim() != "[package]")
    .skip(1)
    .find_map(|line| line.trim().strip_prefix("name = \"")?.strip_suffix('"'))
    .context("rules manifest has no package name")?;
  Ok(name.to_owned())
}

pub(crate) fn host_architecture() -> Result<String> {
  #[cfg(windows)]
  {
    match env::consts::ARCH {
      "x86_64" => Ok("x86_64".to_owned()),
      architecture => bail!("unsupported Windows architecture: {architecture}"),
    }
  }
  #[cfg(not(windows))]
  {
    let output = Command::new("uname")
      .arg("-m")
      .output()
      .context("failed to determine the host architecture")?;
    if !output.status.success() {
      bail!("uname exited with status {}", output.status);
    }
    let architecture = String::from_utf8(output.stdout)?.trim().to_owned();
    match architecture.as_str() {
      "arm64" | "x86_64" => Ok(architecture),
      _ => bail!("unsupported macOS architecture: {architecture}"),
    }
  }
}

pub(crate) fn unity_editor(project: &Path) -> Result<PathBuf> {
  if let Some(configured) = env::var_os("UNITY_EDITOR") {
    return Ok(configured.into());
  }
  let version_path = project.join("ProjectSettings/ProjectVersion.txt");
  let version = fs::read_to_string(&version_path)
    .with_context(|| format!("failed to read {}", version_path.display()))?
    .lines()
    .find_map(|line| line.strip_prefix("m_EditorVersion: "))
    .context("ProjectVersion.txt has no editor version")?
    .to_owned();
  #[cfg(windows)]
  return Ok(format!(r"C:\Program Files\Unity\Hub\Editor\{version}\Editor\Unity.exe").into());
  #[cfg(not(windows))]
  Ok(format!("/Applications/Unity/Hub/Editor/{version}/Unity.app/Contents/MacOS/Unity").into())
}

pub(crate) fn architectures(path: &Path) -> Result<Vec<String>> {
  let output = output("lipo", [OsStr::new("-archs"), path.as_os_str()])?;
  let architectures: Vec<String> = output.split_whitespace().map(str::to_owned).collect();
  if architectures.is_empty() {
    bail!("lipo reported no architectures for {}", path.display());
  }
  Ok(architectures)
}

pub(crate) fn exported_symbols(path: &Path) -> Result<Vec<String>> {
  output("nm", [OsStr::new("-gjU"), path.as_os_str()]).map(|output| {
    output
      .lines()
      .map(str::trim)
      .filter(|line| !line.is_empty())
      .map(|line| line.strip_prefix('_').unwrap_or(line).to_owned())
      .collect()
  })
}

pub(crate) fn sign(path: &Path, identity: &str) -> Result<()> {
  status(
    "codesign",
    [
      OsStr::new("--force"),
      OsStr::new("--sign"),
      OsStr::new(identity),
      path.as_os_str(),
    ],
  )
}

pub(crate) fn signature_is_valid(path: &Path) -> bool {
  Command::new("codesign")
    .args(["--verify", "--deep", "--strict"])
    .arg(path)
    .stdout(Stdio::null())
    .stderr(Stdio::null())
    .status()
    .is_ok_and(|status| status.success())
}

fn output<I, S>(program: &str, args: I) -> Result<String>
where
  I: IntoIterator<Item = S>,
  S: AsRef<OsStr>,
{
  let output = Command::new(program)
    .args(args)
    .output()
    .with_context(|| format!("failed to run {program}"))?;
  if !output.status.success() {
    let diagnostic = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    bail!("{program} failed: {diagnostic}");
  }
  String::from_utf8(output.stdout).with_context(|| format!("{program} returned non-UTF-8 output"))
}

fn status<I, S>(program: &str, args: I) -> Result<()>
where
  I: IntoIterator<Item = S>,
  S: AsRef<OsStr>,
{
  let status = Command::new(program)
    .args(args)
    .status()
    .with_context(|| format!("failed to run {program}"))?;
  if !status.success() {
    bail!("{program} exited with status {status}");
  }
  Ok(())
}
