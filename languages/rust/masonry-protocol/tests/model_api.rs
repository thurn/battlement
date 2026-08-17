use std::any::TypeId;

use masonry_protocol::{
    CameraState, GameObject, GameObjectKind, ParentScene, ParticleEffectAddress,
    ParticleSpawnLocation, ParticleSpawnPayload, PreparedAsset, SceneAddress, TextureAddress,
    Tween, TweenRepeat,
};
use schemars::{JsonSchema, generate::SchemaSettings};
use serde_json::json;

#[test]
fn asset_address_roles_are_distinct_types_with_the_same_wire_shape() {
    let scene = SceneAddress::new("mygame/scenes/board");
    let texture = TextureAddress::new("mygame/textures/card-back");

    assert_ne!(TypeId::of::<SceneAddress>(), TypeId::of::<TextureAddress>());
    assert_eq!(SceneAddress::schema_name(), "SceneAddress");
    assert_eq!(TextureAddress::schema_name(), "TextureAddress");
    assert_eq!(
        serde_json::to_value(scene).unwrap(),
        json!("mygame/scenes/board")
    );
    assert_eq!(
        serde_json::to_value(texture).unwrap(),
        json!("mygame/textures/card-back")
    );
}

#[test]
fn prepared_asset_couples_its_kind_to_its_address_type() {
    let asset = PreparedAsset::Texture(TextureAddress::new("mygame/textures/card-back"));

    assert_eq!(
        serde_json::to_value(asset).unwrap(),
        json!({
            "kind": "texture",
            "address": "mygame/textures/card-back"
        })
    );
}

#[test]
fn placement_spawn_location_and_repetition_are_tagged_unions() {
    let scene_id = "ca64d87d-33d9-4a19-be6e-597035312d01".parse().unwrap();
    let object_id = "cc847d6e-1468-42c6-9bec-9af5b5aa5c03".parse().unwrap();

    assert_eq!(
        serde_json::to_value(ParentScene::Scene(scene_id)).unwrap(),
        json!({ "scene": "ca64d87d-33d9-4a19-be6e-597035312d01" })
    );
    assert_eq!(
        serde_json::to_value(ParticleSpawnLocation::GameObject(object_id)).unwrap(),
        json!({ "gameObject": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03" })
    );
    assert_eq!(
        serde_json::to_value(TweenRepeat::Once).unwrap(),
        json!("once")
    );
}

#[test]
fn generated_schemas_expose_union_branches_with_camel_case_fields() {
    for schema in [
        draft_7_schema::<ParentScene>(),
        draft_7_schema::<ParticleSpawnLocation>(),
        draft_7_schema::<TweenRepeat>(),
    ] {
        assert!(schema.contains("\"oneOf\""));
    }

    assert!(draft_7_schema::<ParentScene>().contains("\"scene\""));
    assert!(draft_7_schema::<ParticleSpawnLocation>().contains("\"gameObject\""));
    assert!(draft_7_schema::<TweenRepeat>().contains("\"additionalTraversals\""));
}

#[test]
fn default_values_stay_omitted_from_serialized_records() {
    let object = GameObject::new(
        "cc847d6e-1468-42c6-9bec-9af5b5aa5c03".parse().unwrap(),
        GameObjectKind::Empty,
    );

    assert_eq!(serde_json::to_value(Tween::default()).unwrap(), json!({}));
    assert_eq!(
        serde_json::to_value(CameraState::default()).unwrap(),
        json!({})
    );
    assert_eq!(
        serde_json::to_value(object).unwrap(),
        json!({
            "objectId": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
            "kind": "empty"
        })
    );
}

#[test]
fn particle_location_union_rejects_multiple_branches() {
    let payload = ParticleSpawnPayload {
        address: ParticleEffectAddress::new("mygame/effects/dust"),
        location: ParticleSpawnLocation::GameObject(
            "cc847d6e-1468-42c6-9bec-9af5b5aa5c03".parse().unwrap(),
        ),
        lifetime_ms: 800,
    };

    assert_eq!(
        serde_json::to_value(payload).unwrap(),
        json!({
            "address": "mygame/effects/dust",
            "location": {
                "gameObject": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03"
            },
            "lifetimeMs": 800
        })
    );

    assert!(
        serde_json::from_value::<ParticleSpawnPayload>(json!({
            "address": "mygame/effects/dust",
            "location": {
                "gameObject": "cc847d6e-1468-42c6-9bec-9af5b5aa5c03",
                "worldPosition": { "x": 0.0, "y": 0.0, "z": 0.0 }
            },
            "lifetimeMs": 800
        }))
        .is_err()
    );

    let schema = draft_7_schema::<ParticleSpawnPayload>();
    assert!(schema.contains("\"oneOf\""));
    assert!(schema.contains("\"gameObject\""));
    assert!(schema.contains("\"worldPosition\""));
}

fn draft_7_schema<T: JsonSchema>() -> String {
    serde_json::to_string(
        &SchemaSettings::draft07()
            .into_generator()
            .into_root_schema_for::<T>(),
    )
    .unwrap()
}
