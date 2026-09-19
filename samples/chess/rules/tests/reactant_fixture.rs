use std::time::Duration;

use battlement::{GameObjectKind, ObjectId, PointerButton, ScreenPosition, Vector3};
use battlement_fake::{
  assets::{FakeAssetCatalog, FakePrefab},
  client::{FakeClient, PointerInput},
};
use battlement_rules::{
  ChessEngine, MUSIC_TRACKS, PIECE_PREFABS, PLAY_BUTTON_ID,
  assets::{self, effects, white},
  audio::SOUND_EFFECTS,
  create_engine_with_position,
  reactant_fixture::{FixtureMove, ReactantChessFixture},
};
use cozy_chess::Square;
use reactant_testing::Display;

const TIMEOUT: Duration = Duration::from_secs(10);

#[test]
fn normal_move_matches_the_legacy_path_and_duration() {
  let fen = "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1";
  let mut legacy = legacy(fen);
  let old_pawn = legacy_piece_at(&legacy, square('e', 2));
  legacy_click_move_without_settle(&mut legacy, old_pawn, square('e', 2), square('e', 4));

  let mut display = fixture(fen);
  let pawn = fixture_piece(&mut display, Square::E2);
  assert_eq!(
    display
      .objects()
      .filter(|object| {
        !object.active_self()
          && matches!(
            object.kind(),
            GameObjectKind::Plane { materials }
              if materials.iter().any(|material| material.address == assets::LEGAL_SQUARE)
          )
      })
      .count(),
    64
  );
  dispatch(&mut display, Square::E2, Square::E4);
  legacy.advance_time(Duration::from_millis(75));
  display.advance_time(Duration::from_millis(75));
  assert_same_position(&display, pawn, &legacy, old_pawn);
  legacy.advance_time(Duration::from_millis(75));
  display.advance_time(Duration::from_millis(75));
  assert_same_position(&display, pawn, &legacy, old_pawn);
  assert_position(&display, pawn, square('e', 3));
  legacy.advance_time(Duration::from_millis(150));
  display.advance_time(Duration::from_millis(150));

  assert_position(&display, pawn, square('e', 4));
  legacy.assert_world_position(old_pawn, square('e', 4), 1e-9);
}

#[test]
fn ordinary_knight_move_preserves_both_legacy_legs() {
  let fen = "4k3/8/8/8/8/2N5/8/4K3 w - - 0 1";
  let mut legacy = legacy(fen);
  let old_knight = legacy_piece_at(&legacy, square('c', 3));
  legacy_click_move_without_settle(&mut legacy, old_knight, square('c', 3), square('d', 5));

  let mut display = fixture(fen);
  let knight = fixture_piece(&mut display, Square::C3);
  dispatch(&mut display, Square::C3, Square::D5);
  legacy.advance_time(Duration::from_millis(200));
  display.advance_time(Duration::from_millis(200));
  assert_same_position(&display, knight, &legacy, old_knight);
  assert_position(&display, knight, square('c', 5));
  legacy.advance_time(Duration::from_millis(60));
  display.advance_time(Duration::from_millis(60));
  assert_same_position(&display, knight, &legacy, old_knight);
  legacy.advance_time(Duration::from_millis(60));
  display.advance_time(Duration::from_millis(60));

  assert_position(&display, knight, square('d', 5));
  legacy.assert_world_position(old_knight, square('d', 5), 1e-9);
}

