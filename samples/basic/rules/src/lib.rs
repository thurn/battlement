//! Native rules engine for the standalone basic sample.

#[cfg(test)]
use masonry::{
    Action, ActionId, PointerButton, PointerButtonPayload, PointerPayload, ResponseMessage,
    ScreenPosition,
};
use masonry::{
    ActionBody, CameraClearMode, CameraProjection, CameraState, ClientMessage, Color, Command,
    CommandBody, Connect, CoreErrorCode, Easing, GameObject, GameObjectKind, MaterialAssignment,
    ObjectId, ParentScene, PointerEvent, PreparedAsset, PropertyCommand, Quaternion, Response,
    Scene, SceneId, SessionId, SetMaterialPayload, Snapshot, TextState, Tween,
    TweenPositionPayload, Vector3,
};
use masonry_native::{Engine, EngineError};

const CONTENT_SCENE: &str = "basic/content";
const GRAY_MATERIAL: &str = "basic/material/gray";
const YELLOW_MATERIAL: &str = "basic/material/yellow";
const BLUE_MATERIAL: &str = "basic/material/blue";
const FONT: &str = "basic/font";

struct BasicEngine {
    session_id: SessionId,
    positions: [bool; 3],
    poll_target: Option<ObjectId>,
    polled_change_delivered: bool,
    last_action: &'static str,
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
                CommandBody::RendererSetMaterial(PropertyCommand::canceling(SetMaterialPayload {
                    object_id: payload.object_id,
                    address: YELLOW_MATERIAL.into(),
                    slot: None,
                })),
            ),
            ActionBody::PointerExit(payload) => (
                payload.object_id,
                "pointer exit",
                "target → gray",
                CommandBody::RendererSetMaterial(PropertyCommand::canceling(SetMaterialPayload {
                    object_id: payload.object_id,
                    address: GRAY_MATERIAL.into(),
                    slot: None,
                })),
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
                    CommandBody::TransformTweenLocalPosition(PropertyCommand::canceling(
                        TweenPositionPayload {
                            object_id: payload.object_id,
                            position: Vector3::new(x, 0.0, z),
                            tween: Tween::new().duration_ms(500).easing(Easing::InOutSine),
                        },
                    )),
                )
            }
            _ => return Ok(empty),
        };
        if !self.polled_change_delivered && self.poll_target.is_none() {
            self.poll_target = self::cube_index(object_id)
                .map(|index| self::cube_id((index + 2) % self.positions.len()));
        }
        self.last_action = action_name;
        Ok(Response::commands_for_action(
            self.session_id,
            action.action_id,
            vec![
                body,
                self::status_command(action_name, command_name, "immediate"),
            ],
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
    let scene_id = self::scene_id(1);
    let camera = GameObject::new(
        self::object_id(10),
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
        self::object_id(20),
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
                materials: vec![MaterialAssignment::new(0, GRAY_MATERIAL)],
            },
        )
        .position(Vector3::new(-2.0 + index as f64 * 2.0, 0.0, 0.0))
        .scale(Vector3::new(1.4, 1.4, 1.4))
        .pointer_events([PointerEvent::Enter, PointerEvent::Exit, PointerEvent::Click]);
        objects.push(cube);

        let label = GameObject::new(
            self::object_id(30 + index as u128),
            TextState::new(((b'A' + index as u8) as char).to_string(), FONT).size(2.5),
        )
        .position(Vector3::new(-2.0 + index as f64 * 2.0, 1.3, 0.0));
        objects.push(label);
    }

    Snapshot::new(
        session_id,
        vec![
            PreparedAsset::scene(CONTENT_SCENE),
            PreparedAsset::material(GRAY_MATERIAL),
            PreparedAsset::material(YELLOW_MATERIAL),
            PreparedAsset::material(BLUE_MATERIAL),
            PreparedAsset::font(FONT),
        ],
        vec![Scene::new(scene_id, CONTENT_SCENE)],
        objects,
        self::object_id(10),
    )
}

fn status(action: &str, command: &str, response: &str) -> String {
    format!(
        "Masonry — Basic Native Sample\nRunning  •  native masonry_rules\n\
         last action: {action}  •  last command: {command}  •  response: {response}"
    )
}

