mod command_coverage;
mod support;

use std::{panic::AssertUnwindSafe, sync::Arc};

use battlement::{
    Action, ActionBody, ActionId, Batch, CameraState, ClientMessage, Command, CommandBody,
    DragMode, DragPayload, GameObject, GameObjectKind, LocalTransform, ObjectId,
    ParallelCommandGroup, PointerEvent, PreparedAsset, Response, ResponseMessage, Scene, SceneId,
    Snapshot, Style, UiDocument, UiFontAddress, UiNode, UnityFontAddress, Vector3,
};
use battlement_fake::{
    assets::{FakeAnimator, FakeAssetCatalog, FakePrefab},
    client::{FakeClient, PointerInput},
    world::WorldTransform,
};
use support::ScriptedEngine;
use uuid::Uuid;

fn session(value: u128) -> battlement::SessionId {
    battlement::SessionId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn action(value: u128) -> ActionId {
    ActionId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn batch_id(value: u128) -> battlement::BatchId {
    battlement::BatchId::from_uuid(Uuid::from_u128(value)).unwrap()
}

fn command_id(value: u128) -> battlement::CommandId {
    battlement::CommandId::from_uuid(Uuid::from_u128(value)).unwrap()
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
            .with_light(battlement::LightState::default())
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

fn snapshot(session_id: battlement::SessionId, objects: Vec<GameObject>) -> Snapshot {
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

#[test]
fn snapshot_can_select_unity_main_camera_without_a_camera_object() {
    let session_id = session(99);
    let snapshot = Snapshot::new_with_main_camera(
        session_id,
        vec![PreparedAsset::Scene("test/scene".into())],
        vec![Scene::new(scene_id(10), "test/scene")],
        Vec::new(),
    );
    let engine = ScriptedEngine::new(
        [Response::new(
            session_id,
            vec![ResponseMessage::Snapshot(snapshot)],
        )],
        [],
        [],
    );

    let client = FakeClient::connect(engine, catalog());

    assert!(client.world().uses_main_camera());
    assert_eq!(client.world().input_camera_id(), None);
}

fn base_response(session_id: battlement::SessionId, objects: Vec<GameObject>) -> Response {
    Response::new(
        session_id,
        vec![ResponseMessage::Snapshot(snapshot(session_id, objects))],
    )
}

fn command(session_id: battlement::SessionId, body: CommandBody, id: u128) -> Response {
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
    assert_eq!(connect.platform, "battlement-fake");
    assert_eq!(connect.unity_version, "battlement-fake");
    assert_eq!(connect.screen.width, 1920);
    assert_eq!(connect.screen.height, 1080);
    assert!(client.world().input_enabled());
    assert_eq!(client.world().input_camera_id(), Some(object_id(1)));
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
            rotation: battlement::Quaternion::IDENTITY,
            scale: Vector3::ONE,
        },
        1e-9,
    );
    assert!(client.assert_object(object_id(3)).active_in_hierarchy());
}

#[test]
fn commands_are_applied_only_by_explicit_poll_and_duplicate_batches_are_ignored() {
    let session_id = session(3);
    let position = CommandBody::TransformSetLocalPosition(battlement::PropertyCommand::canceling(
        battlement::PositionPayload {
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
    let world_hit = Vector3::new(2.0, 3.0, 4.0);
    let mut target = GameObject::new(object_id(2), GameObjectKind::Cube { materials: vec![] });
    target.pointer_events = vec![
        PointerEvent::Enter,
        PointerEvent::Down,
        PointerEvent::Up,
        PointerEvent::Click,
    ];
    let empty = Response::new(session_id, vec![]);
    let expected = [
        ActionBody::PointerEnter(battlement::PointerPayload {
            object_id: object_id(2),
            pointer_id: 0,
            screen_position: battlement::ScreenPosition { x: 960.0, y: 540.0 },
            world_hit,
        }),
        ActionBody::PointerDown(battlement::PointerButtonPayload {
            object_id: object_id(2),
            pointer_id: 0,
            screen_position: battlement::ScreenPosition { x: 960.0, y: 540.0 },
            world_hit,
            button: battlement::PointerButton::Left,
        }),
        ActionBody::PointerUp(battlement::PointerButtonPayload {
            object_id: object_id(2),
            pointer_id: 0,
            screen_position: battlement::ScreenPosition { x: 960.0, y: 540.0 },
            world_hit,
            button: battlement::PointerButton::Left,
        }),
        ActionBody::PointerClick(battlement::PointerButtonPayload {
            object_id: object_id(2),
            pointer_id: 0,
            screen_position: battlement::ScreenPosition { x: 960.0, y: 540.0 },
            world_hit,
            button: battlement::PointerButton::Left,
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

    client.click_at(object_id(2), world_hit);
}

#[test]
fn drag_helpers_emit_world_locations_and_move_the_fake_object() {
    let session_id = session(40);
    let start = Vector3::new(2.0, 0.0, 1.0);
    let end = Vector3::new(-3.0, 0.0, 4.0);
    let target = GameObject::new(object_id(2), GameObjectKind::Cube { materials: vec![] })
        .position(start)
        .draggable(DragMode::PreserveOffset);
    let input = PointerInput {
        pointer_id: 0,
        screen_position: battlement::ScreenPosition { x: 500.0, y: 300.0 },
        world_hit: Vector3::new(2.25, 0.0, 1.0),
        button: battlement::PointerButton::Left,
    };
    let expected = [
        ActionBody::DragStart(DragPayload::new(
            object_id(2),
            0,
            input.screen_position,
            start,
        )),
        ActionBody::DragEnd(DragPayload::new(
            object_id(2),
            0,
            input.screen_position,
            end,
        )),
    ];
    let empty = Response::new(session_id, vec![]);
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

    client.drag_start(object_id(2), input);
    client.drag_end(object_id(2), input, end);

    assert_eq!(client.world().world_transform(object_id(2)).position, end);
}

#[test]
fn reconnect_resets_session_state_but_retains_journal() {
    let first = session(5);
    let second = session(6);
    let body = CommandBody::ObjectSetActive(battlement::ObjectSetActivePayload {
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
        CommandBody::TimeWait(battlement::WaitPayload { duration_ms: 1 }),
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
        let body = CommandBody::ObjectSetActive(battlement::ObjectSetActivePayload {
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
        let body = CommandBody::ObjectReparent(battlement::ObjectReparentPayload {
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
fn dynamic_ui_font_styles_require_the_prepared_catalog_kind() {
    let create_session = session(53);
    let create_document = UiDocument::new(object_id(89));
    let create_root = create_document.root_id;
    let create_snapshot = snapshot(create_session, vec![camera()]).ui_document(create_document);
    let create = CommandBody::VisualElementCreate(Box::new(battlement::VisualElementCreate::new(
        create_root,
        UiNode::new(
            object_id(91),
            battlement::Label::new("create")
                .style(Style::new().unity_font(UnityFontAddress::new("test/unity-font"))),
        ),
    )));
    let mut create_catalog = FakeAssetCatalog::new();
    create_catalog.add_scene("test/scene");
    create_catalog.add_unity_font("test/unity-font");
    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let engine = ScriptedEngine::new(
            [Response::new(
                create_session,
                vec![ResponseMessage::Snapshot(create_snapshot)],
            )],
            [],
            [Some(command(create_session, create, 4))],
        );
        let mut client = FakeClient::connect(engine, Arc::new(create_catalog));
        client.poll();
    }));
    assert!(panic.is_err());

    let update_session = session(54);
    let label_id = object_id(93);
    let update_document = UiDocument::new(object_id(94))
        .child(UiNode::new(label_id, battlement::Label::new("update")));
    let update_snapshot = snapshot(update_session, vec![camera()])
        .prepared_assets([
            PreparedAsset::Scene("test/scene".into()),
            PreparedAsset::UiFont(UiFontAddress::new("test/font")),
        ])
        .ui_document(update_document);
    let update =
        CommandBody::VisualElementUpdate(Box::new(battlement::VisualElementUpdate::Properties {
            object_id: label_id,
            element: Box::new(
                battlement::Label::default()
                    .style(Style::new().unity_font(UnityFontAddress::new("test/font")))
                    .into(),
            ),
        }));
    let mut update_catalog = FakeAssetCatalog::new();
    update_catalog.add_scene("test/scene");
    update_catalog.add_ui_font("test/font");
    let panic = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let engine = ScriptedEngine::new(
            [Response::new(
                update_session,
                vec![ResponseMessage::Snapshot(update_snapshot)],
            )],
            [],
            [Some(command(update_session, update, 5))],
        );
        let mut client = FakeClient::connect(engine, Arc::new(update_catalog));
        client.poll();
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
