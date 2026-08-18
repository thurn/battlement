use masonry::{Command, GameObjectKind, Snapshot, Validate, ValidationError};
use serde_json::{Value, json};

const SESSION_ID: &str = "94fa422b-301d-442d-b9a7-10ea54318e78";
const SCENE_ID: &str = "ca64d87d-33d9-4a19-be6e-597035312d01";
const SECOND_SCENE_ID: &str = "a12e5b12-0fa7-4afb-bc2b-dcb9db398e48";
const CAMERA_ID: &str = "cc847d6e-1468-42c6-9bec-9af5b5aa5c03";
const OBJECT_ID: &str = "e6d89383-f87e-465d-a755-8c1d67bf3874";
const COMMAND_ID: &str = "565e76aa-b480-43c2-900b-1cb9d90e4602";

#[test]
fn valid_snapshot_defaults_pass_cross_field_validation() {
    assert_eq!(snapshot(base_snapshot()).validate(), Ok(()));
}

#[test]
fn snapshot_validation_rejects_representative_cross_field_failures() {
    let mut cases = Vec::new();

    let mut missing_primary = base_snapshot();
    missing_primary["preparedAssets"] = json!([
        { "kind": "scene", "address": "scene/main" },
        { "kind": "scene", "address": "scene/second" }
    ]);
    missing_primary["scenes"] = json!([
        { "sceneId": SCENE_ID, "address": "scene/main" },
        { "sceneId": SECOND_SCENE_ID, "address": "scene/second" }
    ]);
    cases.push((missing_primary, ValidationError::InvalidPrimaryScene));

    let mut duplicate_asset = base_snapshot();
    duplicate_asset["preparedAssets"] = json!([
        { "kind": "scene", "address": "scene/main" },
        { "kind": "font", "address": "scene/main" }
    ]);
    cases.push((duplicate_asset, ValidationError::DuplicatePreparedAddress));

    let mut missing_parent = base_snapshot();
    missing_parent["objects"][0]["parentId"] = json!(OBJECT_ID);
    cases.push((missing_parent, ValidationError::InvalidReference));

    let mut zero_quaternion = base_snapshot();
    zero_quaternion["objects"][0]["localTransform"] = json!({
        "rotation": { "x": 0.0, "y": 0.0, "z": 0.0, "w": 0.0 }
    });
    cases.push((zero_quaternion, ValidationError::ZeroQuaternion));

    let mut invalid_clipping = base_snapshot();
    invalid_clipping["objects"][0]["camera"] = json!({ "near": 10.0, "far": 1.0 });
    cases.push((invalid_clipping, ValidationError::InvalidClipping));

    for (fixture, expected) in cases {
        assert_eq!(snapshot(fixture).validate(), Err(expected));
    }
}

#[test]
fn snapshot_validation_rejects_non_finite_numbers() {
    let mut fixture = snapshot(base_snapshot());
    let GameObjectKind::Camera { camera } = &mut fixture.objects[0].kind else {
        panic!("fixture camera missing");
    };
    camera.near = f64::NAN;

    assert_eq!(fixture.validate(), Err(ValidationError::NonFiniteNumber));
}

#[test]
fn command_validation_rejects_cross_field_and_blocking_failures() {
    let fixtures = [
        (
            json!({
                "commandId": COMMAND_ID,
                "type": "masonry.camera.setClipping",
                "payload": { "objectId": CAMERA_ID, "near": 4.0, "far": 2.0 }
            }),
            ValidationError::InvalidClipping,
        ),
        (
            json!({
                "commandId": COMMAND_ID,
                "type": "masonry.transform.tweenLocalScale",
                "payload": {
                    "objectId": OBJECT_ID,
                    "scale": { "x": 2.0, "y": 2.0, "z": 2.0 },
                    "repeat": { "count": {
                        "additionalTraversals": 1,
                        "mode": "restart"
                    }}
                }
            }),
            ValidationError::InvalidRepeat,
        ),
        (
            json!({
                "commandId": COMMAND_ID,
                "type": "masonry.audio.play",
                "payload": { "address": "audio/loop", "loop": true }
            }),
            ValidationError::InvalidBlocking,
        ),
        (
            json!({
                "commandId": COMMAND_ID,
                "type": "masonry.time.wait",
                "blocking": false,
                "payload": { "durationMs": 10 }
            }),
            ValidationError::InvalidBlocking,
        ),
    ];

    for (fixture, expected) in fixtures {
        let command: Command = serde_json::from_value(fixture).unwrap();
        assert_eq!(command.validate(), Err(expected));
    }
}

fn base_snapshot() -> Value {
    json!({
        "sessionId": SESSION_ID,
        "preparedAssets": [{ "kind": "scene", "address": "scene/main" }],
        "scenes": [{ "sceneId": SCENE_ID, "address": "scene/main" }],
        "objects": [{
            "objectId": CAMERA_ID,
            "kind": "camera",
            "camera": {}
        }],
        "inputCameraId": CAMERA_ID
    })
}

fn snapshot(value: Value) -> Snapshot {
    serde_json::from_value(value).unwrap()
}
