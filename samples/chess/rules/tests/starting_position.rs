use std::time::Duration;

use masonry::{
    CommandBody, DragMode, GameObjectKind, KeyCode, PointerButton, ScreenPosition, Vector3,
};
use masonry_fake::{
    assets::{FakeAssetCatalog, FakePrefab},
    client::{FakeClient, PointerInput},
    time::ManualClock,
};
use masonry_rules::{
    BLACK_KING_PREFAB, CONTENT_SCENE, MUSIC_TRACKS, PIECE_PREFABS, WHITE_QUEEN_PREFAB,
    create_engine, create_engine_with_clock,
};

#[test]
fn initial_world_places_all_pieces_on_standard_squares() {
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
        pieces
            .iter()
            .all(|piece| piece.drag_mode() == Some(DragMode::SnapToPointer))
    );
    assert_eq!(
        pieces[0].local_transform().position,
        Vector3::new(-3.5, 0.0, -3.5)
    );
    assert_eq!(
        pieces[7].local_transform().position,
        Vector3::new(3.5, 0.0, -3.5)
    );
    assert_eq!(
        pieces[24].local_transform().position,
        Vector3::new(-3.5, 0.0, 3.5)
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
fn dragging_a_piece_snaps_its_center_to_the_nearest_board_square() {
    let mut client = FakeClient::connect(
        create_engine().expect("engine should initialize"),
        self::assets(),
    );
    let piece_id = client
        .world()
        .objects()
        .find(|object| matches!(object.kind(), GameObjectKind::Prefab { .. }))
        .expect("the board should contain pieces")
        .id();
    let pointer = self::pointer_input();

    client.drag_start(piece_id, pointer);
    client.drag_end(piece_id, pointer, Vector3::new(1.2, 0.0, -0.7));

    client.assert_world_position(piece_id, Vector3::new(1.5, 0.0, -0.5), 1e-9);
}

#[test]
fn dragging_beyond_the_board_snaps_to_an_edge_square() {
    let mut client = FakeClient::connect(
        create_engine().expect("engine should initialize"),
        self::assets(),
    );
    let piece_id = client
        .world()
        .objects()
        .find(|object| matches!(object.kind(), GameObjectKind::Prefab { .. }))
        .expect("the board should contain pieces")
        .id();
    let pointer = self::pointer_input();

    client.drag_start(piece_id, pointer);
    client.drag_end(piece_id, pointer, Vector3::new(20.0, 0.0, -20.0));

    client.assert_world_position(piece_id, Vector3::new(3.5, 0.0, -3.5), 1e-9);
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

fn pointer_input() -> PointerInput {
    PointerInput {
        pointer_id: 0,
        screen_position: ScreenPosition::new(500.0, 300.0),
        world_hit: Vector3::new(-3.5, 0.0, -3.5),
        button: PointerButton::Left,
    }
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

fn clocked_client() -> (FakeClient<masonry_rules::ChessEngine>, ManualClock) {
    FakeClient::connect_clocked(
        |clock| create_engine_with_clock(move || clock.now()),
        self::assets(),
    )
}

fn played_music(client: &FakeClient<masonry_rules::ChessEngine>) -> Vec<&str> {
    client
        .commands()
        .iter()
        .filter_map(|entry| match &entry.command.body {
            CommandBody::AudioPlay(play) => Some(play.address.as_str()),
            _ => None,
        })
        .collect()
}
