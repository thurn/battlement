//! Guardrails for the sample's intent helpers. A scripted expectation must be
//! consumed, and a successful move must pass through an admitted UI action.
//! Generic prefab/transform observation tests live in reactant-testing.
mod support;
use crate::support::{fixtures, game::ChessTest};
use cozy_chess::{Piece, Square};

#[test]
#[should_panic(expected = "visible input did not admit a chess action")]
fn play_rejects_input_blocked_by_pause() {
  // Direct rules dispatch would bypass the menu and incorrectly pass this test.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.pause();
  game.play(Square::E2, Square::E4);
}

#[test]
#[should_panic(expected = "unused choice")]
fn promotion_rejects_an_unused_answer() {
  // Scripted choices are expectations, not optional hints for an ordinary pawn move.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.promote(Square::E2, Square::E4, Piece::Knight);
}

#[test]
#[should_panic(expected = "unused computer reply")]
fn reply_rejects_a_move_during_the_player_turn() {
  // A held opponent cannot consume a reply while White owns the turn.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.reply(Square::E7, Square::E5);
}
