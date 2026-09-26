use std::{
  error::Error,
  fmt::{Display, Formatter, Result as FmtResult},
  fs::File,
  io::{Read, Seek},
  path::Path,
  process::{Child, Command, Output, Stdio},
  sync::atomic::{AtomicBool, Ordering},
  thread,
  time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use anyhow::Result;

use crate::build_cache_io;

/// Caller-owned cancellation for build admission and child processes.
#[derive(Clone, Copy, Debug, Default)]
pub struct BuildControl<'a> {
  interrupted: Option<&'a AtomicBool>,
}

/// A build stopped by its caller before further work could start.
#[derive(Debug)]
pub struct BuildInterrupted;

struct OwnedChild {
  child: Child,
  running: bool,
}

impl BuildControl<'_> {
  pub fn new(interrupted: &AtomicBool) -> BuildControl<'_> {
    BuildControl {
      interrupted: Some(interrupted),
    }
  }

  pub fn check(self) -> Result<()> {
    if self
      .interrupted
      .is_some_and(|flag| flag.load(Ordering::Acquire))
    {
      return Err(BuildInterrupted.into());
    }
    Ok(())
  }

  pub(crate) fn lock_exclusive(self, path: &Path) -> Result<File> {
    loop {
      self.check()?;
      if let Some(file) = build_cache_io::try_lock_exclusive(path)? {
        return Ok(file);
      }
      thread::sleep(Duration::from_millis(100));
    }
  }

  pub(crate) fn output(self, command: &mut Command) -> Result<Output> {
    self.check()?;
    let mut stdout = tempfile::tempfile()?;
    let mut stderr = tempfile::tempfile()?;
    command
      .stdin(Stdio::null())
      .stdout(stdout.try_clone()?)
      .stderr(stderr.try_clone()?);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = OwnedChild {
      child: command.spawn()?,
      running: true,
    };
    let status = loop {
      self.check()?;
      if let Some(status) = child.child.try_wait()? {
        child.running = false;
        break status;
      }
      thread::sleep(Duration::from_millis(50));
    };
    stdout.rewind()?;
    stderr.rewind()?;
    let mut output = Output {
      status,
      stdout: Vec::new(),
      stderr: Vec::new(),
    };
    stdout.read_to_end(&mut output.stdout)?;
    stderr.read_to_end(&mut output.stderr)?;
    Ok(output)
  }
}

impl Display for BuildInterrupted {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
    formatter.write_str("build interrupted")
  }
}

impl Error for BuildInterrupted {}

impl Drop for OwnedChild {
  fn drop(&mut self) {
    if !self.running {
      return;
    }
    #[cfg(unix)]
    {
      // The transaction wrapper owns Unity's separate group and restores its journal
      // when interrupted; give it time to finish that cleanup before escalation.
      let group = -(self.child.id() as i32);
      unsafe {
        libc::kill(group, libc::SIGINT);
      }
      let deadline = Instant::now() + Duration::from_secs(20);
      while self.child.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(50));
      }
      unsafe {
        libc::kill(group, libc::SIGTERM);
        libc::kill(group, libc::SIGKILL);
      }
    }
    #[cfg(windows)]
    let _ = Command::new("taskkill")
      .args(["/PID", &self.child.id().to_string(), "/T", "/F"])
      .output();
    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}
