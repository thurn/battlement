mod support;

use std::time::Duration;

use battlement::{
  ControllerButton, ControllerDirection, DragMode, PhysicalKey, SemanticRole, Vector3,
};
use chess_rules::{ChessGame, contract};
use reactant_testing::GameActionResult;

use crate::support::{
  CAPTURE, DEFAULT, MemoryPersistence, TIMEOUT, assert_state, click_play, client, drag_input,
  piece, piece_at, play_move, press_key, square, start_with_controller, start_with_keyboard,
};

#[test]
fn empty_store_opens_an_accessible_title_and_complete_board() {
  let mut display = client(MemoryPersistence::empty(), Duration::ZERO);
  assert_state(&display, contract::marker::TITLE);

  let play = display
    .accessibility()
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Play chess"))
    .expect("Play is exposed to Unity accessibility");
  assert_eq!(play.role, SemanticRole::Button);
  assert_eq!(play.label.as_deref(), Some("Play chess"));
  assert!(play.actions.activate);
  assert!(display.global_keys().contains(&PhysicalKey::Enter));
  let controller = display.controller_input().expect("controller input");
  assert!(controller.buttons.contains(&ControllerButton::South));
  assert_eq!(controller.stick_dead_zone, Some(0.35));
  assert_eq!(controller.repeat_delay_ms, Some(275));
  assert_eq!(controller.repeat_interval_ms, Some(125));

  click_play(&mut display);
  assert_state(&display, contract::marker::INITIAL);
  assert!(
    display
      .audio_occurrences()
      .iter()
      .any(|effect| effect.address == contract::START_SOUND)
  );
  assert!(
    display
      .audio_occurrences()
      .iter()
      .any(|effect| { effect.address == contract::MUSIC_TRACKS[0] && effect.looping })
  );
  let pawn = piece(&display, 'e', 2);
  assert_eq!(display.particle_occurrences().len(), 32);
  assert!(
    display
      .objects()
      .filter(|object| matches!(
        object.kind(),
        battlement::GameObjectKind::BoxHitRegion { .. }
      ))
      .all(|object| object.local_transform().scale == Vector3::ONE)
  );
  assert_eq!(
    display.object(pawn).unwrap().drag_mode(),
    Some(DragMode::SnapToPointer)
  );
}

#[test]
fn keyboard_and_controller_drive_the_same_visible_move_contract() {
  let mut keyboard = client(MemoryPersistence::empty(), Duration::ZERO);
  start_with_keyboard(&mut keyboard);
  let pawn = piece(&keyboard, 'f', 2);
  assert_eq!(
    keyboard.game_action_presented::<ChessGame>(
      TIMEOUT,
      |display| {
        for key in [
          PhysicalKey::ArrowRight,
          PhysicalKey::Enter,
          PhysicalKey::ArrowUp,
          PhysicalKey::ArrowUp,
          PhysicalKey::Enter,
        ] {
          press_key(display, key);
        }
      },
      |display| display.world_point(pawn, Vector3::ZERO) == square('f', 4),
    ),
    GameActionResult::Completed
  );
  assert!(piece_at(&keyboard, 'f', 2).is_none());
  assert!(piece_at(&keyboard, 'f', 4).is_some());

  let mut controller = client(MemoryPersistence::empty(), Duration::ZERO);
  start_with_controller(&mut controller);
  let pawn = piece(&controller, 'd', 2);
  assert_eq!(
    controller.game_action_presented::<ChessGame>(
      TIMEOUT,
      |display| {
        display.controller_navigate(0, ControllerDirection::Left);
        display.controller_button_down(0, ControllerButton::South);
        display.controller_button_up(0, ControllerButton::South);
        display.controller_navigate(0, ControllerDirection::Up);
        display.controller_navigate(0, ControllerDirection::Up);
        display.controller_button_down(0, ControllerButton::South);
        display.controller_button_up(0, ControllerButton::South);
      },
      |display| display.world_point(pawn, Vector3::ZERO) == square('d', 4),
    ),
    GameActionResult::Completed
  );
  assert!(piece_at(&controller, 'd', 2).is_none());
  assert!(piece_at(&controller, 'd', 4).is_some());
}

#[test]
fn native_drag_commits_only_legal_board_drops() {
  let mut legal = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  let pawn = piece(&legal, 'e', 2);
  let input = drag_input(7);
  assert_eq!(
    legal.game_action_presented::<ChessGame>(
      TIMEOUT,
      |display| {
        display.drag_start(pawn, input);
        display.drag_end(pawn, input, square('e', 4));
      },
      |display| display.world_point(pawn, Vector3::ZERO) == square('e', 4),
    ),
    GameActionResult::Completed
  );
  assert!(piece_at(&legal, 'e', 2).is_none());
  assert_eq!(legal.world_point(pawn, Vector3::ZERO), square('e', 4));

  let mut illegal = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  let pawn = piece(&illegal, 'e', 2);
  let input = drag_input(8);
  illegal.drag_start(pawn, input);
  illegal.drag_end(pawn, input, square('e', 5));
  illegal.flush();
  assert_eq!(illegal.world_point(pawn, Vector3::ZERO), square('e', 2));
  assert_eq!(
    illegal.audio_occurrences().last().unwrap().address,
    contract::INVALID_DROP_SOUND
  );

  let mut outside = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  let pawn = piece(&outside, 'e', 2);
  let input = drag_input(9);
  outside.drag_start(pawn, input);
  outside.drag_end(pawn, input, Vector3::new(8.0, 0.0, 0.0));
  outside.flush();
  assert_eq!(outside.world_point(pawn, Vector3::ZERO), square('e', 2));
}

#[test]
fn semantic_move_action_captures_the_visible_target() {
  let mut display = client(MemoryPersistence::with_save(CAPTURE), Duration::ZERO);
  play_move(&mut display, ('d', 4), ('e', 5));
  assert!(piece_at(&display, 'd', 4).is_none());
  assert!(piece_at(&display, 'e', 5).is_some());
}
