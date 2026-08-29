use std::{
  env, fs,
  path::{Path, PathBuf},
  process::{self, Command},
};

use anyhow::{Context, Result, bail};

/// Operating systems supported by Battlement host tooling.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperatingSystem {
  Macos,
  Linux,
  Windows,
  Unsupported,
}

/// Filesystem operations relevant to Ditto host checks.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FilesystemOperation {
  Read,
  Write,
}

/// Host behavior used by tool discovery and doctor checks.
pub trait Host {
  fn operating_system(&self) -> OperatingSystem;
  fn architecture(&self) -> String;
  fn environment(&self, name: &str) -> Option<String>;
  fn home_directory(&self) -> PathBuf;
  fn find_executable(&self, name: &str) -> Option<PathBuf>;
  fn is_file(&self, path: &Path) -> bool;
  fn child_directories(&self, path: &Path) -> Result<Vec<PathBuf>>;
  fn command_output(&self, executable: &Path, arguments: &[&str]) -> Result<String>;
  fn check_directory(&self, path: &Path, operation: FilesystemOperation) -> Result<()>;
  fn available_bytes(&self, path: &Path) -> Result<u64>;
}

/// The current process and local filesystem.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemHost;

impl Host for SystemHost {
  fn operating_system(&self) -> OperatingSystem {
    match env::consts::OS {
      "macos" => OperatingSystem::Macos,
      "linux" => OperatingSystem::Linux,
      "windows" => OperatingSystem::Windows,
      _ => OperatingSystem::Unsupported,
    }
  }

  fn architecture(&self) -> String {
    env::consts::ARCH.to_owned()
  }

  fn environment(&self, name: &str) -> Option<String> {
    env::var(name).ok()
  }

  fn home_directory(&self) -> PathBuf {
    env::var_os("HOME")
      .or_else(|| env::var_os("USERPROFILE"))
      .map_or_else(PathBuf::new, PathBuf::from)
  }

  fn find_executable(&self, name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.components().count() > 1 && candidate.is_file() {
      return candidate.canonicalize().ok();
    }
    env::var_os("PATH").and_then(|value| {
      env::split_paths(&value)
        .flat_map(|directory| executable_candidates(&directory, name))
        .find(|path| path.is_file())
        .and_then(|path| path.canonicalize().ok())
    })
  }

  fn is_file(&self, path: &Path) -> bool {
    path.is_file()
  }

  fn child_directories(&self, path: &Path) -> Result<Vec<PathBuf>> {
    if !path.is_dir() {
      return Ok(Vec::new());
    }
    let mut directories = fs::read_dir(path)?
      .filter_map(|entry| entry.ok())
      .filter_map(|entry| {
        entry
          .file_type()
          .ok()
          .filter(|kind| kind.is_dir())
          .map(|_| entry.path())
      })
      .collect::<Vec<_>>();
    directories.sort();
    Ok(directories)
  }

  fn command_output(&self, executable: &Path, arguments: &[&str]) -> Result<String> {
    let output = Command::new(executable)
      .args(arguments)
      .output()
      .with_context(|| format!("run {}", executable.display()))?;
    if !output.status.success() {
      bail!("{} exited with {}", executable.display(), output.status);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(if stdout.trim().is_empty() {
      stderr.trim().to_owned()
    } else {
      stdout.trim().to_owned()
    })
  }

  fn check_directory(&self, path: &Path, operation: FilesystemOperation) -> Result<()> {
    match operation {
      FilesystemOperation::Read => {
        let mut readable = path;
        while !readable.exists() {
          readable = readable
            .parent()
            .ok_or_else(|| anyhow::anyhow!("{} has no readable parent", path.display()))?;
        }
        fs::read_dir(readable).with_context(|| format!("read {}", readable.display()))?;
      }
      FilesystemOperation::Write => {
        fs::create_dir_all(path).with_context(|| format!("create {}", path.display()))?;
        let probe = path.join(format!(".ditto-write-probe-{}", process::id()));
        fs::write(&probe, b"probe").with_context(|| format!("write {}", path.display()))?;
        fs::remove_file(probe).with_context(|| format!("remove probe in {}", path.display()))?;
      }
    }
    Ok(())
  }

  fn available_bytes(&self, path: &Path) -> Result<u64> {
    fs2::available_space(path).with_context(|| format!("inspect capacity of {}", path.display()))
  }
}

fn executable_candidates(directory: &Path, name: &str) -> Vec<PathBuf> {
  let mut candidates = vec![directory.join(name)];
  if cfg!(windows) && Path::new(name).extension().is_none() {
    candidates.push(directory.join(format!("{name}.exe")));
  }
  candidates
}
