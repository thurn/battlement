//! Menu scenarios exercise the same public host interface as gameplay.
mod support;

use battlement::{CheckedState, SemanticRole};
use cozy_chess::{Color, Piece, Square};
use support::game::ChessTest;

#[test]
fn menu_opens_settings_and_starts_the_3d_game() {
  let mut chess = ChessTest::title();
  assert_eq!(
    chess.display.semantic_node("Chess Chess Revolution").role,
    SemanticRole::Heading
  );
  chess.display.expect_button("PLAY");
  chess.display.activate_accessible("SETTINGS");
  assert_eq!(
    chess.display.semantic_node("Settings").role,
    SemanticRole::Heading
  );
  chess.display.activate_accessible("RETURN");
  chess.display.expect_button("PLAY");

  chess.start();
  chess.expect_piece(Square::E2, Color::White, Piece::Pawn);
  chess.display.expect_button("Main menu");
}

#[test]
fn gameplay_can_return_to_menu_and_resume_the_retained_game() {
  let mut chess = ChessTest::title();
  chess.start();
  chess.play(Square::E2, Square::E4);
  chess.show_menu();
  chess.display.expect_button("PLAY");
  chess.start();
  chess.expect_piece(Square::E4, Color::White, Piece::Pawn);
  chess.expect_empty(Square::E2);
}

#[test]
fn settings_keep_player_choices_across_menu_navigation() {
  let mut chess = ChessTest::title();
  chess.display.activate_accessible("SETTINGS");
  chess.display.activate_accessible("Text Size 100%");
  chess.display.activate_accessible("200%");
  chess.display.activate_accessible("Reduce Motion");
  chess.display.activate_accessible("RETURN");
  chess.display.activate_accessible("SETTINGS");

  chess.display.expect_button("Text Size 200%");
  assert_eq!(
    chess.display.semantic_node("Reduce Motion").state.checked,
    Some(CheckedState::True)
  );
  chess.display.activate_accessible("Sound");
  assert_eq!(
    chess.display.semantic_node("Master Volume").role,
    SemanticRole::Slider
  );
  chess.display.activate_accessible("Input");
  assert!(chess.display.accessibility().nodes.iter().any(|node| {
    node.role == SemanticRole::Table && node.label.as_deref() == Some("Input bindings")
  }));
  chess.display.activate_accessible("RETURN");
  chess.start();
  chess.show_menu();
  chess.display.activate_accessible("SETTINGS");
  chess.display.expect_button("Text Size 200%");
  assert_eq!(
    chess.display.semantic_node("Reduce Motion").state.checked,
    Some(CheckedState::True)
  );
}
