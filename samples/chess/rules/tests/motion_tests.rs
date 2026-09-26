//! Motion policy is checked through real chess input and host presentation.
mod support;

use std::time::Duration;

use battlement::{
  UiAccessibilityAction, UiAccessibilityActionEvent, UiEvent, UiEventBody,
  application::ReducedMotionPreference,
};
use chess_rules::settings::ChessSettings;
use cozy_chess::{Color, Piece, Square};
use reactant_testing::MemoryPersistence;
use support::{board, fixtures, game::ChessTest};

fn activate_without_time(game: &mut ChessTest, label: &str) {
  game.display.deliver_ui_event(UiEvent {
    target_id: game.display.semantic_node(label).object_id,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 0,
      action: UiAccessibilityAction::Activate,
    }),
  });
  game.advance(Duration::ZERO);
}

fn reduced_game(saved: bool, system: bool) -> ChessTest {
  let settings = ChessSettings {
    reduce_motion: saved,
    ..ChessSettings::default()
  };
  let mut game = ChessTest::assemble(
    Some(fixtures::initial()),
    Some(MemoryPersistence::with_file(
      "memory/chess-settings.json",
      &serde_json::to_vec(&settings).unwrap(),
    )),
    &[],
  );
  game.display.set_reduced_motion_preference(if system {
    ReducedMotionPreference::Reduce
  } else {
    ReducedMotionPreference::NoPreference
  });
  game.display.settle();
  game
}

#[test]
fn saved_or_system_motion_policy_keeps_a_visible_120ms_slide() {
  for (saved, system) in [(true, false), (false, true), (true, true)] {
    let mut game = reduced_game(saved, system);
    game
      .display
      .activate_accessible("\u{2068}White Pawn\u{2069} at \u{2068}e2\u{2069}");
    activate_without_time(&mut game, "Move to \u{2068}e4\u{2069}");
    game.advance(Duration::from_millis(60));
    assert!(
      board::pieces(&game.display)
        .iter()
        .any(|(position, asset)| asset.as_str() == "white/pawn"
          && (position.x - 0.5).abs() < 0.001
          && (position.z + 1.5).abs() < 0.01)
    );
    game.advance(Duration::from_millis(60));
    game.display.settle();
    game.expect_piece(Square::E4, Color::White, Piece::Pawn);
    assert!(game.display.particle_occurrences().is_empty());
  }
}

#[test]
fn reduced_knights_and_castling_take_direct_parallel_paths() {
  for (position, piece, from, to, checkpoints) in [
    (
      fixtures::initial(),
      "Knight",
      "g1",
      "f3",
      vec![("white/knight", 2.0, -2.5)],
    ),
    (
      fixtures::castle(),
      "King",
      "e1",
      "g1",
      vec![("white/king", 1.5, -3.5), ("white/rook", 2.5, -3.5)],
    ),
  ] {
    let mut game = ChessTest::from_position(position);
    game
      .display
      .set_reduced_motion_preference(ReducedMotionPreference::Reduce);
    game.display.settle();
    game.display.activate_accessible(&format!(
      "\u{2068}White {piece}\u{2069} at \u{2068}{from}\u{2069}"
    ));
    activate_without_time(&mut game, &format!("Move to \u{2068}{to}\u{2069}"));
    game.advance(Duration::from_millis(60));
    for (address, x, z) in checkpoints {
      assert!(
        game
          .pieces()
          .iter()
          .any(|(point, asset)| asset.as_str() == address
            && (point.x - x).abs() < 0.01
            && (point.z - z).abs() < 0.01),
        "{address}: {:?}",
        game.pieces()
      );
    }
    game.advance(Duration::from_millis(60));
    game.display.settle();
  }
}

