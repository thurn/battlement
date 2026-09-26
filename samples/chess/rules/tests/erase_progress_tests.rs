//! Erasure uses real input and rules with delayed/failing storage acknowledgements.
mod support;

use std::{
  collections::VecDeque,
  path::Path,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
  time::Duration,
};

use cozy_chess::{Color, Piece, Square};
use reactant::{
  PersistenceBackend, PersistenceCompletion, PersistenceOperation, PersistenceRequest,
};
use reactant_testing::MemoryPersistence;
use support::{fixtures, game::ChessTest};

const GAME: &str = "memory/chess-game.json";
const SETTINGS: &str = "memory/chess-settings.json";

struct DelayedProgress {
  memory: Arc<MemoryPersistence>,
  delay: AtomicBool,
  requests: Mutex<VecDeque<(PersistenceRequest, PersistenceCompletion)>>,
}

impl DelayedProgress {
  fn new() -> Arc<Self> {
    Arc::new(Self {
      memory: MemoryPersistence::with_file(SETTINGS, br#"{"reduce_motion":true,"master_volume":23,"keyboard":{"left":"KeyA","right":"ArrowRight","up":"ArrowUp","down":"ArrowDown","move_piece":"Space","pause":"Escape","restart":"KeyR"}}"#),
      delay: AtomicBool::new(false),
      requests: Mutex::new(VecDeque::new()),
    })
  }

  fn next(&self, fail: bool) -> (PersistenceRequest, PersistenceCompletion) {
    let (request, complete) = self
      .requests
      .lock()
      .unwrap()
      .pop_front()
      .expect("pending storage work");
    if fail {
      complete(request.id, Err("injected storage failure".to_owned()));
    } else {
      self.memory.clone().start(request.clone(), complete.clone());
    }
    (request, complete)
  }

  fn expect_remove(&self) {
    let requests = self.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].0.operation, PersistenceOperation::Remove);
  }
}

impl PersistenceBackend for DelayedProgress {
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
    if request.path != Path::new(GAME) || request.operation == PersistenceOperation::Load {
      self.memory.clone().start(request, complete);
    } else if self.delay.load(Ordering::SeqCst) {
      self.requests.lock().unwrap().push_back((request, complete));
    } else {
      self.memory.clone().start(request, complete);
    }
  }
}

fn open_confirmation(game: &mut ChessTest) {
  game.show_menu();
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Erase Saved Data");
  game.display.expect_button("Erase");
}

#[test]
fn erasure_orders_after_pending_save_and_cannot_revive_from_old_completion_or_computer_timer() {
  let storage = DelayedProgress::new();
  storage
    .store(Path::new("memory/other-game.json"), b"unrelated")
    .unwrap();
  let preferences = storage.load(Path::new(SETTINGS)).unwrap();
  let mut game = ChessTest::persisted(storage.clone());
  game.start();
  storage.delay.store(true, Ordering::SeqCst);
  game.play(Square::E2, Square::E4);
  game.permit_reply(Square::D7, Square::D5);
  self::open_confirmation(&mut game);
  game.display.activate_accessible("Erase");
  game.display.expect_button("Erasing…");
  assert_eq!(storage.requests.lock().unwrap().len(), 1);
  assert!(matches!(
    storage.requests.lock().unwrap()[0].0.operation,
    PersistenceOperation::Store(_)
  ));
  assert!(game.display.semantic_node("Cancel").state.disabled);
  assert!(game.display.semantic_node("Erasing…").state.disabled);
  game.pause();
  game.display.expect_button("Erasing…");
  game.restart();
  game.advance(Duration::from_secs(10));
  let (old_write, stale) = storage.next(false);
  game.display.settle();
  storage.expect_remove();
  game.display.expect_button("Erasing…");
  assert!(storage.load(Path::new(GAME)).unwrap().is_some());
  storage.next(false);
  stale(old_write.id, Ok(None));
  game.display.settle();
  game.advance(Duration::from_secs(10));
  game.display.expect_button("PLAY");
  assert!(storage.load(Path::new(GAME)).unwrap().is_none());
  assert_eq!(storage.load(Path::new(SETTINGS)).unwrap(), preferences);
  assert_eq!(
    storage.load(Path::new("memory/other-game.json")).unwrap(),
    Some(b"unrelated".to_vec())
  );
  assert!(storage.requests.lock().unwrap().is_empty());
  game.start();
  game.expect_board(&fixtures::initial());
  drop(game);
  storage.delay.store(false, Ordering::SeqCst);
  let mut restored = ChessTest::persisted(storage.clone());
  assert_eq!(storage.requests.lock().unwrap().len(), 1);
  storage.next(false);
  restored.display.settle();
  restored.start();
  restored.expect_board(&fixtures::initial());
}

