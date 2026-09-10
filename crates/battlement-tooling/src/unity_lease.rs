use std::{
  collections::BTreeSet,
  fs::{self, File, OpenOptions},
  path::{Path, PathBuf},
  process,
  sync::{
    LazyLock, Mutex,
    atomic::{AtomicU64, Ordering},
  },
  thread,
  time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use fs2::FileExt;

const UNITY_EDITOR_SLOTS: usize = 2;
const MACHINE_CAPACITY_SLOTS: usize = 6;
const BROWSER_CAPACITY_UNITS: usize = 1;
const BROWSER_SLOTS: usize = 2;
const COMPILER_CAPACITY_UNITS: usize = 3;
const NATIVE_PLAYER_CAPACITY_UNITS: usize = 2;
const NATIVE_PLAYER_SLOTS: usize = 3;
const UNITY_EDITOR_CAPACITY_UNITS: usize = 3;
const INHERITED_COMPILER_CAPACITY: &str = "BATTLEMENT_INHERITED_COMPILER_CAPACITY";
const INHERITED_COMPILER_CAPACITY_ROOT: &str = "BATTLEMENT_INHERITED_COMPILER_CAPACITY_ROOT";
const STALE_TICKET_GRACE: Duration = Duration::from_secs(5);
const WAIT_DIAGNOSTIC_INTERVAL: Duration = Duration::from_secs(30);
static TICKET_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static ACTIVE_TICKETS: LazyLock<Mutex<BTreeSet<PathBuf>>> =
  LazyLock::new(|| Mutex::new(BTreeSet::new()));

#[derive(Debug)]
struct SlotSet {
  files: Vec<(File, PathBuf, usize)>,
}

#[derive(Debug)]
struct AdmissionTicket {
  file: Option<File>,
  path: PathBuf,
  prefix: String,
}

/// Machine capacity held while one bounded Cargo writer is running.
#[derive(Debug)]
pub struct CompilerCapacityLease {
  _capacity: Option<SlotSet>,
}

/// Machine and browser capacity held for one browser session.
#[derive(Debug)]
pub struct BrowserCapacityLease {
  _capacity: SlotSet,
  _browser: SlotSet,
}

/// Machine and player capacity held for one native player session.
#[derive(Debug)]
pub struct NativePlayerCapacityLease {
  _capacity: SlotSet,
  _player: SlotSet,
}

/// One machine-wide Unity Editor capacity slot shared with legacy Python CI.
#[derive(Debug)]
pub struct UnityEditorLease {
  _capacity: SlotSet,
  editor: SlotSet,
}

impl CompilerCapacityLease {
  /// Tries to reserve one three-job Cargo writer without waiting.
  pub fn try_acquire(directory: &Path) -> Result<Option<Self>> {
    Ok(
      SlotSet::try_acquire(
        directory,
        "machine-heavy",
        MACHINE_CAPACITY_SLOTS,
        COMPILER_CAPACITY_UNITS,
      )?
      .map(|capacity| Self {
        _capacity: Some(capacity),
      }),
    )
  }

  /// Waits for the capacity assigned to one three-job Cargo writer.
  pub fn acquire(directory: &Path) -> Result<Self> {
    if inherited_compiler_capacity(directory) >= COMPILER_CAPACITY_UNITS {
      return Ok(Self { _capacity: None });
    }
    wait_for_capacity(directory, COMPILER_CAPACITY_UNITS, || {
      Self::try_acquire(directory)
    })
  }
}

impl BrowserCapacityLease {
  /// Tries to reserve one browser session without waiting.
  pub fn try_acquire(directory: &Path) -> Result<Option<Self>> {
    Ok(
      try_bounded_resource(directory, BROWSER_CAPACITY_UNITS, "browser", BROWSER_SLOTS)?.map(
        |(capacity, browser)| Self {
          _capacity: capacity,
          _browser: browser,
        },
      ),
    )
  }

  /// Waits until one bounded browser session can start.
  pub fn acquire(directory: &Path) -> Result<Self> {
    wait_for_capacity(directory, BROWSER_CAPACITY_UNITS, || {
      Self::try_acquire(directory)
    })
  }
}

impl NativePlayerCapacityLease {
  /// Tries to reserve one native player session without waiting.
  pub fn try_acquire(directory: &Path) -> Result<Option<Self>> {
    Ok(
      try_bounded_resource(
        directory,
        NATIVE_PLAYER_CAPACITY_UNITS,
        "native-player",
        NATIVE_PLAYER_SLOTS,
      )?
      .map(|(capacity, player)| Self {
        _capacity: capacity,
        _player: player,
      }),
    )
  }

  /// Waits until one bounded native player session can start.
  pub fn acquire(directory: &Path) -> Result<Self> {
    wait_for_capacity(directory, NATIVE_PLAYER_CAPACITY_UNITS, || {
      Self::try_acquire(directory)
    })
  }
}

impl UnityEditorLease {
  /// Tries both shared slots without waiting.
  pub fn try_acquire(directory: &Path) -> Result<Option<Self>> {
    let Some(capacity) = SlotSet::try_acquire(
      directory,
      "machine-heavy",
      MACHINE_CAPACITY_SLOTS,
      UNITY_EDITOR_CAPACITY_UNITS,
    )?
    else {
      return Ok(None);
    };
    Ok(
      SlotSet::try_acquire(directory, "unity-editor", UNITY_EDITOR_SLOTS, 1)?.map(|editor| Self {
        _capacity: capacity,
        editor,
      }),
    )
  }

  /// Waits until one shared slot can be acquired.
  pub fn acquire(directory: &Path) -> Result<Self> {
    wait_for_capacity(directory, UNITY_EDITOR_CAPACITY_UNITS, || {
      Self::try_acquire(directory)
    })
  }

  /// Returns the stable zero-based slot number.
  pub fn slot(&self) -> usize {
    self.editor.files[0].2
  }

  /// Returns the exact legacy-compatible lock path.
  pub fn path(&self) -> &Path {
    &self.editor.files[0].1
  }
}

impl SlotSet {
  fn held(directory: &Path, name: &str, count: usize) -> Result<usize> {
    let mut held = 0;
    for slot in 0..count {
      let path = directory.join(format!("{name}-{slot}.lock"));
      let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)?;
      if file.try_lock_exclusive().is_err() {
        held += 1;
      } else {
        FileExt::unlock(&file)?;
      }
    }
    Ok(held)
  }

  fn try_acquire(directory: &Path, name: &str, count: usize, units: usize) -> Result<Option<Self>> {
    fs::create_dir_all(directory)
      .with_context(|| format!("create resource slot directory {}", directory.display()))?;
    let mut files = Vec::with_capacity(units);
    for slot in 0..count {
      let path = directory.join(format!("{name}-{slot}.lock"));
      let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)?;
      if file.metadata()?.len() == 0 {
        file.set_len(1)?;
      }
      if file.try_lock_exclusive().is_ok() {
        files.push((file, path, slot));
        if files.len() == units {
          return Ok(Some(Self { files }));
        }
      }
    }
    Ok(None)
  }
}