#[test]
fn move_sounds_follow_the_selected_arrival_time() {
  for (position, piece, from, to, reduced, millis) in [
    (fixtures::capture(), "Bishop", "d4", "e5", true, 120),
    (fixtures::initial(), "Pawn", "e2", "e4", true, 120),
    (fixtures::initial(), "Knight", "g1", "f3", false, 320),
  ] {
    let mut game = ChessTest::from_position(position);
    if reduced {
      game
        .display
        .set_reduced_motion_preference(ReducedMotionPreference::Reduce);
      game.display.settle();
    }
    game.display.activate_accessible(&format!(
      "\u{2068}White {piece}\u{2069} at \u{2068}{from}\u{2069}"
    ));
    let sounds = game.display.audio_occurrences().len();
    activate_without_time(&mut game, &format!("Move to \u{2068}{to}\u{2069}"));
    game.advance(Duration::from_millis(millis - 1));
    assert_eq!(game.display.audio_occurrences().len(), sounds);
    game.advance(Duration::from_millis(1));
    assert_eq!(game.display.audio_occurrences().len(), sounds + 1);
    if reduced {
      assert!(game.display.particle_occurrences().is_empty());
    }
  }
}

#[test]
fn changing_system_policy_mid_knight_move_settles_without_replaying_effects() {
  let mut game = ChessTest::from_position(fixtures::initial());
  game
    .display
    .activate_accessible("\u{2068}White Knight\u{2069} at \u{2068}g1\u{2069}");
  activate_without_time(&mut game, "Move to \u{2068}f3\u{2069}");
  game.advance(Duration::from_millis(50));
  let sounds = game.display.audio_occurrences().len();
  game
    .display
    .set_reduced_motion_preference(ReducedMotionPreference::Reduce);
  game.advance(Duration::ZERO);
  game.expect_piece(Square::F3, Color::White, Piece::Knight);
  game.display.settle();
  assert_eq!(game.display.audio_occurrences().len(), sounds);
  game.permit_reply(Square::E7, Square::E5);
  game.advance(Duration::from_secs(2));
  game.display.settle();
  game.expect_piece(Square::E5, Color::Black, Piece::Pawn);
}

#[test]
fn capture_shakes_visuals_and_system_reduction_restores_the_board_baseline() {
  let mut game = ChessTest::from_position(fixtures::capture());
  game
    .display
    .activate_accessible("\u{2068}White Bishop\u{2069} at \u{2068}d4\u{2069}");
  activate_without_time(&mut game, "Move to \u{2068}e5\u{2069}");
  game.advance(Duration::from_millis(330));
  let position = game
    .display
    .prefabs()
    .into_iter()
    .find(|(_, asset)| asset.as_str() == "board")
    .unwrap()
    .0;
  assert!(
    position.x.abs() > 0.005 && position.x.abs() <= 0.031,
    "{position:?}"
  );
  game
    .display
    .set_reduced_motion_preference(ReducedMotionPreference::Reduce);
  game.advance(Duration::ZERO);
  let position = game
    .display
    .prefabs()
    .into_iter()
    .find(|(_, asset)| asset.as_str() == "board")
    .unwrap()
    .0;
  assert!(
    position.x.abs() < 0.0001 && position.z.abs() < 0.0001,
    "{position:?}"
  );
  game.display.settle();
  game.expect_piece(Square::E5, Color::White, Piece::Bishop);
}

#[test]
fn disabling_shake_during_navigation_restores_baseline_without_replaying_it() {
  let mut game = ChessTest::from_position(fixtures::capture());
  game
    .display
    .activate_accessible("\u{2068}White Bishop\u{2069} at \u{2068}d4\u{2069}");
  activate_without_time(&mut game, "Move to \u{2068}e5\u{2069}");
  game.advance(Duration::from_millis(330));
  activate_without_time(&mut game, "Main menu");
  activate_without_time(&mut game, "SETTINGS");
  activate_without_time(&mut game, "Graphics");
  let x = game
    .display
    .prefabs()
    .into_iter()
    .find(|(_, asset)| asset.as_str() == "board")
    .unwrap()
    .0
    .x;
  assert!(
    x.abs() > 0.005,
    "shake should still be active before disabling: {x}"
  );
  activate_without_time(&mut game, "Screenshake");
  for _ in 0..2 {
    let point = game
      .display
      .prefabs()
      .into_iter()
      .find(|(_, asset)| asset.as_str() == "board")
      .unwrap()
      .0;
    assert!(
      point.x.abs() < 0.0001 && point.z.abs() < 0.0001,
      "{point:?}"
    );
    activate_without_time(&mut game, "Screenshake");
  }
}
