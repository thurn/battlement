use battlement::{
    ActionBody, AnimatorState, AnyCommand, Batch, BatchId, CameraClearMode, CameraProjection,
    CameraState, ClientMessage, Color, Command, CommandBody, CommandId, Connect, CustomCommand,
    GameObject, GameObjectKind, ImageState, LightState, LocalTransform, MaterialAssignment,
    ObjectId, ObjectIdPayload, ParallelCommandGroup, ParentScene, PointerEvent, PositionPayload,
    PreparedAsset, PropertyCommand, ReplaceAssetSetPayload, Response, ResponseMessage, Scene,
    SceneAddress, SceneId, SessionId, Snapshot, TextState, Vector3, WaitPayload,
};
use battlement_native::EngineError;
use serde::{Deserialize, Serialize};

const DEFAULT_SCENE: &str = "battlement/tests/default-scene";
const RELEASE_PREFIX: &str = "fixture.release.";
const INTEGRATION_COMMAND: &str = "fixture.integration.scale";
const INTEGRATION_SCENE: &str = "battlement/integration/scene";
const INTEGRATION_PREFAB: &str = "battlement/integration/prefab";
const INTEGRATION_EFFECT: &str = "battlement/integration/effect";
const INTEGRATION_MATERIAL: &str = "battlement/integration/material";
const INTEGRATION_TEXTURE: &str = "battlement/integration/texture";
const INTEGRATION_AUDIO: &str = "battlement/integration/audio";
const INTEGRATION_FONT: &str = "battlement/integration/font";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
/// Tuple payload matching the Unity custom-command fixture formatter.
pub struct FlashPayload(ObjectId, f32);

#[derive(Clone, Copy)]
pub enum ReleaseScenario {
    BatchFailures,
    Timing,
    SnapshotReplacement,
    AssetLifetime,
    CustomFailure,
    PointerInput,
    FatalReconnect,
    IntegrationFixture,
}

impl ReleaseScenario {
    pub fn from_connect(connect: &Connect) -> Option<Self> {
        if connect
            .custom_command_types
            .iter()
            .any(|command_type| command_type == INTEGRATION_COMMAND)
        {
            return Some(Self::IntegrationFixture);
        }
        connect
            .custom_command_types
            .iter()
            .find_map(
                |command_type| match command_type.strip_prefix(RELEASE_PREFIX)? {
                    "batch-failures" => Some(Self::BatchFailures),
                    "timing" => Some(Self::Timing),
                    "snapshot-replacement" => Some(Self::SnapshotReplacement),
                    "asset-lifetime" => Some(Self::AssetLifetime),
                    "custom-failure" => Some(Self::CustomFailure),
                    "pointer-input" => Some(Self::PointerInput),
                    "fatal-reconnect" => Some(Self::FatalReconnect),
                    _ => None,
                },
            )
    }

    pub fn connect_response(self, session_id: SessionId) -> Response<AnyCommand<FlashPayload>> {
        if matches!(self, Self::IntegrationFixture) {
            return integration_connect_response(session_id);
        }
        Response::new(
            session_id,
            vec![ResponseMessage::Snapshot(snapshot(session_id, self))],
        )
    }

    pub fn poll_response(
        self,
        session_id: SessionId,
        connect_count: usize,
        poll_count: usize,
    ) -> Result<Option<Response<AnyCommand<FlashPayload>>>, EngineError> {
        if matches!(self, Self::FatalReconnect) && connect_count == 1 && poll_count == 0 {
            return Err(EngineError::new("fixture transport interruption"));
        }

        Ok(match (self, poll_count) {
            (Self::BatchFailures, 0) => Some(batch_failures(session_id)),
            (Self::BatchFailures, 1) => Some(duplicate_lifetime(session_id)),
            (Self::Timing, 0) => Some(timing(session_id)),
            (Self::SnapshotReplacement, 0) => Some(snapshot_position(session_id, 5.0)),
            (Self::SnapshotReplacement, 1) => Some(self.connect_response(session_id)),
            (Self::SnapshotReplacement, 2) => Some(snapshot_position(session_id, 3.0)),
            (Self::AssetLifetime, 0) => Some(asset_prepare_and_use(session_id)),
            (Self::AssetLifetime, 1) => Some(asset_retire(session_id)),
            (Self::CustomFailure, 0) => Some(custom_failure(session_id)),
            _ => None,
        })
    }

