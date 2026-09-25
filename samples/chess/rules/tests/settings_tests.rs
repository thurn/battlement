//! Saved preferences are exercised through the public application and storage boundaries.
mod support;

use battlement::host_settings::HostPlatform;
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
  game.display.activate_accessible("Taille du texte 100%");
  game.display.activate_accessible("150%");
  game.display.activate_accessible("Réduire les animations");
  game
    .display
    .activate_accessible("Allonger la durée des coups");
  game
    .display
    .activate_accessible("Envoyer les rapports de plantage");
  game.display.activate_accessible("RETOUR");
  game.display.activate_accessible("JOUER");
  game.restart();
  game.display.activate_accessible("Menu principal");
  game.display.activate_accessible("PARAMÈTRES");
  game.display.expect_button("Langue Français");
  game.display.expect_button("Taille du texte 150%");
  drop(game);
  let mut restored = ChessTest::persisted(storage.clone());
  restored.display.activate_accessible("PARAMÈTRES");
  restored.display.expect_button("Langue Français");
  restored.display.expect_button("Taille du texte 150%");
  assert_eq!(
    restored
      .display
      .semantic_node("Réduire les animations")
      .state
      .checked,
    Some(CheckedState::True)
  );
  assert_eq!(
    restored
      .display
      .semantic_node("Allonger la durée des coups")
      .state
      .checked,
    Some(CheckedState::False)
  );
  assert_eq!(
    restored
      .display
      .semantic_node("Envoyer les rapports de plantage")
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
  game.display.expect_button("Réessayer");
  game.display.activate_accessible("Taille du texte 100%");
  game.display.activate_accessible("200%");
  game.display.expect_button("Langue Français");
  assert_eq!(
    storage
      .load(Path::new("memory/chess-settings.json"))
      .unwrap(),
    durable
  );
  storage.recover_store();
  game.display.activate_accessible("Réessayer");
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
      .any(|node| node.label.as_deref() == Some("Réessayer"))
  );
}

#[test]
fn malformed_fields_and_conflicting_maps_default_independently() {
  let bytes = br#"{"language":"french","text_size":"200","reduce_motion":true,"master_volume":101,"music_volume":12,"effects_volume":"loud","display_mode":"invalid","max_framerate":-1,"keyboard":{"left":"KeyA","right":"KeyA","up":"ArrowUp","down":"ArrowDown","move_piece":"Space","pause":"Escape","restart":"KeyR"},"controller":{"left":"east"}}"#;
  let storage = MemoryPersistence::with_file("memory/chess-settings.json", bytes);
  let mut game = ChessTest::persisted(storage.clone());
  game.display.activate_accessible("PARAMÈTRES");
  game.display.expect_button("Langue Français");
  game.display.expect_button("Taille du texte 200%");
  game
    .display
    .activate_accessible("Envoyer les rapports de plantage");
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
  game.display.expect_button("JOUER");
  game.display.activate_accessible("PARAMÈTRES");
  game.display.expect_button("Langue Français");
  assert_eq!(
    storage
      .load(Path::new("memory/chess-settings.json"))
      .unwrap(),
    Some(bytes.to_vec())
  );
}

#[test]
fn reporting_restores_saved_intent_and_exposes_local_results_separately() {
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  assert_eq!(game.display.diagnostics().reporting(), Some(true));
  game.display.activate_accessible("SETTINGS");
  assert_eq!(
    game
      .display
      .semantic_node("Upload Crash Reports")
      .hint
      .as_deref(),
    Some("Reporting is not configured in this build.")
  );
  game.display.activate_accessible("Upload Crash Reports");
  assert_eq!(game.display.diagnostics().reporting(), Some(false));
  drop(game);
  let mut game = ChessTest::persisted(storage);
  assert_eq!(game.display.diagnostics().reporting(), Some(false));
  game.display.activate_accessible("SETTINGS");
  assert_eq!(
    game
      .display
      .semantic_node("Upload Crash Reports")
      .state
      .checked,
    Some(CheckedState::False)
  );
  game
    .display
    .set_host_settings(battlement::host_settings::HostSettings {
      platform: HostPlatform::Web,
      ..Default::default()
    });
  assert!(
    !game
      .display
      .accessibility()
      .nodes
      .iter()
      .any(|node| node.label.as_deref() == Some("Upload Crash Reports"))
  );
  assert_eq!(game.display.diagnostics().reporting(), Some(false));
}