#[test]
fn normal_capture_matches_legacy_and_retains_the_victim_until_arrival() {
  let fen = "4k3/8/8/4p3/3B4/8/8/4K3 w - - 0 1";
  let mut legacy = legacy(fen);
  let old_bishop = legacy_piece_at(&legacy, square('d', 4));
  let old_victim = legacy_piece_at(&legacy, square('e', 5));
  legacy_drag(&mut legacy, old_bishop, square('d', 4), square('e', 5));

  let mut display = fixture(fen);
  let bishop = fixture_piece(&mut display, Square::D4);
  let victim = fixture_piece(&mut display, Square::E5);
  dispatch(&mut display, Square::D4, Square::E5);
  display.advance_time(Duration::from_millis(150));
  assert_position(&display, bishop, midpoint(square('d', 4), square('e', 5)));
  display.advance_time(Duration::from_millis(149));
  assert!(display.object(victim).is_some());
  display.advance_time(Duration::from_millis(1));

  assert!(display.object(victim).is_none());
  assert_position(&display, bishop, square('e', 5));
  legacy.assert_world_position(old_bishop, square('e', 5), 1e-9);
  assert!(legacy.world().object(old_victim).is_none());
  assert_eq!(display.particle_occurrences().len(), 1);
  assert_eq!(display.particle_occurrences()[0].address, effects::CAPTURE);
}

#[test]
fn knight_capture_keeps_both_legs_and_capture_order() {
  let fen = "4k3/8/8/3p4/8/2N5/8/4K3 w - - 0 1";
  let mut legacy = legacy(fen);
  let old_knight = legacy_piece_at(&legacy, square('c', 3));
  legacy_drag(&mut legacy, old_knight, square('c', 3), square('d', 5));

  let mut display = fixture(fen);
  let knight = fixture_piece(&mut display, Square::C3);
  let victim = fixture_piece(&mut display, Square::D5);
  dispatch(&mut display, Square::C3, Square::D5);
  display.advance_time(Duration::from_millis(200));
  assert_position(&display, knight, square('c', 5));
  display.advance_time(Duration::from_millis(60));
  assert_position(&display, knight, midpoint(square('c', 5), square('d', 5)));
  display.advance_time(Duration::from_millis(59));
  assert!(display.object(victim).is_some());
  display.advance_time(Duration::from_millis(1));

  assert!(display.object(victim).is_none());
  assert_position(&display, knight, square('d', 5));
  legacy.assert_world_position(old_knight, square('d', 5), 1e-9);
}

#[test]
fn castling_moves_both_legacy_identities_in_parallel() {
  let fen = "4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1";
  let mut legacy = legacy(fen);
  let old_king = legacy_piece_at(&legacy, square('e', 1));
  let old_rook = legacy_piece_at(&legacy, square('h', 1));
  legacy_drag(&mut legacy, old_king, square('e', 1), square('g', 1));

  let mut display = fixture(fen);
  let king = fixture_piece(&mut display, Square::E1);
  let rook = fixture_piece(&mut display, Square::H1);
  dispatch(&mut display, Square::E1, Square::G1);
  display.advance_time(Duration::from_millis(150));
  assert_position(&display, king, midpoint(square('e', 1), square('g', 1)));
  assert_position(&display, rook, midpoint(square('h', 1), square('f', 1)));
  display.advance_time(Duration::from_millis(150));

  assert_position(&display, king, square('g', 1));
  assert_position(&display, rook, square('f', 1));
  legacy.assert_world_position(old_king, square('g', 1), 1e-9);
  legacy.assert_world_position(old_rook, square('f', 1), 1e-9);
}

#[test]
fn en_passant_uses_the_legacy_capture_square_and_beat() {
  let fen = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1";
  let mut legacy = legacy(fen);
  let old_pawn = legacy_piece_at(&legacy, square('e', 5));
  let old_victim = legacy_piece_at(&legacy, square('d', 5));
  legacy_drag(&mut legacy, old_pawn, square('e', 5), square('d', 6));

  let mut display = fixture(fen);
  let pawn = fixture_piece(&mut display, Square::E5);
  let victim = fixture_piece(&mut display, Square::D5);
  dispatch(&mut display, Square::E5, Square::D6);
  display.advance_time(Duration::from_millis(299));
  assert!(display.object(victim).is_some());
  display.advance_time(Duration::from_millis(1));

  assert!(display.object(victim).is_none());
  assert_position(&display, pawn, square('d', 6));
  legacy.assert_world_position(old_pawn, square('d', 6), 1e-9);
  assert!(legacy.world().object(old_victim).is_none());
  assert!(matches!(
    display.particle_occurrences()[0].location,
    battlement::ParticleSpawnLocation::WorldPosition(position) if position == square('d', 5)
  ));
}