    pub fn submit_response(
        self,
        session_id: SessionId,
        message: ClientMessage<FlashPayload, battlement::CoreErrorCode>,
    ) -> Response<AnyCommand<FlashPayload>> {
        let ClientMessage::Action(action) = message else {
            return Response::new(session_id, Vec::new());
        };
        let ActionBody::PointerClick(payload) = action.body else {
            return Response::new(session_id, Vec::new());
        };
        if !matches!(self, Self::IntegrationFixture) || payload.object_id != object_id(3701) {
            return Response::new(session_id, Vec::new());
        }

        Response::new(
            session_id,
            vec![ResponseMessage::Batch(Batch::new(
                BatchId::new_v4(),
                session_id,
                vec![ParallelCommandGroup::new(vec![set_position_y(3701, 1.25)])],
            ))],
        )
    }
}

pub fn object_id(value: u128) -> ObjectId {
    format!("{value:032x}").parse().unwrap()
}

fn scene_id(value: u128) -> SceneId {
    format!("{value:032x}").parse().unwrap()
}

fn batch_id(value: u128) -> BatchId {
    format!("{value:032x}").parse().unwrap()
}

fn snapshot(session_id: SessionId, scenario: ReleaseScenario) -> Snapshot {
    let mut camera = GameObject::new(
        object_id(1),
        GameObjectKind::Camera {
            camera: CameraState {
                projection: CameraProjection::Orthographic,
                orthographic_size: 3.0,
                ..CameraState::default()
            },
        },
    );
    camera.parent_scene = ParentScene::Persistent;
    camera.local_transform.position = Vector3::new(0.0, 0.0, -10.0);

    let mut objects = vec![camera];
    match scenario {
        ReleaseScenario::BatchFailures => objects.push(empty_object(10, 0.0)),
        ReleaseScenario::SnapshotReplacement => objects.push(empty_object(30, 2.0)),
        ReleaseScenario::CustomFailure => objects.push(empty_object(50, 0.0)),
        ReleaseScenario::PointerInput => {
            objects.push(pointer_cube(60, -1.0));
            objects.push(pointer_cube(61, 1.0));
        }
        ReleaseScenario::IntegrationFixture => unreachable!(),
        _ => {}
    }
    Snapshot::new(
        session_id,
        vec![PreparedAsset::Scene(SceneAddress::new(DEFAULT_SCENE))],
        vec![Scene::new(scene_id(100), DEFAULT_SCENE)],
        objects,
        object_id(1),
    )
}

fn integration_connect_response(session_id: SessionId) -> Response<AnyCommand<FlashPayload>> {
    Response::new(
        session_id,
        vec![ResponseMessage::Snapshot(integration_snapshot(session_id))],
    )
}

