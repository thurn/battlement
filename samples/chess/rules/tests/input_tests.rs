//! Input-specific tests intentionally name gestures and bindings. Keep those
//! details here; gameplay tests use intent helpers so a UI redesign has one home.
mod support;
use crate::support::{fixtures, game::ChessTest, host};
use battlement::{ControllerButton, ControllerDirection, PhysicalKey, SemanticRole, Vector3};
use battlement::{UiAccessibilityAction, UiEventBody};
use chess_rules::{assets, audio};
use cozy_chess::{Color, Piece, Square};

#[test]
fn the_visible_title_starts_a_complete_board() {
  // The accessible node belongs to the displayed world control. Pointer input is
  // geometrically picked at its projected center rather than aimed at a test ID.
  let mut game = ChessTest::title();
  let play = game
    .display
    .accessibility()
    .nodes
    .iter()
    .find(|n| n.label.as_deref() == Some("Play chess"))
    .unwrap();
  assert_eq!(play.role, SemanticRole::Button);
  assert!(play.actions.activate);
  assert!(game.display.global_keys().contains(&PhysicalKey::Enter));
  assert!(
    game
      .display
      .controller_input()
      .unwrap()
      .buttons
      .contains(&ControllerButton::South)
  );
  game.start();
  assert_eq!(host::pieces(&game.display).len(), 32);
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  assert_eq!(game.display.particle_occurrences().len(), 32);
  host::sound(&game.display, audio::START_SOUND);
  host::sound(&game.display, assets::music::CRITICAL);
}

#[test]
fn keyboard_input_moves_the_selected_piece() {
  // These keystrokes exercise the real global input policy and cursor state.
  // Assertions still observe prefab output; the cursor's implementation is private.
  let mut game = ChessTest::title();
  host::key(&mut game.display, PhysicalKey::Enter);
  game.display.finish_inline();
  for key in [
    PhysicalKey::ArrowRight,
    PhysicalKey::Enter,
    PhysicalKey::ArrowUp,
    PhysicalKey::ArrowUp,
    PhysicalKey::Enter,
  ] {
    host::key(&mut game.display, key);
  }
  game.display.finish_inline();
  game.expect_empty(Square::F2);
  game.expect_piece(Square::F4, Color::White, Piece::Pawn);
}

#[test]
fn controller_input_moves_the_selected_piece() {
  // Controller navigation and submit converge on the same production input policy
  // as keyboard and pointer. Separate tests prevent one working path masking another.
  let mut game = ChessTest::title();
  for direction in [
    None,
    Some(ControllerDirection::Left),
    Some(ControllerDirection::Up),
    Some(ControllerDirection::Up),
  ] {
    if let Some(direction) = direction {
      game.display.controller_navigate(0, direction);
    }
    if direction != Some(ControllerDirection::Up) {
      game
        .display
        .controller_button_down(0, ControllerButton::South);
      game
        .display
        .controller_button_up(0, ControllerButton::South);
      game.display.finish_inline();
    }
  }
  game
    .display
    .controller_button_down(0, ControllerButton::South);
  game
    .display
    .controller_button_up(0, ControllerButton::South);
  game.display.finish_inline();
  game.expect_empty(Square::D2);
  game.expect_piece(Square::D4, Color::White, Piece::Pawn);
}

#[test]
fn illegal_and_off_board_drops_restore_the_piece() {
  // Unlike the successful-move helper, this test expects rejection. Real geometric
  // drag input exercises picking and drop projection without supplying an object ID.
  for destination in [host::center(Square::E5), Vector3::new(8.0, 0.0, 0.0)] {
    let mut game = ChessTest::from_position(fixtures::initial());
    let from = game
      .display
      .project_world(host::center(Square::E2))
      .unwrap();
    let to = game.display.project_world(destination).unwrap();
    game.display.pointer_down(0, from);
    game.display.pointer_move(0, to, true);
    game.display.pointer_up(0, to);
    game.display.finish_inline();
    game.expect_piece(Square::E2, Color::White, Piece::Pawn);
    game.expect_empty(Square::E5);
    if destination == host::center(Square::E5) {
      host::sound(&game.display, audio::INVALID_DROP_SOUND);
    }
  }
}

#[test]
fn cancelled_drag_leaves_the_visible_board_unchanged() {
  // Cancellation belongs to the host's gesture lifecycle. It must restore the
  // displaced object without submitting a chess move or starting an opponent turn.
  let mut game = ChessTest::from_position(fixtures::initial());
  let from = game
    .display
    .project_world(host::center(Square::E2))
    .unwrap();
  let to = game
    .display
    .project_world(host::center(Square::E4))
    .unwrap();
  game.display.pointer_down(0, from);
  game.display.pointer_move(0, to, true);
  game.display.pointer_cancel(0);
  game.display.finish_inline();
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  game.expect_empty(Square::E4);
}

#[test]
fn pause_intercepts_gameplay_input() {
  // A piece still exists behind the pause overlay. Existence alone must not grant
  // permission to move it; the same high-level move gesture must now be rejected.
  let mut game = ChessTest::from_position(fixtures::initial());
  game.pause();
  host::move_piece(&mut game.display, Square::E2, Square::E4);
  game.display.finish_inline();
  game.expect_piece(Square::E2, Color::White, Piece::Pawn);
  game.expect_empty(Square::E4);
}

#[test]
fn assistive_activation_selects_a_visible_piece_and_destination() {
  // Accessibility is a real user interface, not an alternate test command channel.
  // Labels select live world hosts; actions route through the same chess controller.
  let mut game = ChessTest::from_position(fixtures::initial());
  for label in ["White Pawn at e2", "Move to e4"] {
    let id = game
      .display
      .accessibility()
      .nodes
      .iter()
      .find(|n| n.label.as_deref() == Some(label))
      .expect("visible accessible board element")
      .object_id;
    game.display.deliver_ui_event(battlement::UiEvent {
      target_id: id,
      cancelable: true,
      default_prevented: false,
      body: UiEventBody::AccessibilityAction(battlement::UiAccessibilityActionEvent {
        backend_generation: 0,
        action: UiAccessibilityAction::Activate,
      }),
    });
    game.display.finish_inline();
  }
  game.expect_empty(Square::E2);
  game.expect_piece(Square::E4, Color::White, Piece::Pawn);
}