fn status_command(action: &str, command: &str, response: &str) -> CommandBody {
    CommandBody::set_text(self::object_id(20), self::status(action, command, response))
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
        poll_target: None,
        polled_change_delivered: false,
        last_action: "none",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_contains_rust_authored_game_and_diagnostics() {
        let snapshot = self::snapshot(SessionId::new_v4());

        assert_eq!(snapshot.objects.len(), 8);
        assert!(
            snapshot
                .prepared_assets
                .contains(&PreparedAsset::Font(FONT.into()))
        );
        assert_eq!(
            snapshot
                .objects
                .iter()
                .filter(|object| matches!(object.kind, GameObjectKind::Cube { .. }))
                .count(),
            3
        );
        assert_eq!(
            snapshot
                .objects
                .iter()
                .filter(|object| matches!(object.kind, GameObjectKind::Text { .. }))
                .count(),
            4
        );
    }

    #[test]
    fn polled_change_follows_first_action_and_targets_another_cube() {
        let mut engine = self::create_engine().expect("engine should be created");
        let session_id = engine.session_id;
        assert!(engine.poll().expect("poll should succeed").is_none());

        let immediate = engine
            .submit(ClientMessage::Action(Action::new(
                ActionId::new_v4(),
                session_id,
                ActionBody::PointerEnter(PointerPayload {
                    object_id: self::cube_id(0),
                    pointer_id: 0,
                    screen_position: ScreenPosition::default(),
                    world_hit: Vector3::default(),
                }),
            )))
            .expect("submit should succeed");
        let ResponseMessage::Batch(immediate_batch) = &immediate.messages[0] else {
            panic!("pointer enter should return a batch");
        };
        let CommandBody::TextSetContent(status) = &immediate_batch.groups[0].commands[1].body
        else {
            panic!("pointer enter should update status");
        };
        assert!(status.text.contains("pointer enter"));
        assert!(status.text.contains("response: immediate"));

        let response = engine
            .poll()
            .expect("poll should succeed")
            .expect("first action should queue a polled change");
        let ResponseMessage::Batch(batch) = &response.messages[0] else {
            panic!("poll should return a batch");
        };
        let CommandBody::RendererSetMaterial(command) = &batch.groups[0].commands[0].body else {
            panic!("poll should set a material");
        };
        assert_eq!(command.payload.object_id, self::cube_id(2));
        let CommandBody::TextSetContent(status) = &batch.groups[0].commands[1].body else {
            panic!("poll should update status");
        };
        assert!(status.text.contains("cube C → blue"));
        assert!(status.text.contains("response: polled"));
        assert!(engine.poll().expect("poll should succeed").is_none());
    }

    #[test]
    fn click_tweens_the_selected_cube_and_updates_status() {
        let mut engine = self::create_engine().expect("engine should be created");
        let response = engine
            .submit(ClientMessage::Action(Action::new(
                ActionId::new_v4(),
                engine.session_id,
                ActionBody::PointerClick(PointerButtonPayload {
                    object_id: self::cube_id(1),
                    pointer_id: 0,
                    screen_position: ScreenPosition::default(),
                    world_hit: Vector3::default(),
                    button: PointerButton::Left,
                }),
            )))
            .expect("submit should succeed");
        let ResponseMessage::Batch(batch) = &response.messages[0] else {
            panic!("click should return a batch");
        };
        let CommandBody::TransformTweenLocalPosition(command) = &batch.groups[0].commands[0].body
        else {
            panic!("click should tween the cube");
        };
        assert_eq!(command.payload.object_id, self::cube_id(1));
        assert_eq!(command.payload.position, Vector3::new(0.0, 0.0, 2.0));
        assert_eq!(command.payload.tween.duration_ms, 500);
        let CommandBody::TextSetContent(status) = &batch.groups[0].commands[1].body else {
            panic!("click should update status");
        };
        assert!(status.text.contains("pointer click"));
        assert!(status.text.contains("500 ms move tween"));
    }
}

masonry_native::export_engine!(create_engine);