fn integration_snapshot(session_id: SessionId) -> Snapshot {
    let scene = scene_id(3790);
    let mut camera = GameObject::new(
        object_id(3700),
        GameObjectKind::Camera {
            camera: CameraState {
                projection: CameraProjection::Orthographic,
                orthographic_size: 4.0,
                clear_mode: CameraClearMode::SolidColor,
                clear_color: Color {
                    r: 0.015,
                    g: 0.025,
                    b: 0.07,
                    a: 1.0,
                },
                ..CameraState::default()
            },
        },
    );
    camera.parent_scene = ParentScene::Persistent;
    camera.local_transform.position = Vector3::new(0.0, 0.0, -10.0);

    let mut light = GameObject::new(
        object_id(3705),
        GameObjectKind::Light {
            light: LightState {
                intensity: 3.0,
                range: 20.0,
                ..LightState::default()
            },
        },
    );
    light.parent_scene = ParentScene::Persistent;
    light.local_transform.position = Vector3::new(0.0, 2.5, -3.0);

    let mut target = GameObject::new(
        object_id(3701),
        GameObjectKind::Prefab {
            address: INTEGRATION_PREFAB.into(),
            materials: vec![MaterialAssignment::new(0, INTEGRATION_MATERIAL)],
            animator: Some(AnimatorState::new("Idle")),
        },
    );
    target.parent_scene = ParentScene::Scene(scene);
    target.pointer_events = vec![
        PointerEvent::Enter,
        PointerEvent::Down,
        PointerEvent::Up,
        PointerEvent::Click,
    ];

    let mut image = GameObject::new(
        object_id(3702),
        GameObjectKind::Image {
            image: ImageState::new(INTEGRATION_TEXTURE, 1.7, 1.7),
        },
    );
    image.parent_scene = ParentScene::Scene(scene);
    image.local_transform.position = Vector3::new(-2.2, 0.0, 0.0);

    let mut text = GameObject::new(
        object_id(3703),
        GameObjectKind::Text {
            text: TextState::new("REAL CONTENT", INTEGRATION_FONT),
        },
    );
    text.parent_scene = ParentScene::Scene(scene);
    text.local_transform.position = Vector3::new(0.0, 2.2, 0.0);

    let mut material_cube = GameObject::new(
        object_id(3704),
        GameObjectKind::Cube {
            materials: vec![MaterialAssignment::new(0, INTEGRATION_MATERIAL)],
        },
    );
    material_cube.parent_scene = ParentScene::Scene(scene);
    material_cube.local_transform.position = Vector3::new(2.2, 0.0, 0.0);

    Snapshot::new(
        session_id,
        vec![
            PreparedAsset::Scene(INTEGRATION_SCENE.into()),
            PreparedAsset::Prefab(INTEGRATION_PREFAB.into()),
            PreparedAsset::ParticleEffect(INTEGRATION_EFFECT.into()),
            PreparedAsset::Material(INTEGRATION_MATERIAL.into()),
            PreparedAsset::Texture(INTEGRATION_TEXTURE.into()),
            PreparedAsset::AudioClip(INTEGRATION_AUDIO.into()),
            PreparedAsset::Font(INTEGRATION_FONT.into()),
        ],
        vec![Scene::new(scene, INTEGRATION_SCENE)],
        vec![camera, light, target, image, text, material_cube],
        object_id(3700),
    )
}

fn empty_object(id: u128, x: f64) -> GameObject {
    let mut object = GameObject::new(object_id(id), GameObjectKind::Empty);
    object.parent_scene = ParentScene::Persistent;
    object.local_transform = LocalTransform {
        position: Vector3::new(x, 0.0, 0.0),
        ..LocalTransform::default()
    };
    object
}

fn pointer_cube(id: u128, x: f64) -> GameObject {
    let mut object = GameObject::new(object_id(id), GameObjectKind::cube());
    object.parent_scene = ParentScene::Persistent;
    object.local_transform.position = Vector3::new(x, 0.0, 0.0);
    object.pointer_events = vec![
        PointerEvent::Enter,
        PointerEvent::Exit,
        PointerEvent::Down,
        PointerEvent::Up,
        PointerEvent::Click,
    ];
    object
}

fn batch_failures(session_id: SessionId) -> Response<AnyCommand<FlashPayload>> {
    let partial = partial_failure(session_id);
    let destroyed = Batch::new(
        BatchId::new_v4(),
        session_id,
        vec![
            ParallelCommandGroup::new(vec![core(CommandBody::ObjectDestroy(ObjectIdPayload {
                object_id: object_id(10),
            }))]),
            ParallelCommandGroup::new(vec![set_position(10, 4.0)]),
        ],
    );
    Response::new(
        session_id,
        vec![
            ResponseMessage::Batch(partial.clone()),
            ResponseMessage::Batch(partial),
            ResponseMessage::Batch(destroyed),
        ],
    )
}

fn duplicate_lifetime(session_id: SessionId) -> Response<AnyCommand<FlashPayload>> {
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(partial_failure(session_id))],
    )
}