impl Drop for SlotSet {
  fn drop(&mut self) {
    for (file, _, _) in &self.files {
      let _ = FileExt::unlock(file);
    }
  }
}

impl AdmissionTicket {
  fn join(directory: &Path, name: &str) -> Result<Self> {
    fs::create_dir_all(directory)
      .with_context(|| format!("create resource slot directory {}", directory.display()))?;
    let prefix = format!(".{name}.queue.");
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let path = directory.join(format!(
      "{prefix}{timestamp:020}.{:010}.{:010}.lock",
      process::id(),
      TICKET_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let file = OpenOptions::new()
      .create_new(true)
      .read(true)
      .write(true)
      .open(&path)?;
    file.set_len(1)?;
    file.lock_exclusive()?;
    ACTIVE_TICKETS
      .lock()
      .expect("active ticket registry is poisoned")
      .insert(path.clone());
    Ok(Self {
      file: Some(file),
      path,
      prefix,
    })
  }

  fn is_first(&self) -> Result<bool> {
    let mut tickets = fs::read_dir(self.path.parent().expect("ticket path has a parent"))?
      .filter_map(|entry| entry.ok())
      .filter(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        name.starts_with(&self.prefix) && name.ends_with(".lock")
      })
      .map(|entry| entry.path())
      .collect::<Vec<_>>();
    tickets.sort();
    for path in tickets {
      if path == self.path {
        return Ok(true);
      }
      if ACTIVE_TICKETS
        .lock()
        .expect("active ticket registry is poisoned")
        .contains(&path)
      {
        return Ok(false);
      }
      let Ok(file) = OpenOptions::new().read(true).write(true).open(&path) else {
        continue;
      };
      if file.try_lock_exclusive().is_ok() {
        let stale = path
          .metadata()
          .and_then(|metadata| metadata.modified())
          .ok()
          .and_then(|modified| modified.elapsed().ok())
          .is_some_and(|age| age >= STALE_TICKET_GRACE);
        let _ = FileExt::unlock(&file);
        drop(file);
        if stale {
          let _ = fs::remove_file(path);
          continue;
        }
      }
      return Ok(false);
    }
    anyhow::bail!("resource admission ticket disappeared while waiting")
  }

  fn position(&self) -> Result<(usize, usize)> {
    let mut tickets = fs::read_dir(self.path.parent().expect("ticket path has a parent"))?
      .filter_map(|entry| entry.ok())
      .map(|entry| entry.path())
      .filter(|path| {
        path
          .file_name()
          .is_some_and(|name| name.to_string_lossy().starts_with(&self.prefix))
      })
      .collect::<Vec<_>>();
    tickets.sort();
    Ok((
      tickets
        .iter()
        .position(|path| path == &self.path)
        .map_or(0, |position| position + 1),
      tickets.len(),
    ))
  }
}

impl Drop for AdmissionTicket {
  fn drop(&mut self) {
    ACTIVE_TICKETS
      .lock()
      .expect("active ticket registry is poisoned")
      .remove(&self.path);
    if let Some(file) = self.file.take() {
      let _ = FileExt::unlock(&file);
      drop(file);
    }
    let _ = fs::remove_file(&self.path);
  }
}

fn try_bounded_resource(
  directory: &Path,
  capacity_units: usize,
  name: &str,
  count: usize,
) -> Result<Option<(SlotSet, SlotSet)>> {
  let Some(capacity) = SlotSet::try_acquire(
    directory,
    "machine-heavy",
    MACHINE_CAPACITY_SLOTS,
    capacity_units,
  )?
  else {
    return Ok(None);
  };
  Ok(SlotSet::try_acquire(directory, name, count, 1)?.map(|resource| (capacity, resource)))
}

fn inherited_compiler_capacity(directory: &Path) -> usize {
  if std::env::var_os(INHERITED_COMPILER_CAPACITY_ROOT).as_deref() != Some(directory.as_os_str()) {
    return 0;
  }
  parse_inherited_compiler_capacity(std::env::var(INHERITED_COMPILER_CAPACITY).ok().as_deref())
}

fn wait_for_capacity<T>(
  directory: &Path,
  units: usize,
  mut acquire: impl FnMut() -> Result<Option<T>>,
) -> Result<T> {
  let ticket = AdmissionTicket::join(directory, "machine-heavy")?;
  let started = std::time::Instant::now();
  let mut next_diagnostic = Duration::from_secs(1);
  loop {
    if ticket.is_first()?
      && let Some(lease) = acquire()?
    {
      return Ok(lease);
    }
    if started.elapsed() >= next_diagnostic {
      let (position, depth) = ticket.position()?;
      let held = SlotSet::held(directory, "machine-heavy", MACHINE_CAPACITY_SLOTS)?;
      eprintln!(
        "Resource capacity: waiting for machine-heavy ({units} of {MACHINE_CAPACITY_SLOTS} \
         units; {held} held; queue {position}/{depth})"
      );
      next_diagnostic += WAIT_DIAGNOSTIC_INTERVAL;
    }
    thread::sleep(Duration::from_millis(100));
  }
}

fn parse_inherited_compiler_capacity(value: Option<&str>) -> usize {
  value.and_then(|value| value.parse().ok()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
  #[test]
  fn inherited_compiler_capacity_requires_a_valid_unit_count() {
    assert_eq!(
      crate::unity_lease::parse_inherited_compiler_capacity(Some("3")),
      3
    );
    assert_eq!(
      crate::unity_lease::parse_inherited_compiler_capacity(Some("invalid")),
      0
    );
    assert_eq!(
      crate::unity_lease::parse_inherited_compiler_capacity(None),
      0
    );
  }
}
