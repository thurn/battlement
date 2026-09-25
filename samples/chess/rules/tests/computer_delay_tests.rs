//! Computer admission follows completed presentation and unpaused application time.
mod support;

use battlement::application::ApplicationState;
use cozy_chess::{Color, Piece, Square};
use std::time::Duration;

use crate::support::{fixtures, game::ChessTest};

fn waiting() -> ChessTest {
  let mut game = ChessTest::from_position(fixtures::initial());
  game.play(Square::E2, Square::E4);
  game.permit_reply(Square::E7, Square::E5);
  game
}

#[test]
fn computer_waits_two_seconds_after_the_player_presentation_and_dispatches_once() {
  let mut game = self::waiting();
  game.advance(Duration::from_millis(1999));
  game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
  game.advance(Duration::from_millis(1));
  game.display.settle();
  game.expect_piece(Square::E5, Color::Black, Piece::Pawn);
  game.advance(Duration::from_secs(20));
  game.expect_piece(Square::E5, Color::Black, Piece::Pawn);
  game.expect_empty(Square::E7);
}

#[test]
fn pause_and_background_preserve_remaining_time() {
  let mut game = self::waiting();
  game.advance(Duration::from_millis(800));
  game.pause();
  game.advance(Duration::from_secs(20));
  game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
  game.pause();
  game.advance(Duration::from_millis(400));
  game.display.set_application_state(ApplicationState {
    focused: false,
    paused: true,
  });
  game.display.settle();
  game.advance(Duration::from_secs(20));
  game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
  game
    .display
    .set_application_state(ApplicationState::default());
  game.display.settle();
  game.advance(Duration::from_millis(799));
  game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
  game.advance(Duration::from_millis(1));
  game.display.settle();
  game.expect_piece(Square::E5, Color::Black, Piece::Pawn);
}

#[test]
fn menu_suspends_the_wait_and_disabling_the_setting_releases_it_on_return() {
  let mut game = self::waiting();
  game.advance(Duration::from_millis(500));
  game.show_menu();
  game.advance(Duration::from_secs(20));
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Increase Move Duration");
  game.display.activate_accessible("RETURN");
  game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
  game.start();
  game.expect_piece(Square::E5, Color::Black, Piece::Pawn);
  game.show_menu();
  game.display.activate_accessible("SETTINGS");
  game.display.activate_accessible("Increase Move Duration");
  game.display.activate_accessible("RETURN");
  game.start();
  game.advance(Duration::from_secs(10));
  game.expect_piece(Square::E5, Color::Black, Piece::Pawn);
}

#[test]
fn restarting_during_a_wait_cancels_the_old_turn_and_gives_the_next_turn_a_full_delay() {
  let mut game = self::waiting();
  game.advance(Duration::from_secs(1));
  game.restart();
  game.advance(Duration::from_secs(10));
  game.expect_board(&fixtures::initial());
  game.play(Square::D2, Square::D4);
  game.permit_reply(Square::D7, Square::D5);
  game.advance(Duration::from_millis(1999));
  game.expect_piece(Square::D7, Color::Black, Piece::Pawn);
  game.advance(Duration::from_millis(1));
  game.display.settle();
  game.expect_piece(Square::D5, Color::Black, Piece::Pawn);
  game.expect_piece(Square::E7, Color::Black, Piece::Pawn);
}
