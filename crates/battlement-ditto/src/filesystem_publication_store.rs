//! Transactional publication state in a filesystem baseline store.

use std::{
  fs::{self, File, OpenOptions},
  path::{Component, Path, PathBuf},
  sync::Mutex,
};

use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use sha2::{Digest, Sha256};

use crate::{
  baseline_publication::{ConditionalMutation, ConditionalObjectStore, StoredObject},
  baseline_store::write_atomic,
};

const LEASE_SUFFIX: &str = "/metadata/write-lease.json";

/// A filesystem object view that holds an advisory lock during state mutations.
pub struct FilesystemPublicationStore {
  root: PathBuf,
  lease: Mutex<Option<FilesystemLease>>,
}

struct FilesystemLease {
  _file: File,
  object: StoredObject,
}

impl FilesystemPublicationStore {
  /// Uses a store root that may be inside or outside the repository.
  pub fn new(root: PathBuf) -> Self {
    Self {
      root,
      lease: Mutex::new(None),
    }
  }

  fn path(&self, key: &str) -> Result<PathBuf> {
    let path = Path::new(key);
    ensure!(
      path
        .components()
        .all(|part| matches!(part, Component::Normal(_))),
      "baseline object key is not a relative normalized path"
    );
    Ok(self.root.join(path))
  }

  fn acquire(&self, key: &str, bytes: &[u8]) -> Result<ConditionalMutation> {
    let mut lease = self.lease.lock().unwrap();
    if lease.is_some() {
      return Ok(ConditionalMutation::PreconditionFailed);
    }
    let path = self.path(key)?;
    let metadata = path
      .parent()
      .context("baseline lease has no metadata directory")?;
    fs::create_dir_all(metadata)?;
    let file = OpenOptions::new()
      .create(true)
      .read(true)
      .write(true)
      .truncate(false)
      .open(metadata.join("write.lock"))?;
    file.lock_exclusive()?;
    *lease = Some(FilesystemLease {
      _file: file,
      object: stored(bytes),
    });
    Ok(ConditionalMutation::Applied)
  }

  fn require_lease(&self) -> Result<()> {
    ensure!(
      self.lease.lock().unwrap().is_some(),
      "filesystem baseline mutation lock is not held"
    );
    Ok(())
  }

  fn write_absent(&self, key: &str, bytes: &[u8]) -> Result<ConditionalMutation> {
    let path = self.path(key)?;
    let parent = path.parent().context("baseline object has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
      fs::write(&temporary, bytes)?;
      OpenOptions::new()
        .write(true)
        .open(&temporary)?
        .sync_all()?;
      match fs::hard_link(&temporary, &path) {
        Ok(()) => {
          self::sync_directory(parent)?;
          Ok(ConditionalMutation::Applied)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
          Ok(ConditionalMutation::PreconditionFailed)
        }
        Err(error) => Err(error).context("publish filesystem baseline object"),
      }
    })();
    let _ = fs::remove_file(temporary);
    result
  }
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<()> {
  File::open(path)?.sync_all()?;
  Ok(())
}

#[cfg(windows)]
fn sync_directory(_path: &Path) -> Result<()> {
  Ok(())
}

impl ConditionalObjectStore for FilesystemPublicationStore {
  fn get(&self, key: &str) -> Result<Option<StoredObject>> {
    if key.ends_with(LEASE_SUFFIX) {
      return Ok(
        self
          .lease
          .lock()
          .unwrap()
          .as_ref()
          .map(|lease| lease.object.clone()),
      );
    }
    match fs::read(self.path(key)?) {
      Ok(bytes) => Ok(Some(stored(&bytes))),
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
      Err(error) => Err(error).context("read filesystem baseline object"),
    }
  }

  fn confirm(&self, key: &str, etag: &str) -> Result<ConditionalMutation> {
    Ok(match self.get(key)? {
      Some(object) if object.etag == etag => ConditionalMutation::Applied,
      _ => ConditionalMutation::PreconditionFailed,
    })
  }

  fn put_if_absent(&self, key: &str, bytes: &[u8]) -> Result<ConditionalMutation> {
    if key.ends_with(LEASE_SUFFIX) {
      return self.acquire(key, bytes);
    }
    if key.ends_with("/metadata/state.json") {
      self.require_lease()?;
      let path = self.path(key)?;
      if path.exists() {
        return Ok(ConditionalMutation::PreconditionFailed);
      }
      write_atomic(&path, bytes)?;
      return Ok(ConditionalMutation::Applied);
    }
    self.write_absent(key, bytes)
  }

  fn put_if_match(&self, key: &str, etag: &str, bytes: &[u8]) -> Result<ConditionalMutation> {
    if key.ends_with(LEASE_SUFFIX) {
      let mut lease = self.lease.lock().unwrap();
      let Some(current) = lease.as_mut() else {
        return Ok(ConditionalMutation::PreconditionFailed);
      };
      if current.object.etag != etag {
        return Ok(ConditionalMutation::PreconditionFailed);
      }
      current.object = stored(bytes);
      return Ok(ConditionalMutation::Applied);
    }
    self.require_lease()?;
    if self.confirm(key, etag)? == ConditionalMutation::PreconditionFailed {
      return Ok(ConditionalMutation::PreconditionFailed);
    }
    write_atomic(&self.path(key)?, bytes)?;
    Ok(ConditionalMutation::Applied)
  }

  fn delete_if_match(&self, key: &str, etag: &str) -> Result<ConditionalMutation> {
    ensure!(
      key.ends_with(LEASE_SUFFIX),
      "only the filesystem lease is conditional-delete capable"
    );
    let mut lease = self.lease.lock().unwrap();
    let matches = lease
      .as_ref()
      .is_some_and(|current| current.object.etag == etag);
    if !matches {
      return Ok(ConditionalMutation::PreconditionFailed);
    }
    *lease = None;
    Ok(ConditionalMutation::Applied)
  }

  fn delete(&self, key: &str) -> Result<()> {
    self.require_lease()?;
    match fs::remove_file(self.path(key)?) {
      Ok(()) => Ok(()),
      Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
      Err(error) => Err(error).context("delete filesystem baseline object"),
    }
  }
}

fn stored(bytes: &[u8]) -> StoredObject {
  StoredObject {
    bytes: bytes.to_vec(),
    etag: format!("{:x}", Sha256::digest(bytes)),
  }
}
