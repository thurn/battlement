use masonry::{GameObjectKind, Vector3};
use masonry_fake::{
    assets::{FakeAssetCatalog, FakePrefab},
    client::FakeClient,
};
use masonry_rules::{CONTENT_SCENE, create_engine};

const PREFABS: [&str; 12] = [
    "chess/white/pawn",
    "chess/white/rook",
    "chess/white/knight",
    "chess/white/bishop",
    "chess/white/queen",
    "chess/white/king",
    "chess/black/pawn",
    "chess/black/rook",
    "chess/black/knight",
    "chess/black/bishop",
    "chess/black/queen",
    "chess/black/king",
];

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
        GameObjectKind::Prefab { address, .. } if address.as_str() == "chess/white/queen"
    ));
    assert!(matches!(
        pieces[28].kind(),
        GameObjectKind::Prefab { address, .. } if address.as_str() == "chess/black/king"
    ));
}

fn assets() -> FakeAssetCatalog {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(CONTENT_SCENE);
    for address in PREFABS {
        assets.add_prefab(address, FakePrefab::new().with_material_slots(1));
    }
    assets
}
