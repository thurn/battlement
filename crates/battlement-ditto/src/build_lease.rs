use anyhow::{Result, ensure};

#[cfg(unix)]
use std::{
  fs::File,
  io::{ErrorKind, Read},
  os::fd::{FromRawFd, RawFd},
};

pub(crate) fn until_eof(file_descriptor: i32) -> Result<()> {
  ensure!(
    file_descriptor >= 0,
    "build lease file descriptor is invalid"
  );
  self::platform_until_eof(file_descriptor)
}

#[cfg(unix)]
fn platform_until_eof(file_descriptor: RawFd) -> Result<()> {
  let mut control = unsafe { File::from_raw_fd(file_descriptor) };
  let mut byte = [0_u8; 1];
  loop {
    match control.read(&mut byte) {
      Ok(0) => return Ok(()),
      Ok(_) => {}
      Err(error) if error.kind() == ErrorKind::Interrupted => {}
      Err(error) => return Err(error.into()),
    }
  }
}

#[cfg(not(unix))]
fn platform_until_eof(_file_descriptor: i32) -> Result<()> {
  anyhow::bail!("retained build leases require inherited Unix file descriptors")
}

#[cfg(all(test, unix))]
mod tests {
  use std::{
    os::fd::IntoRawFd,
    os::unix::net::UnixStream,
    sync::{
      Arc,
      atomic::{AtomicBool, Ordering},
      mpsc,
    },
    thread,
  };

  #[test]
  fn eof_is_the_only_release_boundary() {
    let (reader, writer) = UnixStream::pair().unwrap();
    let released = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&released);
    let (started, waiting) = mpsc::channel();
    let holder = thread::spawn(move || {
      started.send(()).unwrap();
      crate::build_lease::until_eof(reader.into_raw_fd()).unwrap();
      observed.store(true, Ordering::SeqCst);
    });

    waiting.recv().unwrap();
    assert!(!released.load(Ordering::SeqCst));
    drop(writer);
    holder.join().unwrap();
    assert!(released.load(Ordering::SeqCst));
  }
}
