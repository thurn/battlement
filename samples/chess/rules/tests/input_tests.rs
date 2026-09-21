mod support;

use std::time::Duration;

use battlement::{
  ControllerButton, ControllerDirection, DragMode, PhysicalKey, SemanticRole, Vector3,
};
use battlement_rules::contract;

use crate::support::{
  CAPTURE, DEFAULT, MemoryPersistence, click_play, client, drag_input, piece, piece_at, square,
  start_with_keyboard, wait_for_marker,
};
use battlement::ControllerNavigationSource;

#[test]
fn empty_store_opens_an_accessible_title_and_complete_board() {
  let mut client = client(MemoryPersistence::empty(), Duration::ZERO);
  wait_for_marker(&mut client, contract::marker::TITLE);

  let play = client
    .accessibility()
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some("Play chess"))
    .expect("Play is exposed to Unity accessibility");
  assert_eq!(play.role, SemanticRole::Button);
  assert_eq!(play.label.as_deref(), Some("Play chess"));
  assert!(play.actions.activate);
  assert!(client.world().global_keys().contains(&PhysicalKey::Enter));
  let controller = client.world().controller_input().expect("controller input");
  assert!(controller.buttons.contains(&ControllerButton::South));
  assert_eq!(controller.stick_dead_zone, Some(0.35));
  assert_eq!(controller.repeat_delay_ms, Some(275));
  assert_eq!(controller.repeat_interval_ms, Some(125));

  click_play(&mut client);
  for _ in 0..4 {
    client.poll();
  }
  assert!(
    client
      .audio_occurrences()
      .iter()
      .any(|effect| { effect.address == contract::START_SOUND })
  );
  assert!(
    client
      .audio_occurrences()
      .iter()
      .any(|effect| { effect.address == contract::MUSIC_TRACKS[0] && effect.looping })
  );
  let pawn = piece(&client, 'e', 2);
  assert_eq!(client.world().object(pawn).unwrap().drag_mode(), None);
  assert!(
    client
      .world()
      .objects()
      .filter(|object| {
        matches!(
          object.kind(),
          battlement::GameObjectKind::BoxHitRegion { .. }
        )
      })
      .all(|object| object.local_transform().scale == Vector3::ZERO)
  );

  client.settle();
  assert_eq!(client.particle_occurrences().len(), 32);
  assert!(
    client
      .world()
      .objects()
      .filter(|object| {
        matches!(
          object.kind(),
          battlement::GameObjectKind::BoxHitRegion { .. }
        )
      })
      .all(|object| object.local_transform().scale == Vector3::ONE)
  );
  assert_eq!(
    client
      .world()
      .object(piece(&client, 'e', 2))
      .unwrap()
      .drag_mode(),
    Some(DragMode::SnapToPointer)
  );
}

#[test]
fn keyboard_and_controller_drive_the_same_visible_move_contract() {
  let mut keyboard = client(MemoryPersistence::empty(), Duration::ZERO);
  start_with_keyboard(&mut keyboard);
  keyboard.settle();
  for key in [
    PhysicalKey::ArrowRight,
    PhysicalKey::Enter,
    PhysicalKey::ArrowUp,
    PhysicalKey::ArrowUp,
    PhysicalKey::Enter,
  ] {
    keyboard.key_down(key);
    keyboard.key_up(key);
  }
  wait_for_marker(&mut keyboard, contract::marker::PLAYER_MOVE);
  keyboard.advance_time(Duration::from_millis(300));
  assert!(piece_at(&keyboard, 'f', 2).is_none());
  assert!(piece_at(&keyboard, 'f', 4).is_some());

  let mut controller = client(MemoryPersistence::empty(), Duration::ZERO);
  controller.controller_button_down(0, ControllerButton::South);
  controller.controller_button_up(0, ControllerButton::South);
  wait_for_marker(&mut controller, contract::marker::INITIAL);
  controller.settle();
  controller.controller_navigate(
    0,
    ControllerDirection::Left,
    ControllerNavigationSource::Dpad,
    false,
  );
  controller.controller_button_down(0, ControllerButton::South);
  controller.controller_button_up(0, ControllerButton::South);
  controller.controller_navigate(
    0,
    ControllerDirection::Up,
    ControllerNavigationSource::Dpad,
    false,
  );
  controller.controller_navigate(
    0,
    ControllerDirection::Up,
    ControllerNavigationSource::Dpad,
    false,
  );
  controller.controller_button_down(0, ControllerButton::South);
  controller.controller_button_up(0, ControllerButton::South);
  wait_for_marker(&mut controller, contract::marker::PLAYER_MOVE);
  controller.advance_time(Duration::from_millis(300));
  assert!(piece_at(&controller, 'd', 2).is_none());
  assert!(piece_at(&controller, 'd', 4).is_some());
}

#[test]
fn native_drag_commits_only_legal_board_drops() {
  let mut legal = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  wait_for_marker(&mut legal, contract::marker::RESUMED);
  let pawn = piece(&legal, 'e', 2);
  let input = drag_input(7);
  legal.drag_start(pawn, input);
  wait_for_marker(&mut legal, contract::marker::SELECTED);
  legal.drag_end(pawn, input, square('e', 4));
  wait_for_marker(&mut legal, contract::marker::PLAYER_MOVE);
  assert!(piece_at(&legal, 'e', 2).is_none());
  assert_eq!(
    legal.world().world_point(pawn, Vector3::ZERO),
    square('e', 4)
  );

  let mut illegal = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  wait_for_marker(&mut illegal, contract::marker::RESUMED);
  let pawn = piece(&illegal, 'e', 2);
  let input = drag_input(8);
  illegal.drag_start(pawn, input);
  wait_for_marker(&mut illegal, contract::marker::SELECTED);
  illegal.drag_end(pawn, input, square('e', 5));
  for _ in 0..4 {
    illegal.poll();
  }
  assert_eq!(
    illegal.world().world_point(pawn, Vector3::ZERO),
    square('e', 2)
  );
  assert_eq!(
    illegal.audio_occurrences().last().unwrap().address,
    contract::INVALID_DROP_SOUND
  );

  let mut outside = client(MemoryPersistence::with_save(DEFAULT), Duration::ZERO);
  wait_for_marker(&mut outside, contract::marker::RESUMED);
  let pawn = piece(&outside, 'e', 2);
  let input = drag_input(9);
  outside.drag_start(pawn, input);
  wait_for_marker(&mut outside, contract::marker::SELECTED);
  outside.drag_end(pawn, input, Vector3::new(8.0, 0.0, 0.0));
  for _ in 0..4 {
    outside.poll();
  }
  assert_eq!(
    outside.world().world_point(pawn, Vector3::ZERO),
    square('e', 2)
  );
}

#[test]
fn semantic_piece_activation_keeps_selection_and_captures_the_visible_target() {
  let mut client = client(MemoryPersistence::with_save(CAPTURE), Duration::ZERO);
  wait_for_marker(&mut client, contract::marker::RESUMED);
  crate::support::move_piece(&mut client, ('d', 4), ('e', 5));
  wait_for_marker(&mut client, contract::marker::CAPTURE);
  client.advance_time(Duration::from_millis(300));
  assert!(piece_at(&client, 'd', 4).is_none());
  assert!(piece_at(&client, 'e', 5).is_some());
}
