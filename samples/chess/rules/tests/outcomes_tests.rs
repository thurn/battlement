//! Most sample tests should look like these: arrange a logical position, express
//! player intent, and inspect what Unity would display. The helpers own gestures,
//! projection, animation completion, and prefab queries. They never peek at the
//! rules state. A broken renderer therefore cannot pass by reporting a good board.
mod support;
use crate::support::{fixtures, game::ChessTest, host};
use chess_rules::{assets, audio};
use cozy_chess::{Color, Piece, Square};

#[test]
fn a_player_move_is_observable_before_an_explicit_computer_reply() {
  // Holding the opponent makes the observation boundary deliberate. Supplying a
  // legal reply avoids search cost and timing dependence without bypassing the
  // normal computer action, publication, Motion, or final state acceptance.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.play(Square::E2, Square::E4);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
  game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
  game.reply(Square::E7, Square::E5);
  game.expect_piece(Square::E5, Color::Black, Piece::Pawn);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
}

#[test]
fn a_capture_removes_the_victim_and_presents_its_effects() {
  // Count and type assertions catch an invisible survivor, duplicated prefab, or
  // wrong asset even if the moving wrapper reaches the right square.
  let mut game = ChessTest::from_position(fixtures::capture());
  game.play(Square::D4, Square::E5);
  game.expect_piece(Square::E5, Color::White, Piece::Bishop);
  assert_eq!(host::pieces(&game.display).len(), 3);
  assert!(
    game
      .display
      .audio_occurrences()
      .iter()
      .any(|o| audio::CAPTURE_SOUNDS.contains(&o.address))
  );
  assert_eq!(
    game.display.particle_occurrences()[0].address,
    assets::effects::CAPTURE
  );
}

#[test]
fn a_knight_capture_finishes_at_its_destination() {
  // The knight's arcing animation differs from sliding pieces. Finite virtual
  // presentation runs it to completion; no wall-clock delay approximates arrival.
  let mut game = ChessTest::from_position(fixtures::knight_capture());
  game.play(Square::D4, Square::E6);
  game.expect_piece(Square::E6, Color::White, Piece::Knight);
  assert_eq!(host::pieces(&game.display).len(), 3);
}

#[test]
fn castling_moves_both_visible_pieces() {
  // Player intent names the king's destination. The rules library's internal
  // king-to-rook representation is deliberately absent from this scenario.
  let mut game = ChessTest::from_position(fixtures::castle());
  game.play(Square::E1, Square::G1);
  game.expect_piece(Square::G1, Color::White, Piece::King);
  game.expect_piece(Square::F1, Color::White, Piece::Rook);
  game.expect_empty(Square::H1);
  host::sound(&game.display, audio::CASTLE_SOUND);
}

#[test]
fn en_passant_removes_the_piece_beside_the_destination() {
  // En-passant metadata is part of the logical fixture, not a parsed save. The
  // victim lies off the destination, so checking only arrival would miss a bug.
  let mut game = ChessTest::from_position(fixtures::en_passant());
  game.play(Square::E5, Square::D6);
  game.expect_piece(Square::D6, Color::White, Piece::Pawn);
  game.expect_empty(Square::D5);
  assert_eq!(host::pieces(&game.display).len(), 3);
}

#[test]
fn underpromotion_replaces_the_rendered_asset_and_captures() {
  // Fast gameplay supplies a typed answer before input. choose() remains a
  // synchronous call on the original rules stack. Dialog routing has its own
  // worker test; it would be dishonest to claim this checks the dialog.
  let mut game = ChessTest::from_position(fixtures::promotion());
  game.promote(Square::A7, Square::B8, Piece::Knight);
  game.expect_piece(Square::B8, Color::White, Piece::Knight);
  assert_eq!(host::pieces(&game.display).len(), 3);
  host::sound(&game.display, audio::PROMOTION_SOUND);
}

#[test]
fn check_is_a_presented_sound_after_arrival() {
  // A sound occurrence is actual host output, unlike a hidden 'check' marker.
  let mut game = ChessTest::from_position(fixtures::check());
  game.play(Square::A2, Square::A8);
  game.expect_piece(Square::A8, Color::White, Piece::Rook);
  host::sound(&game.display, audio::CHECK_SOUND);
}

#[test]
fn checkmate_presents_victory() {
  // Inspect both the decisive piece and terminal feedback; neither internal
  // GameStatus nor a diagnostic label substitutes for player-visible output.
  let mut game = ChessTest::from_position(fixtures::player_win());
  game.play(Square::G6, Square::G7);
  game.expect_piece(Square::G7, Color::White, Piece::Queen);
  host::sound(&game.display, audio::PLAYER_WIN_SOUND);
}

#[test]
fn stalemate_presents_a_draw() {
  // This position ends immediately, so no opponent script or search is needed.
  let mut game = ChessTest::from_position(fixtures::draw());
  game.play(Square::C7, Square::B6);
  game.expect_piece(Square::B6, Color::White, Piece::Queen);
  host::sound(&game.display, audio::DRAW_SOUND);
}

#[test]
fn a_scripted_computer_mate_presents_defeat() {
  // A black-to-move fixture stays untouched until the test explicitly admits its
  // reply. The same production computer action publishes the resulting motion.
  let mut game = ChessTest::from_position(fixtures::computer_win());
  game.expect_piece(Square::G3, Color::Black, Piece::Queen);
  game.reply(Square::G3, Square::G2);
  game.expect_piece(Square::G2, Color::Black, Piece::Queen);
  host::sound(&game.display, audio::PLAYER_LOSS_SOUND);
}
