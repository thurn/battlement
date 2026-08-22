use std::{
    thread,
    time::{Duration, Instant},
};

use masonry::{
    CommandBody, DragMode, GameObjectKind, KeyCode, ObjectId, PointerButton, ScreenPosition,
    Vector3,
};
use masonry_fake::{
    assets::{FakeAssetCatalog, FakePrefab},
    client::{FakeClient, PointerInput},
    time::ManualClock,
};
use masonry_rules::{
    BLACK_KING_PREFAB, CONTENT_SCENE, ChessEngine, MUSIC_TRACKS, PIECE_PREFABS, WHITE_QUEEN_PREFAB,
    create_engine, create_engine_with_clock, create_engine_with_position,
    create_engine_with_think_time,
};

#[test]
fn initial_world_has_a_standard_player_facing_position() {
    let client = FakeClient::connect(
        create_engine().expect("engine should initialize"),
        self::assets(),
    );
    let pieces = client
        .world()
        .objects()
        .filter(|object| matches!(object.kind(), GameObjectKind::Prefab { .. }))
        .collect::<Vec<_>>();

    assert_eq!(pieces.len(), 32);
    assert!(
        pieces[..16]
            .iter()
            .all(|piece| { piece.drag_mode() == Some(DragMode::SnapToPointer) })
    );
    assert!(pieces[16..].iter().all(|piece| piece.drag_mode().is_none()));
    assert_eq!(
        pieces[0].local_transform().position,
        Vector3::new(-3.5, 0.0, -3.5)
    );
    assert_eq!(
        pieces[31].local_transform().position,
        Vector3::new(3.5, 0.0, 3.5)
    );
    assert!(matches!(
        pieces[3].kind(),
        GameObjectKind::Prefab { address, .. } if address.as_str() == WHITE_QUEEN_PREFAB
    ));
    assert!(matches!(
        pieces[28].kind(),
        GameObjectKind::Prefab { address, .. } if address.as_str() == BLACK_KING_PREFAB
    ));
}

#[test]
fn illegal_drag_returns_the_piece_to_its_square() {
    let mut client = FakeClient::connect(
        create_engine().expect("engine should initialize"),
        self::assets(),
    );
    let pawn = self::piece_at(&client, Vector3::new(0.5, 0.0, -2.5));
    let pointer = self::pointer_input(Vector3::new(0.5, 0.0, -2.5));

    client.drag_start(pawn, pointer);
    client.drag_end(pawn, pointer, Vector3::new(0.5, 0.0, 0.5));

    client.assert_world_position(pawn, Vector3::new(0.5, 0.0, -2.5), 1e-9);
    assert!(client.world().input_enabled());
}

#[test]
fn legal_drag_starts_a_nonblocking_ai_turn_and_applies_its_reply() {
    let mut client = FakeClient::connect(
        create_engine_with_think_time(Duration::from_millis(75)),
        self::assets(),
    );
    let pawn = self::piece_at(&client, Vector3::new(0.5, 0.0, -2.5));
    let pointer = self::pointer_input(Vector3::new(0.5, 0.0, -2.5));
    let leftmost_pawn = self::piece_at(&client, self::square('a', 7));
    let submitted_at = Instant::now();

    client.drag_start(pawn, pointer);
    client.drag_end(pawn, pointer, Vector3::new(0.5, 0.0, -0.5));

    assert!(submitted_at.elapsed() < Duration::from_millis(50));
    client.assert_world_position(pawn, Vector3::new(0.5, 0.0, -0.5), 1e-9);
    assert!(!client.world().input_enabled());

    let deadline = Instant::now() + Duration::from_secs(2);
    while !client.world().input_enabled() && Instant::now() < deadline {
        client.poll();
        thread::sleep(Duration::from_millis(5));
    }

    assert!(
        client.world().input_enabled(),
        "AI did not finish before timeout"
    );
    assert!(submitted_at.elapsed() >= Duration::from_millis(70));
    client.assert_world_position(leftmost_pawn, self::square('a', 7), 1e-9);
    assert!(client.world().objects().any(|object| {
        matches!(object.kind(), GameObjectKind::Prefab { address, .. }
            if address.as_str().starts_with("chess/black/"))
            && object.local_transform().position.z < 2.5
    }));
}

#[test]
fn castling_moves_the_king_and_rook_to_their_visible_squares() {
    let mut client = self::positioned_client("4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1");
    let king = self::piece_at(&client, self::square('e', 1));
    let rook = self::piece_at(&client, self::square('h', 1));

    self::drag(
        &mut client,
        king,
        self::square('e', 1),
        self::square('g', 1),
    );

    client.assert_world_position(king, self::square('g', 1), 1e-9);
    client.assert_world_position(rook, self::square('f', 1), 1e-9);
}

#[test]
fn en_passant_removes_the_captured_pawn_from_the_world() {
    let mut client = self::positioned_client("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1");
    let pawn = self::piece_at(&client, self::square('e', 5));
    let captured = self::piece_at(&client, self::square('d', 5));

    self::drag(
        &mut client,
        pawn,
        self::square('e', 5),
        self::square('d', 6),
    );

    client.assert_world_position(pawn, self::square('d', 6), 1e-9);
    assert!(client.world().object(captured).is_none());
}

