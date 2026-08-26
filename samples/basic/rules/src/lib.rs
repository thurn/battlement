//! Native rules engine for the standalone basic sample.

use battlement::{
    ActionBody, CameraClearMode, CameraProjection, CameraState, ClientMessage, Color, Command,
    CommandBody, Connect, CoreErrorCode, DragMode, Easing, GameObject, GameObjectKind,
    MaterialAssignment, ObjectId, ParentScene, PointerEvent, PositionPayload, PreparedAsset,
    PropertyCommand, Quaternion, Response, Scene, SceneId, SessionId, SetMaterialPayload, Snapshot,
    TextState, Tween, TweenPositionPayload, Vector3, object_id, scene_id,
};
use battlement_native::{Engine, EngineError};

const SCENE_ID: SceneId = scene_id!("cfd68d2d-e6d4-4b6c-a259-c729cd7e190c");

/// Address of the sample's content scene.
pub const CONTENT_SCENE: &str = "basic/content";
/// Address of the cubes' initial material.
pub const WHITE_MATERIAL: &str = "basic/material/white";
/// Address of the cubes' hover material.
pub const YELLOW_MATERIAL: &str = "basic/material/yellow";
/// Address of the material applied by the polled response.
pub const BLUE_MATERIAL: &str = "basic/material/blue";
/// Address of the sample's text font.
pub const FONT: &str = "basic/font";
/// Stable identity of the sample's input camera.
pub const CAMERA_ID: ObjectId = object_id!("54ad5cfa-5698-42e5-b32d-01da99539bfc");
/// Stable identity of the visible diagnostic status text.
pub const STATUS_ID: ObjectId = object_id!("2a188803-9663-43a0-b79b-7884f44d23a8");
/// Stable identities of the sample's interactive cubes.
pub const CUBE_IDS: [ObjectId; 3] = [
    object_id!("9c8921d4-ab2a-4287-a678-68ae3880a6f7"),
    object_id!("93c29a0f-1d4e-4aed-b797-011d730036cc"),
    object_id!("ab96efc3-f6f8-46b8-ad99-3e8f4319c2a0"),
];

/// Native basic-sample rules engine.
pub struct BasicEngine {
    session_id: SessionId,
    positions: [bool; 3],
    poll_target: Option<ObjectId>,
    polled_change_delivered: bool,
    last_action: &'static str,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<BasicEngine, EngineError> {
    Ok(BasicEngine {
        session_id: SessionId::new_v4(),
        positions: [false; 3],
        poll_target: None,
        polled_change_delivered: false,
        last_action: "none",
    })
}

impl Engine for BasicEngine {
    type ActionPayload = ();
    type ErrorCode = CoreErrorCode;
    type Command = Command;

    fn connect(&mut self, _message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.session_id = SessionId::new_v4();
        self.positions = [false; 3];
        self.poll_target = None;
        self.polled_change_delivered = false;
        self.last_action = "none";
        Ok(Response::snapshot(self::snapshot(self.session_id)))
    }

    fn submit(
        &mut self,
        message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        let empty = Response::empty(self.session_id);
        let Some(action) = message.into_action() else {
            return Ok(empty);
        };
        let (object_id, action_name, command_name, body) = match action.body {
            ActionBody::PointerEnter(payload) => (
                payload.object_id,
                "pointer enter",
                "target → yellow",
                Some(CommandBody::RendererSetMaterial(
                    PropertyCommand::canceling(SetMaterialPayload {
                        object_id: payload.object_id,
                        address: YELLOW_MATERIAL.into(),
                        slot: None,
                    }),
                )),
            ),
            ActionBody::PointerExit(payload) => (
                payload.object_id,
                "pointer exit",
                "target → white",
                Some(CommandBody::RendererSetMaterial(
                    PropertyCommand::canceling(SetMaterialPayload {
                        object_id: payload.object_id,
                        address: WHITE_MATERIAL.into(),
                        slot: None,
                    }),
                )),
            ),
            ActionBody::PointerClick(payload) => {
                let Some(index) = self::cube_index(payload.object_id) else {
                    return Ok(empty);
                };
                self.positions[index] = !self.positions[index];
                let x = -2.0 + index as f64 * 2.0;
                let z = if self.positions[index] { 2.0 } else { 0.0 };
                (
                    payload.object_id,
                    "pointer click",
                    "500 ms move tween",
                    Some(CommandBody::TransformTweenLocalPosition(
                        PropertyCommand::canceling(TweenPositionPayload {
                            object_id: payload.object_id,
                            position: Vector3::new(x, 0.0, z),
                            tween: Tween::new().duration_ms(500).easing(Easing::InOutSine),
                        }),
                    )),
                )
            }
            ActionBody::DragStart(payload) => (
                payload.object_id,
                "drag start",
                "local pointer capture",
                None,
            ),
            ActionBody::DragEnd(payload) => (
                payload.object_id,
                "drag end",
                "commit world position",
                Some(CommandBody::TransformSetWorldPosition(
                    PropertyCommand::canceling(PositionPayload {
                        object_id: payload.object_id,
                        position: payload.world_position,
                    }),
                )),
            ),
            _ => return Ok(empty),
        };
        if !self.polled_change_delivered && self.poll_target.is_none() {
            self.poll_target = self::cube_index(object_id)
                .map(|index| self::cube_id((index + 2) % self.positions.len()));
        }
        self.last_action = action_name;
        let commands =
            body.into_iter()
                .chain([self::status_command(action_name, command_name, "immediate")]);
        Ok(Response::commands_for_action(
            self.session_id,
            action.action_id,
            commands,
        ))
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        let Some(object_id) = self.poll_target.take() else {
            return Ok(None);
        };
        self.polled_change_delivered = true;
        let label =
            (b'A' + self::cube_index(object_id).expect("poll target is a cube") as u8) as char;
        let command = format!("cube {label} → blue");
        Ok(Some(Response::commands(
            self.session_id,
            vec![
                CommandBody::RendererSetMaterial(PropertyCommand::canceling(SetMaterialPayload {
                    object_id,
                    address: BLUE_MATERIAL.into(),
                    slot: None,
                })),
                self::status_command(self.last_action, &command, "polled"),
            ],
        )))
    }
}

