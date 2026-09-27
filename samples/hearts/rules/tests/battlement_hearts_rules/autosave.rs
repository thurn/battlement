use std::{
  path::Path,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  time::{Duration, Instant},
};

use battlement::{Connect, ScreenSize, application::ApplicationState};
use battlement_hearts_rules::{self as hearts, HeartsReducer, domain::HeartsState};
use reactant::{
  PersistenceBackend, PersistenceCompletion, PersistenceOperation, PersistenceRequest,
};
use reactant_rules::ReducerGame;
use reactant_testing::{Display, MemoryPersistence};

use crate::shell;

type Game = ReducerGame<HeartsReducer>;
const MATCH: &str = "memory/hearts-match.json";

#[test]
fn saves_initial_deal_restores_without_teaching_and_keeps_settings_separate() {
  let storage = MemoryPersistence::empty();
  let mut display = self::mount(storage.clone());
  self::press(&mut display, "New game");
  self::press(&mut display, "Start new game");
  self::wait(&mut display, |_| {
    storage
      .load(Path::new(MATCH))
      .unwrap()
      .is_some_and(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).is_ok())
  });
  let initial = display.game_state::<Game>().unwrap();
  assert_eq!(self::saved(&storage), initial);
  self::press(&mut display, "Got it");
  self::press(&mut display, "Menu");
  self::press(&mut display, "Settings");
  self::press(&mut display, "Text size: standard");
  self::press(&mut display, "Back to game");
  self::press(&mut display, "Menu");
  self::press(&mut display, "Save and exit to menu");
  self::wait(&mut display, |display| self::has(display, "Continue"));
  let committed = self::saved(&storage);
  drop(display);
  let mut display = self::mount(storage);
  self::press(&mut display, "Continue");
  self::press(&mut display, "Menu");
  assert_eq!(display.game_state::<Game>().unwrap(), committed);
  assert!(!self::has(&display, "Got it"));
  self::press(&mut display, "Settings");
  assert!(self::has(&display, "Text size: larger"));
}

#[test]
fn corrupt_load_retry_does_not_replace_bytes_and_new_game_requires_confirmation() {
  let storage = MemoryPersistence::with_file(MATCH, b"broken");
  let mut display = self::mount(storage.clone());
  self::press(&mut display, "Retry saved game");
  assert_eq!(storage.load(Path::new(MATCH)).unwrap().unwrap(), b"broken");
  self::press(&mut display, "New game");
  self::press(&mut display, "Cancel");
  assert_eq!(storage.load(Path::new(MATCH)).unwrap().unwrap(), b"broken");
  self::press(&mut display, "New game");
  self::press(&mut display, "Start new game");
  self::wait(&mut display, |_| {
    storage
      .load(Path::new(MATCH))
      .unwrap()
      .is_some_and(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).is_ok())
  });
  self::saved(&storage).validate_saved().unwrap();
}

#[test]
fn failed_save_keeps_game_alive_and_exit_waits_for_retry() {
  let storage = MemoryPersistence::empty();
  let mut display = self::mount(storage.clone());
  storage.fail_store();
  self::press(&mut display, "New game");
  self::press(&mut display, "Start new game");
  self::press(&mut display, "Got it");
  self::press(&mut display, "Menu");
  self::press(&mut display, "Save and exit to menu");
  self::wait(&mut display, |display| self::has(display, "Retry save"));
  let accepted = display.game_state::<Game>().unwrap();
  assert!(storage.load(Path::new(MATCH)).unwrap().is_none());
  display.set_application_state(ApplicationState {
    focused: false,
    paused: true,
  });
  display.settle();
  assert_eq!(display.game_state::<Game>().unwrap(), accepted);
  storage.recover_store();
  display.set_application_state(ApplicationState {
    focused: true,
    paused: false,
  });
  self::press(&mut display, "Retry save");
  self::wait(&mut display, |display| self::has(display, "Continue"));
  assert_eq!(self::saved(&storage), accepted);
}