#[test]
fn promotion_preserves_outer_identity_and_replaces_the_inner_prefab() {
  let fen = "1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1";
  let mut legacy = legacy(fen);
  let old_pawn = legacy_piece_at(&legacy, square('a', 7));
  let old_victim = legacy_piece_at(&legacy, square('b', 8));
  legacy_drag(&mut legacy, old_pawn, square('a', 7), square('b', 8));

  let mut display = fixture(fen);
  let pawn_identity = display
    .with_engine(|fixture| fixture.accepted_state())
    .piece(Square::A7)
    .unwrap()
    .id;
  let pawn = fixture_piece(&mut display, Square::A7);
  let victim = fixture_piece(&mut display, Square::B8);
  dispatch(&mut display, Square::A7, Square::B8);
  display.advance_time(Duration::from_millis(299));
  assert!(display.object(pawn).is_some());
  assert!(display.object(victim).is_some());
  display.advance_time(Duration::from_millis(1));

  assert!(display.object(pawn).is_some());
  assert!(display.object(victim).is_none());
  assert_position(&display, pawn, square('b', 8));
  let state = display.with_engine(|fixture| fixture.accepted_state());
  assert_eq!(state.piece(Square::B8).unwrap().id, pawn_identity);
  assert_eq!(
    state.piece(Square::B8).unwrap().kind,
    cozy_chess::Piece::Queen
  );
  assert!(display.objects().any(|object| {
    object.parent_id() == Some(pawn)
      && matches!(object.kind(), GameObjectKind::Prefab { address, .. } if address == &white::QUEEN)
  }));
  assert!(legacy.world().object(old_pawn).is_none());
  assert!(legacy.world().object(old_victim).is_none());
  assert!(legacy.world().objects().any(|object| {
    object.local_transform().position == square('b', 8)
      && matches!(object.kind(), GameObjectKind::Prefab { address, .. } if address == &white::QUEEN)
  }));
  assert_eq!(display.particle_occurrences().len(), 1);
}

fn fixture(fen: &str) -> Display<ReactantChessFixture> {
  let mut display = Display::connect(
    ReactantChessFixture::from_fen(fen).expect("fixture FEN should be valid"),
    assets(),
  );
  for _ in 0..4 {
    display.poll();
    if display.with_engine(|fixture| fixture.status()) == reactant::GameStatus::Ready {
      return display;
    }
  }
  panic!("initial fixture output did not reach the display");
}

fn dispatch(display: &mut Display<ReactantChessFixture>, from: Square, to: Square) {
  assert!(matches!(
    display.with_engine(|fixture| fixture.dispatch(FixtureMove { from, to })),
    reactant::DispatchResult::Started
  ));
  assert!(display.with_engine(|fixture| fixture.wait_for_worker_stopped(TIMEOUT)));
  for _ in 0..8 {
    display.poll();
    if display.with_engine(|fixture| fixture.status()) == reactant::GameStatus::Ready {
      return;
    }
    assert!(display.with_engine(|fixture| fixture.wait_for_output(TIMEOUT)));
  }
  panic!("fixture output did not reach the display");
}

fn fixture_piece(display: &mut Display<ReactantChessFixture>, square: Square) -> ObjectId {
  let identity = display
    .with_engine(|fixture| fixture.accepted_state())
    .piece(square)
    .expect("fixture piece should exist")
    .id;
  display
    .with_engine(|fixture| fixture.native_piece(identity))
    .expect("fixture piece should have a native host")
}

fn assert_position(display: &Display<ReactantChessFixture>, object: ObjectId, expected: Vector3) {
  let actual = display.world_point(object, Vector3::ZERO);
  assert!((actual.x - expected.x).abs() < 1e-3, "{actual:?}");
  assert!((actual.y - expected.y).abs() < 1e-3, "{actual:?}");
  assert!((actual.z - expected.z).abs() < 1e-3, "{actual:?}");
}

