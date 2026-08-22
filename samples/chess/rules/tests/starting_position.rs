use masonry::{DragMode, GameObjectKind, PointerButton, ScreenPosition, Vector3};
use masonry_fake::{
    assets::{FakeAssetCatalog, FakePrefab},
    client::{FakeClient, PointerInput},
};
use masonry_rules::{
    BLACK_KING_PREFAB, CONTENT_SCENE, PIECE_PREFABS, WHITE_QUEEN_PREFAB, create_engine,
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
    assets
}
