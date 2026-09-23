//! Input tests name bindings deliberately. Gameplay tests use chess intent instead.
//! Shared gesture helpers send real host input and complete finite presentation;
//! every assertion below observes decoded output, never the rules board.
mod support;
use crate::support::{fixtures, game::ChessTest};
use battlement::{ControllerButton, ControllerDirection, PhysicalKey};
use chess_rules::{assets, audio};
use cozy_chess::{Color, Piece, Square};
use reactant_testing::ActionResult;

#[test]
fn play_button_starts_a_game() {
  // The title exposes usable controls; starting presents the board and opening effects.
  let mut game = ChessTest::title();
  game.display.expect_button("PLAY");
  game.display.expect_key_enabled(PhysicalKey::Enter);
  game
    .display
    .expect_controller_button_enabled(ControllerButton::South);
  game.start();
  assert_eq!(game.pieces().len(), 32);
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  game
    .display
    .expect_particles(assets::effects::PIECE_SPAWN, 32);
  game.display.expect_sound(audio::START_SOUND);
  game.display.expect_sound(assets::music::CRITICAL);
}

#[test]
fn keyboard_moves_a_pawn() {
  // Selection and navigation use the production cursor, without inspecting its state.
  let mut game = ChessTest::title();
  game.start();
  game.display.send_keys(&[
    PhysicalKey::ArrowRight,
    PhysicalKey::Enter,
    PhysicalKey::ArrowUp,
    PhysicalKey::ArrowUp,
    PhysicalKey::Enter,
  ]);
  game.expect_empty(Square::F2);
  game.expect_piece(Square::F4, Color::White, Piece::Pawn);
}

#[test]
fn controller_moves_a_pawn() {
  // Separate navigation and submit operations make the intended gesture readable.
  let mut game = ChessTest::title();
  game.start();
  game
    .display
    .navigate_controller(&[ControllerDirection::Left]);
  game
    .display
    .press_controller_button(ControllerButton::South);
  game
    .display
    .navigate_controller(&[ControllerDirection::Up, ControllerDirection::Up]);
  game
    .display
    .press_controller_button(ControllerButton::South);
  game.expect_empty(Square::D2);
  game.expect_piece(Square::D4, Color::White, Piece::Pawn);
}

#[test]
fn illegal_move_restores_the_pawn() {
  // Rejection must restore the visual position and provide audible feedback.
  let mut game = ChessTest::from_position(fixtures::initial());
  assert_eq!(
    game.attempt_move(Square::E2, Square::E5),
    ActionResult::NoAction
  );
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  game.expect_empty(Square::E5);
  game.display.expect_sound(audio::INVALID_DROP_SOUND);
}

#[test]
fn off_board_drop_restores_the_pawn() {
  // The adapter owns the off-board coordinate; this scenario specifies only intent.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.drop_off_board(Square::E2);
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  assert_eq!(game.pieces().len(), 32);
}

#[test]
fn cancelled_drag_restores_the_pawn() {
  // Capture cancellation must not submit a move or admit an opponent response.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.cancel_move(Square::E2, Square::E4);
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  game.expect_empty(Square::E4);
}

#[test]
fn pause_blocks_board_input() {
  // A visible object behind a modal must not remain interactive.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.pause();
  assert_eq!(
    game.attempt_move(Square::E2, Square::E4),
    ActionResult::NoAction
  );
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  game.expect_empty(Square::E4);
}

#[test]
fn accessibility_actions_move_a_pawn() {
  // Labels resolve live accessibility nodes; activation follows the real event route.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.display.activate_accessible("White Pawn at e2");
  game.display.activate_accessible("Move to e4");
  game.expect_empty(Square::E2);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
}