fn assert_same_position(
  display: &Display<ReactantChessFixture>,
  object: ObjectId,
  legacy: &FakeClient<ChessEngine>,
  legacy_object: ObjectId,
) {
  let actual = display.world_point(object, Vector3::ZERO);
  let expected = legacy.world().world_point(legacy_object, Vector3::ZERO);
  assert!(
    (actual.x - expected.x).abs() < 1e-3,
    "{actual:?} != {expected:?}"
  );
  assert!(
    (actual.y - expected.y).abs() < 1e-3,
    "{actual:?} != {expected:?}"
  );
  assert!(
    (actual.z - expected.z).abs() < 1e-3,
    "{actual:?} != {expected:?}"
  );
}

fn legacy(fen: &str) -> FakeClient<ChessEngine> {
  let mut client = FakeClient::connect(
    create_engine_with_position(fen, Duration::from_secs(1)).expect("fixture FEN should be valid"),
    assets(),
  );
  client.click(PLAY_BUTTON_ID);
  client.settle();
  client
}

fn legacy_piece_at(client: &FakeClient<ChessEngine>, position: Vector3) -> ObjectId {
  client
    .world()
    .objects()
    .find(|object| {
      object.local_transform().position == position
        && matches!(object.kind(), GameObjectKind::Prefab { address, .. } if PIECE_PREFABS.contains(address))
    })
    .expect("legacy piece should exist")
    .id()
}

fn legacy_drag(client: &mut FakeClient<ChessEngine>, piece: ObjectId, from: Vector3, to: Vector3) {
  legacy_drag_without_settle(client, piece, from, to);
  client.settle();
}

fn legacy_drag_without_settle(
  client: &mut FakeClient<ChessEngine>,
  piece: ObjectId,
  from: Vector3,
  to: Vector3,
) {
  let pointer = PointerInput {
    pointer_id: 0,
    screen_position: ScreenPosition::new(500.0, 300.0),
    world_hit: from,
    button: PointerButton::Left,
  };
  client.drag_start(piece, pointer);
  client.drag_end(piece, pointer, to);
}

fn legacy_click_move_without_settle(
  client: &mut FakeClient<ChessEngine>,
  piece: ObjectId,
  from: Vector3,
  to: Vector3,
) {
  legacy_drag_without_settle(client, piece, from, from);
  let target = client
    .world()
    .objects()
    .find(|object| {
      matches!(object.kind(), GameObjectKind::Plane { .. })
        && object.local_transform().position.x == to.x
        && object.local_transform().position.z == to.z
    })
    .expect("legacy destination highlight should exist")
    .id();
  client.click(target);
}

fn square(file: char, rank: u8) -> Vector3 {
  Vector3::new(
    f64::from(file as u8 - b'a') - 3.5,
    0.0,
    f64::from(rank) - 4.5,
  )
}

fn midpoint(from: Vector3, to: Vector3) -> Vector3 {
  Vector3::new(
    (from.x + to.x) / 2.0,
    (from.y + to.y) / 2.0,
    (from.z + to.z) / 2.0,
  )
}

fn assets() -> FakeAssetCatalog {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene(assets::CONTENT);
  for address in PIECE_PREFABS {
    catalog.add_prefab(
      address,
      FakePrefab::new()
        .with_material_slots(1)
        .with_pointer_collider(),
    );
  }
  for address in MUSIC_TRACKS {
    catalog.add_audio_clip(address);
  }
  for address in SOUND_EFFECTS {
    catalog.add_audio_clip(address);
  }
  catalog.add_texture(assets::PLAY_BUTTON);
  catalog.add_material(assets::LEGAL_SQUARE);
  catalog.add_texture(assets::REFRESH_BUTTON);
  catalog.add_prefab(
    effects::PIECE_SELECTED,
    FakePrefab::new().with_particle_systems(),
  );
  catalog.add_particle_effect(effects::PIECE_SPAWN);
  catalog.add_particle_effect(effects::CAPTURE);
  catalog
}