fn mount(storage: Arc<dyn PersistenceBackend>) -> Display {
  let mut display = Display::mount_with(
    move || hearts::application_with_storage(storage.clone()),
    shell::catalog(),
    Connect::new("test", "test", ScreenSize::new(1280, 720)).persistent_data_path("memory"),
  );
  self::wait(&mut display, |display| self::has(display, "New game"));
  display
}

fn has(display: &Display, label: &str) -> bool {
  display
    .accessibility()
    .nodes
    .iter()
    .any(|node| node.label.as_deref() == Some(label) && !node.state.disabled)
}

fn press(display: &mut Display, label: &str) {
  self::wait(display, |display| self::has(display, label));
  display.activate_accessible(label);
  display.flush();
  display.settle();
}

fn wait(display: &mut Display, ready: impl Fn(&Display) -> bool) {
  let deadline = Instant::now() + Duration::from_secs(10);
  loop {
    display.flush();
    display.settle();
    if ready(display) {
      return;
    }
    assert!(
      Instant::now() < deadline,
      "application did not reach requested state: {:?}",
      display
        .accessibility()
        .nodes
        .iter()
        .filter_map(|node| node.label.clone())
        .collect::<Vec<_>>()
    );
    std::thread::sleep(Duration::from_millis(1));
  }
}

fn saved(storage: &MemoryPersistence) -> HeartsState {
  let bytes = storage.load(Path::new(MATCH)).unwrap().unwrap();
  let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
  serde_json::from_value(json["state"].clone()).unwrap()
}

struct HeldStorage {
  memory: Arc<MemoryPersistence>,
  hold: AtomicBool,
  pending: Mutex<Option<(PersistenceRequest, PersistenceCompletion)>>,
}

impl HeldStorage {
  fn release(&self) {
    self.hold.store(false, Ordering::SeqCst);
    let (request, complete) = self.pending.lock().unwrap().take().unwrap();
    complete(request.id, request.execute(self.memory.as_ref()));
  }
}

impl PersistenceBackend for HeldStorage {
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    self.memory.load(path)
  }
  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    self.memory.store(path, bytes)
  }
  fn remove(&self, path: &Path) -> Result<(), String> {
    self.memory.remove(path)
  }
  fn start(self: Arc<Self>, request: PersistenceRequest, complete: PersistenceCompletion) {
    let game_write = request.path == Path::new(MATCH)
      && matches!(request.operation, PersistenceOperation::Store(_));
    if game_write && self.hold.load(Ordering::SeqCst) {
      let previous = self.pending.lock().unwrap().replace((request, complete));
      assert!(previous.is_none(), "only one physical write owns the slot");
    } else {
      complete(request.id, request.execute(self.memory.as_ref()));
    }
  }
}

#[test]
fn replacement_session_wins_after_old_write_and_background_keeps_save_alive() {
  let storage = Arc::new(HeldStorage {
    memory: MemoryPersistence::empty(),
    hold: AtomicBool::new(true),
    pending: Mutex::new(None),
  });
  let mut display = self::mount(storage.clone());
  self::press(&mut display, "New game");
  self::press(&mut display, "Start new game");
  let old = display.game_state::<Game>().unwrap();
  assert!(storage.pending.lock().unwrap().is_some());
  self::press(&mut display, "Got it");
  self::press(&mut display, "New game");
  self::press(&mut display, "Start new game");
  display.set_application_state(ApplicationState {
    focused: false,
    paused: true,
  });
  display.flush();
  display.settle();
  let current = display.game_state::<Game>().unwrap();
  assert_ne!(old.random().deck, current.random().deck);
  assert!(storage.memory.load(Path::new(MATCH)).unwrap().is_none());
  storage.release();
  self::wait(&mut display, |_| {
    storage.memory.load(Path::new(MATCH)).unwrap().is_some()
  });
  assert_eq!(self::saved(&storage.memory), current);
  display.set_application_state(ApplicationState {
    focused: true,
    paused: false,
  });
  self::press(&mut display, "Menu");
  self::press(&mut display, "Save and exit to menu");
  self::wait(&mut display, |display| self::has(display, "Continue"));
}
