//! Atomic native filesystem persistence.

use std::path::Path;
#[cfg(not(target_os = "emscripten"))]
use std::{
  fs::{self, File},
  io::{self, ErrorKind, Write},
};
#[cfg(not(target_os = "emscripten"))]
use tempfile::NamedTempFile;

use crate::persistence_operation::PersistenceBackend;
#[cfg(target_os = "emscripten")]
use crate::persistence_operation::{
  PersistenceCompletion, PersistenceOperation, PersistenceRequest,
};
#[cfg(target_os = "emscripten")]
use battlement_native::BrowserPersistenceOperation;

/// Host filesystem persistence: atomic native replacement or acknowledged browser sync.
#[derive(Default)]
pub struct FilePersistenceBackend;

impl PersistenceBackend for FilePersistenceBackend {
  #[cfg(not(target_os = "emscripten"))]
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    match fs::read(path) {
      Ok(bytes) => Ok(Some(bytes)),
      Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
      Err(error) => Err(error.to_string()),
    }
  }

  #[cfg(not(target_os = "emscripten"))]
  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    self::replace(path, Some(bytes), &File::sync_all).map_err(|error| error.to_string())
  }

  #[cfg(not(target_os = "emscripten"))]
  fn remove(&self, path: &Path) -> Result<(), String> {
    self::replace(path, None, &File::sync_all).map_err(|error| error.to_string())
  }

  #[cfg(target_os = "emscripten")]
  fn load(&self, _: &Path) -> Result<Option<Vec<u8>>, String> {
    Err("browser storage requires asynchronous acknowledgment".to_owned())
  }

  #[cfg(target_os = "emscripten")]
  fn store(&self, _: &Path, _: &[u8]) -> Result<(), String> {
    Err("browser storage requires asynchronous acknowledgment".to_owned())
  }

  #[cfg(target_os = "emscripten")]
  fn remove(&self, _: &Path) -> Result<(), String> {
    Err("browser storage requires asynchronous acknowledgment".to_owned())
  }

  #[cfg(target_os = "emscripten")]
  fn start(&self, request: PersistenceRequest, complete: PersistenceCompletion) {
    let operation = match &request.operation {
      PersistenceOperation::Load => BrowserPersistenceOperation::Load,
      PersistenceOperation::Store(bytes) => BrowserPersistenceOperation::Store(bytes),
      PersistenceOperation::Remove => BrowserPersistenceOperation::Remove,
    };
    battlement_native::start_browser_persistence(&request.path, operation, move |result| {
      complete(request.id, result);
    });
  }
}

#[cfg(not(target_os = "emscripten"))]
fn parent(path: &Path) -> &Path {
  path
    .parent()
    .filter(|path| !path.as_os_str().is_empty())
    .unwrap_or(Path::new("."))
}

#[cfg(not(target_os = "emscripten"))]
fn replace(
  path: &Path,
  bytes: Option<&[u8]>,
  sync: &impl Fn(&File) -> io::Result<()>,
) -> io::Result<()> {
  if bytes.is_none() && !path.try_exists()? {
    return Ok(());
  }
  let parent = self::parent(path);
  fs::create_dir_all(parent)?;
  let previous = self::backup(path, parent)?;
  let installed = if let Some(bytes) = bytes {
    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    Some(temporary.persist(path).map_err(|error| error.error)?)
  } else {
    fs::remove_file(path)?;
    None
  };
  let result = installed
    .as_ref()
    .map_or(Ok(()), sync)
    .and_then(|()| self::sync_directory(parent, sync));
  if let Err(error) = result {
    drop(installed);
    let restored = match previous {
      Some(previous) => match previous.persist(path) {
        Ok(_) => Ok(()),
        Err(error) => {
          let failure = error.error;
          let (_, retained) = error.file.keep()?;
          Err(io::Error::other(format!(
            "{failure}; prior bytes retained at {}",
            retained.display()
          )))
        }
      },
      None => match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
      },
    };
    restored.map_err(|restore| {
      io::Error::other(format!(
        "{error}; prior value restoration failed: {restore}"
      ))
    })?;
    self::sync_directory(parent, &File::sync_all)?;
    return Err(error);
  }
  Ok(())
}

#[cfg(not(target_os = "emscripten"))]
fn backup(path: &Path, parent: &Path) -> io::Result<Option<NamedTempFile>> {
  let mut previous = match File::open(path) {
    Ok(file) => file,
    Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
    Err(error) => return Err(error),
  };
  let mut backup = NamedTempFile::new_in(parent)?;
  io::copy(&mut previous, &mut backup)?;
  backup.as_file().sync_all()?;
  Ok(Some(backup))
}

#[cfg(not(target_os = "emscripten"))]
fn sync_directory(path: &Path, sync: &impl Fn(&File) -> io::Result<()>) -> io::Result<()> {
  #[cfg(unix)]
  sync(&File::open(path)?)?;
  #[cfg(not(unix))]
  let _ = (path, sync);
  Ok(())
}

#[cfg(all(test, not(target_os = "emscripten")))]
#[path = "persistence_file_tests.rs"]
mod tests;
