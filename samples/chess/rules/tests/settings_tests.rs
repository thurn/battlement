//! Saved preferences are exercised through the public application and storage boundaries.
mod support;

use battlement::{CheckedState, PhysicalKey};
use chess_rules::settings::{self, ChessSettings, Language, TextSize};
use reactant::{
  PersistenceBackend, PersistenceCompletion, PersistenceOperation, PersistenceRequest,
};
use reactant_testing::MemoryPersistence;
use std::{cell::RefCell, path::Path, rc::Rc};
use support::game::ChessTest;

#[test]
fn settings_survive_game_navigation_restart_and_new_application() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Language English");
  game.display.activate_accessible("Français");
  game.display.activate_accessible("Text Size 100%");
  game.display.activate_accessible("150%");
  game.display.activate_accessible("Reduce Motion");
  game.display.activate_accessible("Increase Move Duration");
  game.display.activate_accessible("Upload Crash Reports");
  game.display.activate_accessible("RETURN");
  game.start();
  game.restart();
  game.show_menu();
  game.display.activate_accessible("SETTINGS");
  game.display.expect_button("Language Français");
  game.display.expect_button("Text Size 150%");
  drop(game);
  let mut restored = ChessTest::persisted(storage.clone());
  restored.display.activate_accessible("SETTINGS");
  restored.display.expect_button("Language Français");
  restored.display.expect_button("Text Size 150%");
  assert_eq!(
    restored
      .display
      .semantic_node("Reduce Motion")
      .state
      .checked,
    Some(CheckedState::True)
  );
  assert_eq!(
    restored
      .display
      .semantic_node("Increase Move Duration")
      .state
      .checked,
    Some(CheckedState::False)
  );
  assert_eq!(
    restored
      .display
      .semantic_node("Upload Crash Reports")
      .state
      .checked,
    Some(CheckedState::False)
  );
  assert!(
    storage
      .load(Path::new("memory/chess-game.json"))
      .unwrap()
      .is_some()
  );
}

#[test]
fn failed_save_keeps_applied_preferences_and_retry_saves_current_intent() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Reduce Motion");
  let durable = storage
    .load(Path::new("memory/chess-settings.json"))
    .unwrap();
  storage.fail_store();
  game.display.activate_accessible("Language English");
  game.display.activate_accessible("Français");
  game.display.expect_button("Retry");
  game.display.activate_accessible("Text Size 100%");
  game.display.activate_accessible("200%");
  game.display.expect_button("Language Français");
  assert_eq!(
    storage
      .load(Path::new("memory/chess-settings.json"))
      .unwrap(),
    durable
  );
  storage.recover_store();
  game.display.activate_accessible("Retry");
  let saved: ChessSettings = serde_json::from_slice(
    &storage
      .load(Path::new("memory/chess-settings.json"))
      .unwrap()
      .unwrap(),
  )
  .unwrap();
  assert_eq!(saved.language, Language::French);
  assert_eq!(saved.text_size, TextSize::Percent200);
  assert!(saved.reduce_motion);
  assert!(
    !game
      .display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Retry"))
  );
}

#[test]
fn malformed_fields_and_conflicting_maps_default_independently() {
  let bytes = br#"{"language":"french","text_size":"200","reduce_motion":true,"master_volume":101,"music_volume":12,"effects_volume":"loud","display_mode":"invalid","max_framerate":-1,"keyboard":{"left":"KeyA","right":"KeyA","up":"ArrowUp","down":"ArrowDown","move_piece":"Space","pause":"Escape","restart":"KeyR"},"controller":{"left":"east"}}"#;
  let storage = MemoryPersistence::with_file("memory/chess-settings.json", bytes);
  let mut game = ChessTest::persisted(storage.clone());
  game.display.activate_accessible("SETTINGS");
  game.display.expect_button("Language Français");
  game.display.expect_button("Text Size 200%");
  game.display.activate_accessible("Upload Crash Reports");
  let saved: ChessSettings = serde_json::from_slice(
    &storage
      .load(Path::new("memory/chess-settings.json"))
      .unwrap()
      .unwrap(),
  )
  .unwrap();
  assert_eq!(saved.master_volume, 80);
  assert_eq!(saved.music_volume, 12);
  assert_eq!(saved.effects_volume, 75);
  assert_eq!(saved.keyboard.left, PhysicalKey::ArrowLeft);
  assert_eq!(saved.keyboard, ChessSettings::default().keyboard);
  assert_eq!(saved.controller, ChessSettings::default().controller);
}

struct DelayedSettings {
  memory: Rc<MemoryPersistence>,
  read: RefCell<Option<(PersistenceRequest, PersistenceCompletion)>>,
}

impl PersistenceBackend for DelayedSettings {
  fn load(&self, path: &Path) -> Result<Option<Vec<u8>>, String> {
    self.memory.load(path)
  }

  fn store(&self, path: &Path, bytes: &[u8]) -> Result<(), String> {
    self.memory.store(path, bytes)
  }

  fn remove(&self, path: &Path) -> Result<(), String> {
    self.memory.remove(path)
  }

  fn start(&self, request: PersistenceRequest, complete: PersistenceCompletion) {
    if request.operation == PersistenceOperation::Load
      && request
        .path
        .file_name()
        .is_some_and(|name| name == settings::FILE_NAME)
    {
      self.read.replace(Some((request, complete)));
    } else {
      self.memory.start(request, complete);
    }
  }
}

#[test]
fn preference_hydration_precedes_startup_effects_and_never_writes_defaults() {
  let bytes = br#"{"language":"french","master_volume":0}"#;
  let storage = Rc::new(DelayedSettings {
    memory: MemoryPersistence::with_file("memory/chess-settings.json", bytes),
    read: RefCell::new(None),
  });
  let mut game = ChessTest::persisted(storage.clone());
  assert!(game.display.audio_occurrences().is_empty());
  assert!(game.display.particle_occurrences().is_empty());
  assert!(
    !game
      .display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("PLAY"))
  );
  assert_eq!(
    storage
      .load(Path::new("memory/chess-settings.json"))
      .unwrap(),
    Some(bytes.to_vec())
  );
  let (request, complete) = storage.read.borrow_mut().take().unwrap();
  complete(request.id, storage.load(&request.path));
  game.display.settle();
  game.display.expect_button("PLAY");
  game.display.activate_accessible("SETTINGS");
  game.display.expect_button("Language Français");
  assert_eq!(
    storage
      .load(Path::new("memory/chess-settings.json"))
      .unwrap(),
    Some(bytes.to_vec())
  );
}
