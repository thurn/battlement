//! Only persistence scenarios load bytes. Ordinary setup constructs a Board in memory.
//! Faults belong to the injected storage service; input, rules, and rendering stay real.
mod support;
use crate::support::{fixtures, game::ChessTest, storage};
use chess_rules::{assets, audio};
use cozy_chess::{Color, Piece, Square};
use reactant_testing::MemoryPersistence;
use std::time::Duration;

#[test]
fn saved_moves_survive_a_fresh_engine() {
  // Dropping the original prevents shared application state from faking persistence.
  let storage = MemoryPersistence::empty();
  let mut game = ChessTest::persisted(storage.clone());
  game.start();
  game.play(Square::E2, Square::E4);
  drop(game);
  let restored = ChessTest::persisted(storage);
  restored.expect_empty(Square::E2);
  restored.expect_piece(Square::E4, Color::White, Piece::Pawn);
}

#[test]
fn malformed_save_recovers_to_the_title() {
  // Invalid syntax must leave a usable entry point rather than a broken board.
  let mut game = ChessTest::persisted(storage::with_save(b"not json"));
  game.start();
  game.expect_board(&fixtures::initial());
}

#[test]
fn invalid_saved_position_recovers_to_the_title() {
  // Valid JSON can still describe invalid chess; recovery is the same visible behavior.
  let mut game = ChessTest::persisted(storage::with_save(br#"{"position":"invalid"}"#));
  game.start();
  game.expect_board(&fixtures::initial());
}

#[test]
fn load_failure_leaves_the_title_usable() {
  // A storage error should not prevent starting a new game.
  let storage = storage::with_save(include_bytes!("fixtures/default.json"));
  storage.fail_load();
  let mut game = ChessTest::persisted(storage);
  game.start();
  game.expect_board(&fixtures::initial());
}

#[test]
fn save_failure_does_not_interrupt_play() {
  // Saving is a side effect; the rendered move still completes when it fails.
  let storage = MemoryPersistence::empty();
  storage.fail_store();
  let mut game = ChessTest::persisted(storage);
  game.start();
  game.play(Square::E2, Square::E4);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
}

#[test]
fn delete_failure_does_not_interrupt_new_game() {
  // Reset must replace the visible session even when old bytes cannot be removed.
  let storage = storage::with_save(include_bytes!("fixtures/default.json"));
  storage.fail_remove();
  let mut game = ChessTest::persisted(storage);
  game.new_game();
  game.expect_board(&fixtures::initial());
}

#[test]
fn diagnostics_are_absent_when_disabled() {
  // Diagnostic metadata is checked only as a feature, never as a gameplay oracle.
  let game = ChessTest::from_position(fixtures::initial());
  assert!(game.display.diagnostics().metadata().is_empty());
}

#[test]
fn diagnostics_describe_the_sample_when_enabled() {
  // The optional host module receives the public sample and opponent labels.
  let game = ChessTest::assemble(Some(fixtures::initial()), None, &["battlement.diagnostics"]);
  let metadata = game.display.diagnostics().metadata();
  assert_eq!(
    metadata.get("sample.name").map(String::as_str),
    Some("chess")
  );
  assert_eq!(
    metadata.get("chess.opponent").map(String::as_str),
    Some("computer")
  );
}

#[test]
fn music_crossfades_after_two_minutes() {
  // Virtual time visits scheduled deadlines; it never waits for elapsed wall time.
  let mut game = ChessTest::title();
  game.start();
  game.advance(Duration::from_secs(120));
  assert_eq!(
    game.display.sounds("music/"),
    vec![assets::music::CRITICAL, assets::music::SWITCH_WITH_ME]
  );
  game
    .display
    .expect_crossfade(assets::music::SWITCH_WITH_ME, Duration::from_secs(5));
}

#[test]
fn new_game_does_not_repeat_opening_effects() {
  // Occurrence counts distinguish the reset feedback from replayed opening effects.
  let mut game = ChessTest::title();
  game.start();
  game.new_game();
  game
    .display
    .expect_particles(assets::effects::PIECE_SPAWN, 32);
  game.display.expect_sound(audio::RESET_SOUND);
}

#[test]
fn restart_creates_a_playable_fresh_session() {
  // Playing again detects stale callbacks that a static initial-board check would miss.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.play(Square::E2, Square::E4);
  game.restart();
  game.expect_board(&fixtures::initial());
  game.play(Square::D2, Square::D4);
  game.expect_piece(Square::D4, Color::White, Piece::Pawn);
  game.expect_piece(Square::D7, Color::Black, Piece::Pawn);
}
