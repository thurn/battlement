//! Native rules engine for the standalone basic sample.

use masonry::{
    ActionBody, ActionId, Batch, BatchId, CameraClearMode, CameraProjection, CameraState,
    ClientMessage, Color, Command, CommandBody, CommandId, Connect, CoreErrorCode, Easing,
    GameObject, GameObjectKind, LocalTransform, MaterialAssignment, ObjectId, ParallelCommandGroup,
    ParentScene, PointerEvent, PreparedAsset, PropertyCommand, Quaternion, Response,
    ResponseMessage, Scene, SceneAddress, SceneId, SessionId, SetMaterialPayload, Snapshot, Tween,
    TweenPositionPayload, Vector3,
};
use masonry_native::{Engine, EngineError};

const CONTENT_SCENE: &str = "basic/content";
const GRAY_MATERIAL: &str = "basic/material/gray";
const YELLOW_MATERIAL: &str = "basic/material/yellow";
const BLUE_MATERIAL: &str = "basic/material/blue";

struct BasicEngine {
    session_id: SessionId,
    positions: [bool; 3],
    poll_pending: bool,
}

impl Engine for BasicEngine {
    type ActionPayload = ();
    type ErrorCode = CoreErrorCode;
    type Command = Command;

    fn connect(&mut self, _message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.session_id = SessionId::new_v4();
        self.positions = [false; 3];
        self.poll_pending = true;
        Ok(Response::new(
            self.session_id,
            vec![ResponseMessage::Snapshot(self::snapshot(self.session_id))],
        ))
    }

    fn submit(
        &mut self,
        message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        let ClientMessage::Action(action) = message else {
            return Ok(Response::new(self.session_id, Vec::new()));
        };
        let (object_id, body) = match action.body {
            ActionBody::PointerEnter(payload) => (
                payload.object_id,
                CommandBody::RendererSetMaterial(PropertyCommand::canceling(SetMaterialPayload {
                    object_id: payload.object_id,
                    address: YELLOW_MATERIAL.into(),
                    slot: None,
                })),
            ),
            ActionBody::PointerExit(payload) => (
                payload.object_id,
                CommandBody::RendererSetMaterial(PropertyCommand::canceling(SetMaterialPayload {
                    object_id: payload.object_id,
                    address: GRAY_MATERIAL.into(),
                    slot: None,
                })),
            ),
            ActionBody::PointerClick(payload) => {
                let Some(index) = self::cube_index(payload.object_id) else {
                    return Ok(Response::new(self.session_id, Vec::new()));
                };
                self.positions[index] = !self.positions[index];
                let x = -2.0 + index as f64 * 2.0;
                let z = if self.positions[index] { 2.0 } else { 0.0 };
                (
                    payload.object_id,
                    CommandBody::TransformTweenLocalPosition(PropertyCommand::canceling(
                        TweenPositionPayload {
                            object_id: payload.object_id,
                            position: Vector3::new(x, 0.0, z),
                            tween: Tween {
                                duration_ms: 500,
                                easing: Easing::InOutSine,
                                ..Tween::default()
                            },
                        },
                    )),
                )
            }
            _ => return Ok(Response::new(self.session_id, Vec::new())),
        };
        Ok(self::batch(
            self.session_id,
            action.action_id,
            object_id,
            body,
        ))
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        if !self.poll_pending {
            return Ok(None);
        }
        self.poll_pending = false;
        Ok(Some(self::command_response(
            self.session_id,
            CommandBody::RendererSetMaterial(PropertyCommand::canceling(SetMaterialPayload {
                object_id: self::cube_id(2),
                address: BLUE_MATERIAL.into(),
                slot: None,
            })),
        )))
    }
}

fn snapshot(session_id: SessionId) -> Snapshot {
    let scene_id = self::scene_id(1);
    let mut camera = GameObject::new(
        self::object_id(10),
        GameObjectKind::Camera {
            camera: CameraState {
                projection: CameraProjection::Perspective,
                field_of_view: 52.0,
                clear_mode: CameraClearMode::SolidColor,
                clear_color: Color {
                    r: 0.025,
                    g: 0.035,
                    b: 0.065,
                    a: 1.0,
                },
                ..CameraState::default()
            },
        },
    );
    camera.parent_scene = ParentScene::Persistent;
    camera.local_transform.position = Vector3::new(0.0, 3.0, -7.5);
    camera.local_transform.rotation = Quaternion {
        x: 0.16,
        y: 0.0,
        z: 0.0,
        w: 0.987,
    };

    let mut objects = vec![camera];
    for index in 0..3 {
        let mut cube = GameObject::new(
            self::cube_id(index),
            GameObjectKind::Cube {
                materials: vec![MaterialAssignment::new(0, GRAY_MATERIAL)],
            },
        );
        cube.parent_scene = ParentScene::Scene(scene_id);
        cube.local_transform = LocalTransform {
            position: Vector3::new(-2.0 + index as f64 * 2.0, 0.0, 0.0),
            scale: Vector3::new(1.6, 1.6, 1.6),
            ..LocalTransform::default()
        };
        cube.pointer_events = vec![PointerEvent::Enter, PointerEvent::Exit, PointerEvent::Click];
        objects.push(cube);
    }

    Snapshot::new(
        session_id,
        vec![
            PreparedAsset::Scene(SceneAddress::new(CONTENT_SCENE)),
            PreparedAsset::Material(GRAY_MATERIAL.into()),
            PreparedAsset::Material(YELLOW_MATERIAL.into()),
            PreparedAsset::Material(BLUE_MATERIAL.into()),
        ],
        vec![Scene::new(scene_id, CONTENT_SCENE)],
        objects,
        self::object_id(10),
    )
}

fn batch(
    session_id: SessionId,
    action_id: ActionId,
    _object_id: ObjectId,
    body: CommandBody,
) -> Response<Command> {
    let mut batch = Batch::new(
        BatchId::new_v4(),
        session_id,
        vec![ParallelCommandGroup::new(vec![Command::new(
            CommandId::new_v4(),
            body,
        )])],
    );
    batch.caused_by_action_id = Some(action_id);
    Response::new(session_id, vec![ResponseMessage::Batch(batch)])
}

fn command_response(session_id: SessionId, body: CommandBody) -> Response<Command> {
    Response::new(
        session_id,
        vec![ResponseMessage::Batch(Batch::new(
            BatchId::new_v4(),
            session_id,
            vec![ParallelCommandGroup::new(vec![Command::new(
                CommandId::new_v4(),
                body,
            )])],
        ))],
    )
}

fn cube_index(id: ObjectId) -> Option<usize> {
    (0..3).find(|index| self::cube_id(*index) == id)
}

fn cube_id(index: usize) -> ObjectId {
    self::object_id(100 + index as u128)
}

fn object_id(value: u128) -> ObjectId {
    format!("{value:032x}").parse().expect("fixed object ID")
}

fn scene_id(value: u128) -> SceneId {
    format!("{value:032x}").parse().expect("fixed scene ID")
}

fn create_engine() -> Result<BasicEngine, EngineError> {
    Ok(BasicEngine {
        session_id: SessionId::new_v4(),
        positions: [false; 3],
        poll_pending: false,
    })
}

masonry_native::export_engine!(create_engine);
