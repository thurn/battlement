mod support;

use std::time::Duration;

use battlement::{GameObjectKind, Vector3};
use chess_rules::{ChessGame, contract};
use reactant_testing::{Display, GameActionResult};

use crate::support::{
  AI_TURN, CAPTURE, CASTLE, CHECK, COMPUTER_WIN, DRAW, EN_PASSANT, KNIGHT_CAPTURE,
  MemoryPersistence, PLAYER_WIN, PROMOTION, TIMEOUT, assert_state, client, piece, piece_at,
  play_move, request_move, square,
};

#[test]
fn captures_and_castling_publish_visible_motion_and_effects() {
  let mut capture = client(MemoryPersistence::with_save(CAPTURE), Duration::ZERO);
  let bishop = piece(&capture, 'd', 4);
  play_move(&mut capture, ('d', 4), ('e', 5));
  assert_eq!(capture.world_point(bishop, Vector3::ZERO), square('e', 5));
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
  play_move(&mut knight, ('d', 4), ('e', 6));
  assert!(
    knight
      .audio_occurrences()
      .iter()
      .any(|effect| { contract::CAPTURE_SOUNDS.contains(&effect.address) })
  );

  let mut castle = client(MemoryPersistence::with_save(CASTLE), Duration::ZERO);
  play_move(&mut castle, ('e', 1), ('g', 1));
  assert!(piece_at(&castle, 'g', 1).is_some());
  assert!(piece_at(&castle, 'f', 1).is_some());
  assert!(
    castle
      .audio_occurrences()
      .iter()
      .any(|effect| effect.address == contract::CASTLE_SOUND)
  );
}

#[test]
fn en_passant_and_promotion_replace_the_visible_piece_tree() {
  let mut en_passant = client(MemoryPersistence::with_save(EN_PASSANT), Duration::ZERO);
  let pawn = piece(&en_passant, 'e', 5);
  let victim = piece(&en_passant, 'd', 5);
  play_move(&mut en_passant, ('e', 5), ('d', 6));
  assert!(en_passant.object(victim).is_none());
  assert_eq!(en_passant.world_point(pawn, Vector3::ZERO), square('d', 6));

  let mut promotion = client(MemoryPersistence::with_save(PROMOTION), Duration::ZERO);
  let pawn = piece(&promotion, 'a', 7);
  let victim = piece(&promotion, 'b', 8);
  assert_eq!(
    request_move(&mut promotion, ('a', 7), ('b', 8)),
    GameActionResult::AwaitingInput
  );
  let knight = promotion.find_ui(contract::ROOT_ID, "promote-knight");
  assert_eq!(
    promotion.game_action_presented::<ChessGame>(
      TIMEOUT,
      |display| display.click_ui(knight),
      |display| display.world_point(pawn, Vector3::ZERO) == square('b', 8),
    ),
    GameActionResult::Completed
  );
  assert!(promotion.object(victim).is_none());
  assert!(promotion.object(pawn).is_some());
  assert_eq!(promotion.world_point(pawn, Vector3::ZERO), square('b', 8));
  assert!(promotion.objects().any(|object| {
    object.parent_id() == Some(pawn)
      && matches!(
        object.kind(),
        GameObjectKind::Prefab { address, .. } if *address == contract::WHITE_KNIGHT
      )
  }));
}

#[test]
fn saved_positions_reach_check_draw_and_terminal_states() {
  let mut check = client(MemoryPersistence::with_save(CHECK), Duration::from_secs(1));
  play_move(&mut check, ('a', 2), ('a', 8));
  assert_state(&check, contract::marker::CHECK);

  let mut player_win = client(MemoryPersistence::with_save(PLAYER_WIN), Duration::ZERO);
  play_move(&mut player_win, ('g', 6), ('g', 7));
  assert_state(&player_win, contract::marker::PLAYER_WIN);

  let mut draw = client(MemoryPersistence::with_save(DRAW), Duration::ZERO);
  play_move(&mut draw, ('c', 7), ('b', 6));
  assert_state(&draw, contract::marker::DRAW);

  let computer_win = client(MemoryPersistence::with_save(COMPUTER_WIN), Duration::ZERO);
  assert_state(&computer_win, contract::marker::COMPUTER_WIN);
}

#[test]
fn seeded_zero_budget_ai_produces_the_same_unity_position() {
  let first = client(MemoryPersistence::with_save(AI_TURN), Duration::ZERO);
  assert_state(&first, contract::marker::AI_RESPONSE);
  let first_position = occupied_squares(&first);

  let second = client(MemoryPersistence::with_save(AI_TURN), Duration::ZERO);
  assert_state(&second, contract::marker::AI_RESPONSE);
  assert_eq!(occupied_squares(&second), first_position);
}

fn occupied_squares(display: &Display) -> Vec<(i16, i16)> {
  let mut positions = display
    .objects()
    .filter(|object| matches!(object.kind(), GameObjectKind::BoxHitRegion { .. }))
    .map(|object| {
      let position = display.world_point(object.id(), Vector3::ZERO);
      ((position.x * 2.0) as i16, (position.z * 2.0) as i16)
    })
    .collect::<Vec<_>>();
  positions.sort_unstable();
  positions
}
