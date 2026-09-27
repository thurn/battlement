use std::{
  path::{Path, PathBuf},
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
};

use battlement::{ObjectId, PickingMode, object_id};
use reactant::{
  Application, FilePersistenceBackend, PersistenceBackend, PersistenceCompletion,
  PersistenceOperation, PersistenceRequest, hooks, prelude::*,
};
use trox::ls;
use uuid::Uuid;

use crate::{
  app::{self, Opponents},
  startup::Startup,
};

const HELD: ObjectId = object_id!("6644ed66-12dc-4590-9af8-19d174a47080");

struct Storage {
  prefix: String,
  files: Mutex<Vec<PathBuf>>,
  corrupt: AtomicBool,
  fail: AtomicBool,
  hold: AtomicBool,
  pending: Mutex<Vec<(PersistenceRequest, PersistenceCompletion)>>,
  held: DisplayStore<bool>,
}

struct Fixture;

pub(crate) fn application() -> Application {
  app::configured_content(Fixture, false)
}

impl Storage {
  fn path(&self, path: &Path) -> PathBuf {
    let mapped = path.with_file_name(format!(
      "{}-{}",
      self.prefix,
      path.file_name().unwrap().to_string_lossy()
    ));
    let mut files = self.files.lock().unwrap();
    if !files.contains(&mapped) {
      files.push(mapped.clone());
    }
    mapped
  }

  fn release(self: &Arc<Self>) {
    self.hold.store(false, Ordering::SeqCst);
    let pending = std::mem::take(&mut *self.pending.lock().unwrap());
    self.held.set(false);
    for (request, complete) in pending {
      self.clone().start(request, complete);
    }
  }
}

impl PersistenceBackend for Storage {
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    let mapped = self.path(path);
    if path.file_name().unwrap() == "hearts-match.json"
      && self.corrupt.swap(false, Ordering::SeqCst)
    {
      FilePersistenceBackend.store(&mapped, b"unreadable fixture save")?;
    }
    FilePersistenceBackend.load(&mapped)
  }

  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    if self.fail.swap(false, Ordering::SeqCst) {
      return Err("injected save failure".into());
    }
    FilePersistenceBackend.store(&self.path(path), bytes)
  }

  fn remove(&self, path: &Path) -> Result<(), String> {
    FilePersistenceBackend.remove(&self.path(path))
  }

  fn start(self: Arc<Self>, request: PersistenceRequest, complete: PersistenceCompletion) {
    if matches!(request.operation, PersistenceOperation::Store(_))
      && self.hold.load(Ordering::SeqCst)
    {
      self.pending.lock().unwrap().push((request, complete));
      self.held.set(true);
      return;
    }
    std::thread::spawn(move || complete(request.id, request.execute(self.as_ref())));
  }
}

impl Drop for Storage {
  fn drop(&mut self) {
    for path in self.files.get_mut().unwrap() {
      let _ = std::fs::remove_file(path);
    }
  }
}

impl Component for Fixture {
  fn render(&self) -> impl Render {
    let storage = hooks::use_memo(
      || {
        Arc::new(Storage {
          prefix: format!("hearts-ditto-{}", Uuid::new_v4()),
          files: Mutex::new(Vec::new()),
          corrupt: AtomicBool::new(true),
          fail: AtomicBool::new(false),
          hold: AtomicBool::new(false),
          pending: Mutex::new(Vec::new()),
          held: DisplayStore::new(false),
        })
      },
      (),
    );
    let cleanup = storage.clone();
    hooks::use_effect(move || move || cleanup.release(), ());
    let (background, set_background) = hooks::use_state(false);
    let (generation, restart) = hooks::use_state(0_u64);
    let held = hooks::use_external_store(storage.held.clone());
    let controls = Controls {
      storage: storage.clone(),
      held,
      restart,
      background: set_background,
    };
    ContextProvider::new().context(controls).child(
      reactant::application::provider(battlement::application::ApplicationState {
        focused: !background,
        paused: background,
      })
      .child(
        Startup {
          backend: storage,
          opponents: Opponents::Scripted,
          new_match: app::fixture_match,
        }
        .key(generation),
      ),
    )
  }
}

#[derive(Clone)]
struct Controls {
  storage: Arc<Storage>,
  held: bool,
  restart: hooks::StateSetter<u64>,
  background: hooks::StateSetter<bool>,
}

impl PartialEq for Controls {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.storage, &other.storage) && self.held == other.held
  }
}

pub(crate) struct FixtureControls;

impl Component for FixtureControls {
  fn render(&self) -> impl Render {
    hooks::use_optional_context::<Controls>().map(|controls| {
      let fail = controls.storage.clone();
      let hold = controls.storage.clone();
      let release = controls.storage;
      let restart = controls.restart;
      let held = controls.held;
      let background_setter = controls.background.clone();
      let set_background = controls.background;
      View::new()
        .picking_mode(PickingMode::Ignore)
        .style(Style::new().width(100.pct()).height(100.pct()))
        .child(
          View::new()
            .style(
              Style::new()
                .position(Position::Absolute)
                .right(18.px())
                .top(90.px())
                .width(175.px()),
            )
            .child((
              Button::new(ls("Background app")).on_press(move || background_setter.set(true)),
              Button::new(ls("Foreground app")).on_press(move || set_background.set(false)),
              Button::new(ls("Reload saved app"))
                .on_press(move || restart.update(|value| value + 1)),
              Button::new(ls("Fail next save"))
                .on_press(move || fail.fail.store(true, Ordering::SeqCst)),
              Button::new(ls("Hold writes"))
                .on_press(move || hold.hold.store(true, Ordering::SeqCst)),
              Button::new(ls("Release writes")).on_press(move || release.release()),
              View::new()
                .id(HELD.into())
                .enabled(held)
                .child(Text::new(ls(if held {
                  "Write held"
                } else {
                  "Storage running"
                }))),
            )),
        )
    })
  }
}
