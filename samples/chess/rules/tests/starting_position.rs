use masonry::{GameObjectKind, Vector3};
use masonry_fake::{
    assets::{FakeAssetCatalog, FakePrefab},
    client::FakeClient,
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

fn assets() -> FakeAssetCatalog {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(CONTENT_SCENE);
    for address in PIECE_PREFABS {
        assets.add_prefab(address, FakePrefab::new().with_material_slots(1));
    }
    assets
}