fn snapshot(session_id: SessionId) -> Snapshot {
    let camera = GameObject::new(
        CAMERA_ID,
        CameraState::new()
            .projection(CameraProjection::Perspective)
            .field_of_view(52.0)
            .clear_mode(CameraClearMode::SolidColor)
            .clear_color(Color::rgb(0.025, 0.035, 0.065)),
    )
    .parent_scene(ParentScene::Persistent)
    .position(Vector3::new(0.0, 2.8, -11.0))
    .rotation(Quaternion::new(0.12, 0.0, 0.0, 0.993));

    let status = GameObject::new(
        STATUS_ID,
        TextState::new(self::status("none", "initial snapshot", "connect"), FONT)
            .size(1.8)
            .wrap_width(18.0),
    )
    .parent_scene(ParentScene::Persistent)
    .position(Vector3::new(0.0, 3.25, 1.0));

    let mut objects = vec![camera, status];
    for index in 0..3 {
        let cube = GameObject::new(
            self::cube_id(index),
            GameObjectKind::Cube {
                materials: vec![MaterialAssignment::new(0, WHITE_MATERIAL)],
            },
        )
        .position(Vector3::new(-2.0 + index as f64 * 2.0, 0.0, 0.0))
        .scale(Vector3::new(1.4, 1.4, 1.4))
        .pointer_events([PointerEvent::Enter, PointerEvent::Exit, PointerEvent::Click]);
        let cube = match index {
            0 => cube.draggable(DragMode::SnapToPointer),
            1 => cube.draggable(DragMode::PreserveOffset),
            _ => cube,
        };
        objects.push(cube);

        let label = GameObject::new(
            self::label_id(index),
            TextState::new(((b'A' + index as u8) as char).to_string(), FONT).size(2.5),
        )
        .position(Vector3::new(-2.0 + index as f64 * 2.0, 1.3, 0.0));
        objects.push(label);
    }

    Snapshot::new(
        session_id,
        vec![
            PreparedAsset::scene(CONTENT_SCENE),
            PreparedAsset::material(WHITE_MATERIAL),
            PreparedAsset::material(YELLOW_MATERIAL),
            PreparedAsset::material(BLUE_MATERIAL),
            PreparedAsset::text_mesh_pro_font(FONT),
        ],
        vec![Scene::new(SCENE_ID, CONTENT_SCENE)],
        objects,
        CAMERA_ID,
    )
}

fn status(action: &str, command: &str, response: &str) -> String {
    format!(
        "Battlement — Basic Native Sample\nA: snap drag  •  B: offset drag  •  C: click tween\n\
         Running  •  native battlement_rules\n\
         last action: {action}  •  last command: {command}  •  response: {response}"
    )
}

fn status_command(action: &str, command: &str, response: &str) -> CommandBody {
    CommandBody::set_text(STATUS_ID, self::status(action, command, response))
}

fn cube_index(id: ObjectId) -> Option<usize> {
    (0..3).find(|index| self::cube_id(*index) == id)
}

fn cube_id(index: usize) -> ObjectId {
    CUBE_IDS[index]
}

fn label_id(index: usize) -> ObjectId {
    [
        object_id!("8aaf3f5a-c30a-4b57-9e57-83492ae48f92"),
        object_id!("1a4b48f1-d599-470e-9388-6965edb45798"),
        object_id!("6812d3b2-151e-46d7-a4b9-d41c36c44f33"),
    ][index]
}

battlement_native::export_engine!(create_engine);