#[test]
fn deletion_failure_cancel_restores_the_latest_recoverable_board() {
  let storage = DelayedProgress::new();
  let mut game = ChessTest::persisted(storage.clone());
  game.start();
  game.play(Square::E2, Square::E4);
  let saved = storage.load(Path::new(GAME)).unwrap();
  storage.delay.store(true, Ordering::SeqCst);
  self::open_confirmation(&mut game);
  game.display.activate_accessible("Erase");
  storage.expect_remove();
  storage.next(true);
  game.display.settle();
  game.display.expect_button("Retry");
  assert_eq!(storage.load(Path::new(GAME)).unwrap(), saved);
  storage.delay.store(false, Ordering::SeqCst);
  game.display.activate_accessible("Cancel");
  game.display.activate_accessible("RETURN");
  game.start();
  game.expect_empty(Square::E2);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
  game.reply(Square::D7, Square::D5);
  game.expect_piece(Square::D5, Color::Black, Piece::Pawn);
}

#[test]
fn deletion_failure_retries_without_announcing_success_early() {
  let storage = DelayedProgress::new();
  let mut game = ChessTest::persisted(storage.clone());
  game.start();
  game.play(Square::E2, Square::E4);
  storage.delay.store(true, Ordering::SeqCst);
  self::open_confirmation(&mut game);
  game.display.activate_accessible("Erase");
  storage.next(true);
  game.display.settle();
  game.display.expect_button("Retry");
  game.display.activate_accessible("Retry");
  storage.expect_remove();
  game.display.expect_button("Erasing…");
  assert!(storage.load(Path::new(GAME)).unwrap().is_some());
  storage.next(false);
  game.display.settle();
  game.display.expect_button("PLAY");
  assert!(storage.load(Path::new(GAME)).unwrap().is_none());
}

#[test]
fn canceling_confirmation_leaves_progress_and_the_pending_computer_turn_untouched() {
  let storage = DelayedProgress::new();
  let mut game = ChessTest::persisted(storage.clone());
  game.start();
  game.play(Square::E2, Square::E4);
  game.permit_reply(Square::D7, Square::D5);
  let saved = storage.load(Path::new(GAME)).unwrap();
  self::open_confirmation(&mut game);
  game.display.activate_accessible("Cancel");
  assert_eq!(storage.load(Path::new(GAME)).unwrap(), saved);
  game.display.activate_accessible("RETURN");
  game.start();
  game.advance(Duration::from_secs(2));
  game.display.settle();
  game.expect_piece(Square::D5, Color::Black, Piece::Pawn);
}

#[test]
fn absent_progress_is_success_and_does_not_remove_preferences() {
  let storage = DelayedProgress::new();
  let preferences = storage.load(Path::new(SETTINGS)).unwrap();
  let mut game = ChessTest::persisted(storage.clone());
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Erase Saved Data");
  game.display.activate_accessible("Erase");
  game.display.expect_button("PLAY");
  assert_eq!(storage.load(Path::new(SETTINGS)).unwrap(), preferences);
  assert!(storage.load(Path::new(GAME)).unwrap().is_none());
}
