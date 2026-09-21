mod support;

use std::time::Duration;

use battlement::{GameObjectKind, Vector3};
use battlement_rules::{Engine, contract};

use crate::support::{
  AI_TURN, CAPTURE, CASTLE, CHECK, COMPUTER_WIN, DRAW, EN_PASSANT, KNIGHT_CAPTURE,
  MemoryPersistence, PLAYER_WIN, PROMOTION, client, find_ui, move_piece, piece, piece_at, square,
  wait_for_marker,
};

#[test]
fn captures_and_castling_publish_visible_motion_and_effects() {
  let mut capture = client(MemoryPersistence::with_save(CAPTURE), Duration::ZERO);
  wait_for_marker(&mut capture, contract::marker::RESUMED);
  let bishop = piece(&capture, 'd', 4);
  move_piece(&mut capture, ('d', 4), ('e', 5));
  wait_for_marker(&mut capture, contract::marker::CAPTURE);
  capture.advance_time(Duration::from_millis(299));
  assert_ne!(
    capture.world().world_point(bishop, Vector3::ZERO),
    square('e', 5)
  );
  assert!(
    !capture
      .audio_occurrences()
      .iter()
      .any(|effect| { contract::CAPTURE_SOUNDS.contains(&effect.address) })
  );
  capture.advance_time(Duration::from_millis(1));
  assert_eq!(
    capture.world().world_point(bishop, Vector3::ZERO),
    square('e', 5)
  );
  assert!(
    capture
      .audio_occurrences()
      .iter()
      .any(|effect| { contract::CAPTURE_SOUNDS.contains(&effect.address) })
  );
  assert_eq!(
    capture.particle_occurrences()[0].address,
    contract::CAPTURE_EFFECT
  );

  let mut knight = client(MemoryPersistence::with_save(KNIGHT_CAPTURE), Duration::ZERO);
  wait_for_marker(&mut knight, contract::marker::RESUMED);
  move_piece(&mut knight, ('d', 4), ('e', 6));
  wait_for_marker(&mut knight, contract::marker::CAPTURE);
  knight.advance_time(Duration::from_millis(300));
  assert!(
    !knight
      .audio_occurrences()
      .iter()
      .any(|effect| { contract::CAPTURE_SOUNDS.contains(&effect.address) })
  );
  knight.advance_time(Duration::from_millis(20));
  assert!(
    knight
      .audio_occurrences()
      .iter()
      .any(|effect| { contract::CAPTURE_SOUNDS.contains(&effect.address) })
  );

  let mut castle = client(MemoryPersistence::with_save(CASTLE), Duration::ZERO);
  wait_for_marker(&mut castle, contract::marker::RESUMED);
  move_piece(&mut castle, ('e', 1), ('g', 1));
  wait_for_marker(&mut castle, contract::marker::CASTLE);
  castle.advance_time(Duration::from_millis(300));
  assert!(piece_at(&castle, 'g', 1).is_some());
  assert!(piece_at(&castle, 'f', 1).is_some());
  assert!(
    castle
      .audio_occurrences()
      .iter()
      .any(|effect| { effect.address == contract::CASTLE_SOUND })
  );
}

#[test]
fn en_passant_and_promotion_replace_the_visible_piece_tree() {
  let mut en_passant = client(MemoryPersistence::with_save(EN_PASSANT), Duration::ZERO);
  wait_for_marker(&mut en_passant, contract::marker::RESUMED);
  let pawn = piece(&en_passant, 'e', 5);
  let victim = piece(&en_passant, 'd', 5);
  move_piece(&mut en_passant, ('e', 5), ('d', 6));
  wait_for_marker(&mut en_passant, contract::marker::EN_PASSANT);
  en_passant.advance_time(Duration::from_millis(320));
  for _ in 0..4 {
    en_passant.poll();
  }
  assert!(en_passant.world().object(victim).is_none());
  assert_eq!(
    en_passant.world().world_point(pawn, Vector3::ZERO),
    square('d', 6)
  );

  let mut promotion = client(MemoryPersistence::with_save(PROMOTION), Duration::ZERO);
  wait_for_marker(&mut promotion, contract::marker::RESUMED);
  let pawn = piece(&promotion, 'a', 7);
  let victim = piece(&promotion, 'b', 8);
  move_piece(&mut promotion, ('a', 7), ('b', 8));
  for _ in 0..10_000 {
    promotion.poll();
    if find_ui(&promotion, "promote-knight").is_some() {
      break;
    }
    std::thread::yield_now();
  }
  let knight = find_ui(&promotion, "promote-knight").expect("promotion choice is presented");
  promotion.ui().click(knight);
  wait_for_marker(&mut promotion, contract::marker::PROMOTION);
  promotion.advance_time(Duration::from_millis(320));
  for _ in 0..4 {
    promotion.poll();
  }
  assert!(promotion.world().object(victim).is_none());
  assert!(promotion.world().object(pawn).is_some());
  assert_eq!(
    promotion.world().world_point(pawn, Vector3::ZERO),
    square('b', 8)
  );
  assert!(promotion.world().objects().any(|object| {
    object.parent_id() == Some(pawn)
      && matches!(
        object.kind(),
        GameObjectKind::Prefab { address, .. } if *address == contract::WHITE_KNIGHT
      )
  }));
}

#[test]
fn saved_positions_reach_check_draw_and_terminal_markers() {
  let mut check = client(MemoryPersistence::with_save(CHECK), Duration::from_secs(1));
  wait_for_marker(&mut check, contract::marker::RESUMED);
  move_piece(&mut check, ('a', 2), ('a', 8));
  wait_for_marker(&mut check, contract::marker::CHECK);

  let mut player_win = client(MemoryPersistence::with_save(PLAYER_WIN), Duration::ZERO);
  wait_for_marker(&mut player_win, contract::marker::RESUMED);
  move_piece(&mut player_win, ('g', 6), ('g', 7));
  wait_for_marker(&mut player_win, contract::marker::PLAYER_WIN);

  let mut draw = client(MemoryPersistence::with_save(DRAW), Duration::ZERO);
  wait_for_marker(&mut draw, contract::marker::RESUMED);
  move_piece(&mut draw, ('c', 7), ('b', 6));
  wait_for_marker(&mut draw, contract::marker::DRAW);

  let mut computer_win = client(MemoryPersistence::with_save(COMPUTER_WIN), Duration::ZERO);
  wait_for_marker(&mut computer_win, contract::marker::COMPUTER_WIN);
}

#[test]
fn seeded_zero_budget_ai_produces_the_same_unity_position() {
  let mut first = client(MemoryPersistence::with_save(AI_TURN), Duration::ZERO);
  wait_for_marker(&mut first, contract::marker::AI_RESPONSE);
  let first_position = occupied_squares(&first);

  let mut second = client(MemoryPersistence::with_save(AI_TURN), Duration::ZERO);
  wait_for_marker(&mut second, contract::marker::AI_RESPONSE);
  assert_eq!(occupied_squares(&second), first_position);
}

fn occupied_squares<E: Engine>(client: &battlement_fake::client::FakeClient<E>) -> Vec<(i16, i16)> {
  let mut positions = client
    .world()
    .objects()
    .filter(|object| matches!(object.kind(), GameObjectKind::BoxHitRegion { .. }))
    .map(|object| {
      let position = client.world().world_point(object.id(), Vector3::ZERO);
      ((position.x * 2.0) as i16, (position.z * 2.0) as i16)
    })
    .collect::<Vec<_>>();
  positions.sort_unstable();
  positions
}