fn partial_failure(session_id: SessionId) -> Batch<AnyCommand<FlashPayload>> {
    Batch::new(
        batch_id(200),
        session_id,
        vec![ParallelCommandGroup::new(vec![
            create(empty_object(11, 0.0)),
            set_position(12, 1.0),
            create(empty_object(13, 0.0)),
        ])],
    )
}

fn timing(session_id: SessionId) -> Response<AnyCommand<FlashPayload>> {
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(Batch::new(
            BatchId::new_v4(),
            session_id,
            vec![
                ParallelCommandGroup::new(vec![create(empty_object(20, 0.0))]),
                ParallelCommandGroup::new(vec![
                    core(CommandBody::TimeWait(WaitPayload { duration_ms: 300 })),
                    AnyCommand::Core(
                        Command::new(
                            CommandId::new_v4(),
                            CommandBody::TimeWait(WaitPayload { duration_ms: 800 }),
                        )
                        .nonblocking(),
                    ),
                ]),
                ParallelCommandGroup::new(vec![create(empty_object(21, 0.0))]),
            ],
        ))],
    )
}

fn snapshot_position(session_id: SessionId, position: f64) -> Response<AnyCommand<FlashPayload>> {
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(Batch::new(
            BatchId::new_v4(),
            session_id,
            vec![ParallelCommandGroup::new(vec![set_position(30, position)])],
        ))],
    )
}

fn asset_prepare_and_use(session_id: SessionId) -> Response<AnyCommand<FlashPayload>> {
    let mut prefab = GameObject::new(
        object_id(40),
        GameObjectKind::Prefab {
            address: "fixture/release-prefab".into(),
            materials: Vec::new(),
            animator: None,
        },
    );
    prefab.parent_scene = ParentScene::Persistent;
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(Batch::new(
            BatchId::new_v4(),
            session_id,
            vec![
                ParallelCommandGroup::new(vec![core(CommandBody::AssetsReplaceSet(
                    ReplaceAssetSetPayload {
                        assets: vec![
                            PreparedAsset::Scene(SceneAddress::new(DEFAULT_SCENE)),
                            PreparedAsset::Prefab("fixture/release-prefab".into()),
                        ],
                    },
                ))]),
                ParallelCommandGroup::new(vec![create(prefab)]),
            ],
        ))],
    )
}

fn asset_retire(session_id: SessionId) -> Response<AnyCommand<FlashPayload>> {
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(Batch::new(
            BatchId::new_v4(),
            session_id,
            vec![ParallelCommandGroup::new(vec![core(
                CommandBody::AssetsReplaceSet(ReplaceAssetSetPayload {
                    assets: vec![PreparedAsset::Scene(SceneAddress::new(DEFAULT_SCENE))],
                }),
            )])],
        ))],
    )
}

fn custom_failure(session_id: SessionId) -> Response<AnyCommand<FlashPayload>> {
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(Batch::new(
            BatchId::new_v4(),
            session_id,
            vec![ParallelCommandGroup::new(vec![
                AnyCommand::Custom(CustomCommand::new(
                    CommandId::new_v4(),
                    "fixture.character.flash",
                    FlashPayload(object_id(50), 2.0),
                )),
                create(empty_object(51, 0.0)),
            ])],
        ))],
    )
}

fn create(object: GameObject) -> AnyCommand<FlashPayload> {
    core(CommandBody::object_create(object))
}

fn set_position(id: u128, x: f64) -> AnyCommand<FlashPayload> {
    core(CommandBody::TransformSetLocalPosition(
        PropertyCommand::canceling(PositionPayload {
            object_id: object_id(id),
            position: Vector3::new(x, 0.0, 0.0),
        }),
    ))
}

fn set_position_y(id: u128, y: f64) -> AnyCommand<FlashPayload> {
    core(CommandBody::TransformSetLocalPosition(
        PropertyCommand::canceling(PositionPayload {
            object_id: object_id(id),
            position: Vector3::new(0.0, y, 0.0),
        }),
    ))
}

fn core(body: CommandBody) -> AnyCommand<FlashPayload> {
    AnyCommand::Core(Command::new(CommandId::new_v4(), body))
}
