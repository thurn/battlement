use masonry::{
    AnyCommand, Batch, BatchId, CameraProjection, CameraState, Command, CommandBody, CommandId,
    Connect, CustomCommand, GameObject, GameObjectKind, LocalTransform, ObjectId, ObjectIdPayload,
    ParallelCommandGroup, ParentScene, PointerEvent, PositionPayload, PreparedAsset,
    PropertyCommand, ReplaceAssetSetPayload, Response, ResponseMessage, Scene, SceneAddress,
    SceneId, SessionId, Snapshot, Vector3, WaitPayload,
};
use masonry_native::EngineError;
use serde::{Deserialize, Serialize};

const DEFAULT_SCENE: &str = "masonry/tests/default-scene";
const RELEASE_PREFIX: &str = "fixture.release.";

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
}

impl ReleaseScenario {
    pub fn from_connect(connect: &Connect) -> Option<Self> {
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

fn core(body: CommandBody) -> AnyCommand<FlashPayload> {
    AnyCommand::Core(Command::new(CommandId::new_v4(), body))
}
