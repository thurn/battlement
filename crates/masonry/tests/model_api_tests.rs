use std::any::TypeId;

use masonry::{
    GameObjectKind, GridLayout, ParticleEffectAddress, ParticleSpawnLocation, ParticleSpawnPayload,
    PreparedAsset, SceneAddress, TextureAddress, Vector3,
};

#[test]
fn asset_address_roles_are_distinct_types() {
    let scene = SceneAddress::new("mygame/scenes/board");
    let texture = TextureAddress::new("mygame/textures/card-back");

    assert_ne!(TypeId::of::<SceneAddress>(), TypeId::of::<TextureAddress>());
    assert_eq!(scene.as_str(), "mygame/scenes/board");
    assert_eq!(texture.as_str(), "mygame/textures/card-back");
}

#[test]
fn prepared_asset_couples_its_kind_to_its_address_type() {
    let asset = PreparedAsset::Texture(TextureAddress::new("mygame/textures/card-back"));

    assert!(matches!(asset, PreparedAsset::Texture(_)));
}

#[test]
fn particle_spawn_location_is_an_enum() {
    let payload = ParticleSpawnPayload {
        address: ParticleEffectAddress::new("mygame/effects/dust"),
        location: ParticleSpawnLocation::WorldPosition(Vector3::ZERO),
        lifetime_ms: 800,
    };

    assert_eq!(
        payload.location,
        ParticleSpawnLocation::WorldPosition(Vector3::ZERO)
    );
}

#[test]
fn centered_grid_maps_cells_onto_arbitrary_step_vectors() {
    let grid = GridLayout::centered(
        Vector3::new(10.0, 2.0, -4.0),
        3,
        2,
        Vector3::new(2.0, 0.0, 1.0),
        Vector3::new(0.0, 3.0, 0.0),
    );

    assert_eq!(grid.position(0, 0), Vector3::new(8.0, 0.5, -5.0));
    assert_eq!(grid.position(2, 1), Vector3::new(12.0, 3.5, -3.0));
    assert!(matches!(
        GameObjectKind::prefab("mygame/piece"),
        GameObjectKind::Prefab { .. }
    ));
}
