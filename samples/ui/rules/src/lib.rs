//! Native Rust engine for the standalone Battlement UI lab.

use battlement::{
    CameraState, ClientMessage, Command, Connect, CoreErrorCode, GameObject, ObjectId, ParentScene,
    PreparedAsset, Response, Scene, SceneId, SessionId, Snapshot, UiDocument, object_id, scene_id,
};
use battlement_native::{Engine, EngineError};

mod components;
mod design_system;

const SCENE_ID: SceneId = scene_id!("cf5dd2ef-7df2-414f-a616-cbae8b9462b5");
const DOCUMENT_ID: ObjectId = object_id!("1a7d999f-ceb2-40af-9267-3bff4628d7a5");
const ROOT_ID: ObjectId = object_id!("d463c180-1ecf-4b23-b205-9f3259aa2376");
const CAMERA_ID: ObjectId = object_id!("c097e11b-4ec3-43e1-9320-609ef0f61a12");

/// Address of the sample's minimal content scene.
pub const CONTENT_SCENE: &str = "ui/content";

/// Native UI-lab rules engine.
pub struct UiLabEngine {
    session_id: SessionId,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<UiLabEngine, EngineError> {
    Ok(UiLabEngine {
        session_id: SessionId::new_v4(),
    })
}

impl Engine for UiLabEngine {
    type ActionPayload = ();
    type ErrorCode = CoreErrorCode;
    type Command = Command;

    fn connect(&mut self, _message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.session_id = SessionId::new_v4();
        Ok(Response::snapshot(snapshot(self.session_id)))
    }

    fn submit(
        &mut self,
        _message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        Ok(Response::empty(self.session_id))
    }

    fn poll(&mut self) -> Result<Option<Response<Self::Command>>, EngineError> {
        Ok(None)
    }
}

fn snapshot(session_id: SessionId) -> Snapshot {
    let camera =
        GameObject::new(CAMERA_ID, CameraState::new()).parent_scene(ParentScene::Persistent);
    let ui = UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID)
        .name("battlement-ui-lab")
        .style(design_system::root())
        .child(components::navigation())
        .child(components::canvas())
        .child(components::inspector(ROOT_ID, Some("Rust snapshot")));
    Snapshot::new(
        session_id,
        vec![PreparedAsset::scene(CONTENT_SCENE)],
        vec![Scene::new(SCENE_ID, CONTENT_SCENE)],
        vec![camera],
        CAMERA_ID,
    )
    .ui_document(ui)
}

battlement_native::export_engine!(create_engine);
