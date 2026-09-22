//! Service boundaries deserve their own scenarios. Only persistence tests load
//! bytes; timer tests use an explicit virtual duration. None waits for wall time.
mod support;
use crate::support::{catalog, fixtures, game::ChessTest, host, storage::MemoryPersistence};
use battlement::CommandBody;
use chess_rules::audio;
use cozy_chess::{Color, Piece, Square};
use std::time::Duration;

#[test]
fn a_persisted_move_survives_a_fresh_engine() {
  // Unlike gameplay fixtures, this deliberately crosses serialization twice.
  // Dropping the first engine prevents shared state from masquerading as storage.
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
fn corrupt_storage_falls_back_to_an_interactive_title() {
  // Syntax errors and invalid chess positions are independent persistence failures.
  // Recovery is observed by starting a normal board, not inspecting error flags.
  for bytes in [b"not json".as_slice(), br#"{"position":"invalid"}"#] {
    let mut game = ChessTest::persisted(MemoryPersistence::with_save(bytes));
    game.start();
    game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  }
}

#[test]
fn storage_failures_do_not_break_play_or_reset() {
  // Fault injection is restricted to the external storage service. Rendering and
  // input stay real, so these tests establish graceful behavior at that boundary.
  let failed_load = MemoryPersistence::with_save(include_bytes!("fixtures/default.json"));
  failed_load.fail_load();
  let mut game = ChessTest::persisted(failed_load);
  game.start();
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  let failed_store = MemoryPersistence::empty();
  failed_store.fail_store();
  let mut game = ChessTest::persisted(failed_store);
  game.start();
  game.play(Square::E2, Square::E4);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
  let failed_remove = MemoryPersistence::with_save(include_bytes!("fixtures/default.json"));
  failed_remove.fail_remove();
  let mut game = ChessTest::persisted(failed_remove);
  game.new_game();
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
}

#[test]
fn diagnostics_are_optional_host_output() {
  // Metadata is tested only as a diagnostics feature. Gameplay never uses it to
  // stand in for a rendered board or terminal sound.
  let absent = ChessTest::from_position(fixtures::initial());
  assert!(absent.display.diagnostics().metadata().is_empty());
  let enabled = ChessTest::assemble(Some(fixtures::initial()), None, &["battlement.diagnostics"]);
  let metadata = enabled.display.diagnostics().metadata();
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
fn virtual_time_crossfades_music_and_reset_does_not_repeat_opening() {
  // Advancing exactly two minutes fires known timers; it does not repeatedly ask
  // whether the next track has appeared. Presentation then executes the crossfade.
  let mut game = ChessTest::title();
  game.start();
  let spawn_count = game.display.particle_occurrences().len();
  game.advance(Duration::from_secs(120));
  let music = game
    .display
    .audio_occurrences()
    .iter()
    .filter(|o| catalog::MUSIC.contains(&o.address))
    .collect::<Vec<_>>();
  assert_eq!(music.len(), 2);
  assert_eq!(music[1].address, catalog::MUSIC[1]);
  assert!(game.display.commands().iter().any(|entry| matches!(&entry.command.body,CommandBody::AudioPlay(p) if p.address==catalog::MUSIC[1] && p.fade_in_ms==5000)));
  assert!(
    game
      .display
      .commands()
      .iter()
      .any(|entry| matches!(&entry.command.body,CommandBody::AudioStop(p) if p.fade_out_ms==5000))
  );
  game.new_game();
  assert_eq!(game.display.particle_occurrences().len(), spawn_count);
  host::sound(&game.display, audio::RESET_SOUND);
}

#[test]
fn restarting_replaces_the_session_and_leaves_the_opponent_held() {
  // Session replacement must discard old publications and callbacks. The fresh
  // board is asserted after reset, then used again to catch a stale session owner.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.play(Square::E2, Square::E4);
  game.restart();
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  game.expect_empty(Square::E4);
  game.play(Square::D2, Square::D4);
  game.expect_piece(Square::D4, Color::White, Piece::Pawn);
  game.expect_piece(Square::D7, Color::Black, Piece::Pawn);
}
