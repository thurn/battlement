//! Live dialogs need the real blocking rules worker; ordinary tests script choices.
//! Both use the same Display API. Only this configuration waits on notifications.
mod support;
use crate::support::{fixtures, game::ChessTest};
use chess_rules::assets;
use cozy_chess::{Color, Piece, Square};
use reactant_testing::ActionResult;
use std::time::Duration;

#[test]
fn promotion_dialog_resumes_the_move() {
  // Choosing an actual displayed button resumes the original synchronous choose call.
  let mut game = ChessTest::live_dialog(fixtures::promotion());
  assert_eq!(
    game.attempt_move(Square::A7, Square::B8),
    ActionResult::AwaitingInput
  );
  game.display.expect_button("Knight");
  game.display.expect_prefab_count(assets::white::KNIGHT, 0);
  game.display.click_button("Knight");
  game.expect_empty(Square::A7);
  game.expect_piece(Square::B8, Color::White, Piece::Knight);
  assert_eq!(game.pieces().len(), 3);
}

#[test]
fn restart_discards_a_pending_promotion_answer() {
  // A captured event belongs to the old session and cannot mutate the replacement.
  let mut game = ChessTest::live_dialog(fixtures::promotion());
  assert_eq!(
    game.attempt_move(Square::A7, Square::B8),
    ActionResult::AwaitingInput
  );
  let stale_answer = game.display.button_event("Knight");
  game.restart();
  game
    .display
    .action(|display| display.deliver_ui_event(stale_answer));
  game.expect_piece(Square::A2, Color::White, Piece::Pawn);
  game.expect_piece(Square::B8, Color::Black, Piece::Knight);
  assert_eq!(game.pieces().len(), 32);
}

#[test]
fn zero_budget_computer_plays_a_deterministic_reply() {
  // This tests the production selector, not the scripted opponent. Independent
  // games must produce the same complete visible board, including unmoved pieces.
  let mut first = ChessTest::with_computer(fixtures::initial());
  let mut second = ChessTest::with_computer(fixtures::initial());
  first.play(Square::E2, Square::E4);
  second.play(Square::E2, Square::E4);
  first.advance(Duration::from_secs(2));
  second.advance(Duration::from_secs(2));
  first.display.settle();
  second.display.settle();
  first.expect_piece(Square::E4, Color::White, Piece::Pawn);
  first.expect_piece(Square::A5, Color::Black, Piece::Pawn);
  first.expect_empty(Square::A7);
  first.expect_board(&fixtures::opening_reply());
  assert_eq!(first.pieces(), second.pieces());
}
