mod command_coverage;
mod support;

use std::{panic::AssertUnwindSafe, sync::Arc};

use masonry::{
    Action, ActionBody, ActionId, Batch, CameraState, ClientMessage, Command, CommandBody,
    GameObject, GameObjectKind, LocalTransform, ObjectId, ParallelCommandGroup, PointerEvent,
    PreparedAsset, Response, ResponseMessage, Scene, SceneId, Snapshot, Vector3,
};
use masonry_fake::{
    assets::{FakeAnimator, FakeAssetCatalog, FakePrefab},
    client::FakeClient,
    world::WorldTransform,
};
use support::ScriptedEngine;
use uuid::Uuid;

fn session(value: u128) -> masonry::SessionId {
    masonry::SessionId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn action(value: u128) -> ActionId {
    ActionId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn batch_id(value: u128) -> masonry::BatchId {
    masonry::BatchId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn command_id(value: u128) -> masonry::CommandId {
    masonry::CommandId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn object_id(value: u128) -> ObjectId {
    ObjectId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn scene_id(value: u128) -> SceneId {
    SceneId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn catalog() -> Arc<FakeAssetCatalog> {
    let mut value = FakeAssetCatalog::new();
    value.add_scene("test/scene");
    value.add_scene("test/scene2");
    value.add_material("test/material");
    value.add_texture("test/texture");
    value.add_font("test/font");
    value.add_audio_clip("test/audio");
    value.add_particle_effect("test/particles");
    value.add_prefab(
        "test/prefab",
        FakePrefab::new()
            .with_material_slots(2)
            .with_camera(CameraState::default())
            .with_light(masonry::LightState::default())
            .with_animator(
                FakeAnimator::new()
                    .with_state(0, "Idle")
                    .with_state(0, "Walk")
                    .with_bool_parameter("running")
                    .with_int_parameter("count")
                    .with_float_parameter("blend")
                    .with_trigger_parameter("fire"),
            )
            .with_particle_systems()
            .with_pointer_collider(),
    );
    Arc::new(value)
}

fn snapshot(session_id: masonry::SessionId, objects: Vec<GameObject>) -> Snapshot {
    Snapshot::new(
        session_id,
        vec![PreparedAsset::Scene("test/scene".into())],
        vec![Scene::new(scene_id(10), "test/scene")],
        objects,
        object_id(1),
    )
}

fn camera() -> GameObject {
    GameObject::new(
        object_id(1),
        GameObjectKind::Camera {
            camera: CameraState::default(),
        },
    )
}

fn base_response(session_id: masonry::SessionId, objects: Vec<GameObject>) -> Response {
    Response::new(
        session_id,
        vec![ResponseMessage::Snapshot(snapshot(session_id, objects))],
    )
}

fn command(session_id: masonry::SessionId, body: CommandBody, id: u128) -> Response {
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(Batch::new(
            batch_id(id + 1000),
            session_id,
            vec![ParallelCommandGroup::new(vec![Command::new(
                command_id(id),
                body,
            )])],
        ))],
    )
}

#[test]
fn default_connect_and_snapshot_are_observable() {
    let session_id = session(1);
    let engine = ScriptedEngine::new([base_response(session_id, vec![camera()])], [], []);
    let probe = engine.probe.clone();
    let client = FakeClient::connect(engine, catalog());

    let connect = &probe.borrow().connects[0];
    assert_eq!(connect.platform, "masonry-fake");
    assert_eq!(connect.unity_version, "masonry-fake");
    assert_eq!(connect.screen.width, 1920);
    assert_eq!(connect.screen.height, 1080);
    assert!(client.world().input_enabled());
    assert_eq!(client.world().input_camera_id(), object_id(1));
    assert_eq!(client.world().primary_scene_id(), scene_id(10));
}

#[test]
fn hierarchy_transforms_and_active_state_match_contract() {
    let session_id = session(2);
    let parent = GameObject {
        local_transform: LocalTransform {
            position: Vector3::new(10.0, 0.0, 0.0),
            ..LocalTransform::default()
        },
        ..GameObject::new(object_id(2), GameObjectKind::Empty)
    };
    let child = GameObject {
        parent_id: Some(object_id(2)),
        local_transform: LocalTransform {
            position: Vector3::new(2.0, 3.0, 4.0),
            ..LocalTransform::default()
        },
        ..GameObject::new(object_id(3), GameObjectKind::Cube { materials: vec![] })
    };
    let engine = ScriptedEngine::new(
        [base_response(session_id, vec![camera(), parent, child])],
        [],
        [],
    );
    let client = FakeClient::connect(engine, catalog());

    assert_eq!(client.world().children(object_id(2)).unwrap().count(), 1);
    client.assert_world_transform(
        object_id(3),
        WorldTransform {
            position: Vector3::new(12.0, 3.0, 4.0),
            rotation: masonry::Quaternion::IDENTITY,
            scale: Vector3::ONE,
        },
        1e-9,
    );
    assert!(client.assert_object(object_id(3)).active_in_hierarchy());
}

#[test]
fn commands_are_applied_only_by_explicit_poll_and_duplicate_batches_are_ignored() {
    let session_id = session(3);
    let position = CommandBody::TransformSetLocalPosition(masonry::PropertyCommand::canceling(
        masonry::PositionPayload {
            object_id: object_id(2),
            position: Vector3::new(4.0, 5.0, 6.0),
        },
    ));
    let response = command(session_id, position, 20);
    let engine = ScriptedEngine::new(
        [base_response(
            session_id,
            vec![
                camera(),
                GameObject::new(object_id(2), GameObjectKind::Empty),
            ],
        )],
        [],
        [Some(response.clone()), Some(response)],
    );
    let mut client = FakeClient::connect(engine, catalog());

    client.assert_local_transform(object_id(2), LocalTransform::default(), 0.0);
    client.poll();
    client.assert_local_transform(
        object_id(2),
        LocalTransform {
            position: Vector3::new(4.0, 5.0, 6.0),
            ..LocalTransform::default()
        },
        0.0,
    );
    assert_eq!(client.commands().len(), 1);
    client.poll();
    assert_eq!(client.commands().len(), 1);
}

#[test]
fn input_emits_exact_pointer_order_and_deterministic_ids() {
    let session_id = session(4);
    let mut target = GameObject::new(object_id(2), GameObjectKind::Cube { materials: vec![] });
    target.pointer_events = vec![
        PointerEvent::Enter,
        PointerEvent::Down,
        PointerEvent::Up,
        PointerEvent::Click,
    ];
    let empty = Response::new(session_id, vec![]);
    let expected = [
        ActionBody::PointerEnter(masonry::PointerPayload {
            object_id: object_id(2),
            pointer_id: 0,
            screen_position: masonry::ScreenPosition { x: 960.0, y: 540.0 },
            world_hit: Vector3::ZERO,
        }),
        ActionBody::PointerDown(masonry::PointerButtonPayload {
            object_id: object_id(2),
            pointer_id: 0,
            screen_position: masonry::ScreenPosition { x: 960.0, y: 540.0 },
            world_hit: Vector3::ZERO,
            button: masonry::PointerButton::Left,
        }),
        ActionBody::PointerUp(masonry::PointerButtonPayload {
            object_id: object_id(2),
            pointer_id: 0,
            screen_position: masonry::ScreenPosition { x: 960.0, y: 540.0 },
            world_hit: Vector3::ZERO,
            button: masonry::PointerButton::Left,
        }),
        ActionBody::PointerClick(masonry::PointerButtonPayload {
            object_id: object_id(2),
            pointer_id: 0,
            screen_position: masonry::ScreenPosition { x: 960.0, y: 540.0 },
            world_hit: Vector3::ZERO,
            button: masonry::PointerButton::Left,
        }),
    ];
    let submits = expected.iter().enumerate().map(|(index, body)| {
        (
            ClientMessage::Action(Action::new(
                action(index as u128 + 1),
                session_id,
                body.clone(),
            )),
            empty.clone(),
        )
    });
    let engine = ScriptedEngine::new(
        [base_response(session_id, vec![camera(), target])],
        submits,
        [],
    );
    let mut client = FakeClient::connect(engine, catalog());

    client.click(object_id(2));
}

#[test]
fn reconnect_resets_session_state_but_retains_journal() {
    let first = session(5);
    let second = session(6);
    let body = CommandBody::ObjectSetActive(masonry::ObjectSetActivePayload {
        object_id: object_id(2),
        active: false,
    });
    let first_command = command(first, body.clone(), 30);
    let second_command = command(second, body, 30);
    let engine = ScriptedEngine::new(
        [
            base_response(
                first,
                vec![
                    camera(),
                    GameObject::new(object_id(2), GameObjectKind::Empty),
                ],
            ),
            base_response(
                second,
                vec![
                    camera(),
                    GameObject::new(object_id(2), GameObjectKind::Empty),
                ],
            ),
        ],
        [],
        [Some(first_command), Some(second_command)],
    );
    let mut client = FakeClient::connect(engine, catalog());
    client.poll();
    assert!(!client.assert_object(object_id(2)).active_self());
    client.reconnect();
    assert!(client.assert_object(object_id(2)).active_self());
    client.poll();
    assert_eq!(client.commands().len(), 2);
    assert_ne!(
        client.commands()[0].session_id,
        client.commands()[1].session_id
    );
}

#[test]
fn representative_invalid_inputs_panic_at_the_fake_boundary() {
    let initial_session = session(51);
    let empty_response = Response::new(initial_session, vec![]);
    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let engine = ScriptedEngine::new([empty_response], [], []);
        let _ = FakeClient::connect(engine, catalog());
    }));
    assert!(panic.is_err());

    let duplicate_object = GameObject::new(object_id(2), GameObjectKind::Empty);
    let invalid_snapshot = Snapshot::new(
        initial_session,
        vec![PreparedAsset::Scene("test/scene".into())],
        vec![Scene::new(scene_id(10), "test/scene")],
        vec![camera(), duplicate_object.clone(), duplicate_object],
        object_id(1),
    );
    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let engine = ScriptedEngine::new(
            [Response::new(
                initial_session,
                vec![ResponseMessage::Snapshot(invalid_snapshot)],
            )],
            [],
            [],
        );
        let _ = FakeClient::connect(engine, catalog());
    }));
    assert!(panic.is_err());

    let invalid_catalog_snapshot = Snapshot::new(
        initial_session,
        vec![PreparedAsset::Scene("test/material".into())],
        vec![Scene::new(scene_id(10), "test/material")],
        vec![camera()],
        object_id(1),
    );
    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let engine = ScriptedEngine::new(
            [Response::new(
                initial_session,
                vec![ResponseMessage::Snapshot(invalid_catalog_snapshot)],
            )],
            [],
            [],
        );
        let _ = FakeClient::connect(engine, catalog());
    }));
    assert!(panic.is_err());

    let command_session = session(52);
    let invalid_command = Command::new(
        command_id(1),
        CommandBody::TimeWait(masonry::WaitPayload { duration_ms: 1 }),
    )
    .nonblocking();
    let invalid_command_response = Response::new(
        command_session,
        vec![ResponseMessage::Batch(Batch::new(
            batch_id(1),
            command_session,
            vec![ParallelCommandGroup::new(vec![invalid_command])],
        ))],
    );
    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let engine = ScriptedEngine::new(
            [base_response(
                command_session,
                vec![
                    camera(),
                    GameObject::new(object_id(2), GameObjectKind::Empty),
                ],
            )],
            [],
            [Some(invalid_command_response)],
        );
        let mut client = FakeClient::connect(engine, catalog());
        client.poll();
    }));
    assert!(panic.is_err());

    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let body = CommandBody::ObjectSetActive(masonry::ObjectSetActivePayload {
            object_id: object_id(99),
            active: false,
        });
        let engine = ScriptedEngine::new(
            [base_response(
                command_session,
                vec![
                    camera(),
                    GameObject::new(object_id(2), GameObjectKind::Empty),
                ],
            )],
            [],
            [Some(command(command_session, body, 2))],
        );
        let mut client = FakeClient::connect(engine, catalog());
        client.poll();
    }));
    assert!(panic.is_err());

    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let body = CommandBody::ObjectReparent(masonry::ObjectReparentPayload {
            object_id: object_id(2),
            parent_id: Some(object_id(2)),
            world_position_stays: false,
        });
        let engine = ScriptedEngine::new(
            [base_response(
                command_session,
                vec![
                    camera(),
                    GameObject::new(object_id(2), GameObjectKind::Empty),
                ],
            )],
            [],
            [Some(command(command_session, body, 3))],
        );
        let mut client = FakeClient::connect(engine, catalog());
        client.poll();
    }));
    assert!(panic.is_err());

    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let engine = ScriptedEngine::new(
            [base_response(
                command_session,
                vec![
                    camera(),
                    GameObject::new(object_id(2), GameObjectKind::Empty),
                ],
            )],
            [],
            [],
        );
        let mut client = FakeClient::connect(engine, catalog());
        client.click(object_id(2));
    }));
    assert!(panic.is_err());
}

#[test]
fn assertion_helpers_report_missing_objects_and_world_transform() {
    let session_id = session(7);
    let engine = ScriptedEngine::new([base_response(session_id, vec![camera()])], [], []);
    let client = FakeClient::connect(engine, catalog());
    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        client.assert_object_absent(object_id(1))
    }));
    assert!(panic.is_err());
    let unknown = std::panic::catch_unwind(AssertUnwindSafe(|| {
        client.world().world_transform(object_id(99))
    }));
    assert!(unknown.is_err());
}