#[test]
fn promotion_replaces_the_pawn_with_a_draggable_queen() {
    let mut client = self::positioned_client("4k3/P7/8/8/8/8/8/4K3 w - - 0 1");
    let pawn = self::piece_at(&client, self::square('a', 7));

    self::drag(
        &mut client,
        pawn,
        self::square('a', 7),
        self::square('a', 8),
    );

    assert!(client.world().object(pawn).is_none());
    let queen = client
        .world()
        .objects()
        .find(|object| object.local_transform().position == self::square('a', 8))
        .expect("promotion should create a piece on a8");
    assert!(matches!(
        queen.kind(),
        GameObjectKind::Prefab { address, .. } if address.as_str() == WHITE_QUEEN_PREFAB
    ));
    assert_eq!(queen.drag_mode(), Some(DragMode::SnapToPointer));
}

#[test]
fn ai_plays_an_available_checkmate_through_polling() {
    let mut client = FakeClient::connect(
        create_engine_with_position("8/8/8/8/8/5kq1/8/7K b - - 0 1", Duration::from_millis(100))
            .expect("position should be valid"),
        self::assets(),
    );
    let queen = self::piece_at(&client, self::square('g', 3));
    let deadline = Instant::now() + Duration::from_secs(2);

    while client.world().world_transform(queen).position == self::square('g', 3)
        && Instant::now() < deadline
    {
        client.poll();
        thread::sleep(Duration::from_millis(5));
    }

    client.assert_world_position(queen, self::square('g', 2), 1e-9);
    assert!(!client.world().input_enabled());
}

#[test]
fn music_loops_for_two_minutes_then_crossfades_in_playlist_order() {
    let (mut client, clock) = self::clocked_client();

    client.poll();
    assert_eq!(self::played_music(&client), vec![MUSIC_TRACKS[0]]);

    for expected in MUSIC_TRACKS.iter().cycle().skip(1).take(MUSIC_TRACKS.len()) {
        clock.advance(Duration::from_secs(119));
        client.poll();
        assert_ne!(self::played_music(&client).last(), Some(expected));
        clock.advance(Duration::from_secs(1));
        client.poll();
        assert_eq!(self::played_music(&client).last(), Some(expected));
    }
}

#[test]
fn arrow_keys_control_background_music_volume_from_rust() {
    let (mut client, _) = self::clocked_client();
    client.poll();
    let play_id = client
        .commands()
        .iter()
        .find_map(|entry| {
            matches!(entry.command.body, CommandBody::AudioPlay(_))
                .then_some(entry.command.command_id)
        })
        .expect("music should start on the first poll");

    client.key_down(KeyCode::ArrowUp);
    assert!((client.world().audio(play_id).unwrap().volume() - 0.45).abs() < 1e-9);
    client.key_up(KeyCode::ArrowUp);
    for _ in 0..10 {
        client.key_down(KeyCode::ArrowDown);
        client.key_up(KeyCode::ArrowDown);
    }
    assert_eq!(client.world().audio(play_id).unwrap().volume(), 0.0);
}

fn piece_at(client: &FakeClient<ChessEngine>, position: Vector3) -> ObjectId {
    client
        .world()
        .objects()
        .find(|object| object.local_transform().position == position)
        .expect("expected a piece on the requested square")
        .id()
}

fn pointer_input(world_hit: Vector3) -> PointerInput {
    PointerInput {
        pointer_id: 0,
        screen_position: ScreenPosition::new(500.0, 300.0),
        world_hit,
        button: PointerButton::Left,
    }
}

fn positioned_client(fen: &str) -> FakeClient<ChessEngine> {
    FakeClient::connect(
        create_engine_with_position(fen, Duration::from_secs(1)).expect("position should be valid"),
        self::assets(),
    )
}

fn drag(client: &mut FakeClient<ChessEngine>, piece: ObjectId, from: Vector3, to: Vector3) {
    let pointer = self::pointer_input(from);
    client.drag_start(piece, pointer);
    client.drag_end(piece, pointer, to);
}

fn square(file: char, rank: u8) -> Vector3 {
    Vector3::new(
        f64::from(file as u8 - b'a') - 3.5,
        0.0,
        f64::from(rank) - 4.5,
    )
}

fn assets() -> FakeAssetCatalog {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(CONTENT_SCENE);
    for address in PIECE_PREFABS {
        assets.add_prefab(
            address,
            FakePrefab::new()
                .with_material_slots(1)
                .with_pointer_collider(),
        );
    }
    for address in MUSIC_TRACKS {
        assets.add_audio_clip(address);
    }
    assets
}

fn clocked_client() -> (FakeClient<ChessEngine>, ManualClock) {
    FakeClient::connect_clocked(
        |clock| create_engine_with_clock(move || clock.now()),
        self::assets(),
    )
}

fn played_music(client: &FakeClient<ChessEngine>) -> Vec<&str> {
    client
        .commands()
        .iter()
        .filter_map(|entry| match &entry.command.body {
            CommandBody::AudioPlay(play) => Some(play.address.as_str()),
            _ => None,
        })
        .collect()
}
