use std::{ffi::OsStr, process::Command};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(unix)]
use std::{io, os::unix::process::CommandExt};

/// Creates a compiler or batch-editor command whose descendants inherit build priority.
pub fn command(program: impl AsRef<OsStr>) -> Command {
  let mut command = Command::new(program);
  #[cfg(unix)]
  // SAFETY: the child hook only calls async-signal-safe process-priority functions.
  unsafe {
    command.pre_exec(|| {
      let current = libc::getpriority(libc::PRIO_PROCESS, 0);
      if current < 10 && libc::setpriority(libc::PRIO_PROCESS, 0, 10) != 0 {
        return Err(io::Error::last_os_error());
      }
      Ok(())
    });
  }
  #[cfg(windows)]
  command.creation_flags(0x00004000); // BELOW_NORMAL_PRIORITY_CLASS
  command
}
