//! Native Rust engine for the standalone Battlement UI lab.

use battlement::{
    Box, CameraState, ClientMessage, Color, Command, Connect, CoreErrorCode, FlexDirection,
    GameObject, GameObjectKind, Label, ObjectId, PanelScaleMode, PanelSettings, ParentScene,
    PreparedAsset, Response, Scene, SceneId, SessionId, Snapshot, Style, UiDocument,
    UiDocumentState, VisualElement, object_id, scene_id,
};
use battlement_native::{Engine, EngineError};

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
    let document_state = UiDocumentState::new(ROOT_ID)
        .panel_settings(PanelSettings::new().scale_mode(PanelScaleMode::ConstantPixelSize));
    let document = GameObject::new(DOCUMENT_ID, GameObjectKind::UiDocument(document_state))
        .parent_scene(ParentScene::Persistent);
    let camera =
        GameObject::new(CAMERA_ID, CameraState::new()).parent_scene(ParentScene::Persistent);
    let ui = UiDocument::with_root_id(DOCUMENT_ID, ROOT_ID)
        .name("battlement-ui-lab")
        .style(
            Style::new()
                .background_color(Color::rgb(0.012, 0.025, 0.045))
                .color(Color::rgb(0.78, 0.88, 0.92))
                .flex_direction(FlexDirection::Row)
                .padding(18.0),
        )
        .child(navigation())
        .child(canvas())
        .child(inspector());
    let mut snapshot = Snapshot::new(
        session_id,
        vec![PreparedAsset::scene(CONTENT_SCENE)],
        vec![Scene::new(SCENE_ID, CONTENT_SCENE)],
        vec![document, camera],
        CAMERA_ID,
    );
    snapshot.ui.push(ui);
    snapshot
}

fn navigation() -> Box {
    Box::new()
        .name("navigation")
        .style(
            Style::new()
                .width(250.0)
                .background_color(Color::rgb(0.025, 0.065, 0.085))
                .padding(22.0),
        )
        .child(
            Label::new("BATTLEMENT").name("brand").style(
                Style::new()
                    .color(Color::rgb(0.18, 0.9, 0.95))
                    .font_size(24.0)
                    .margin(8.0),
            ),
        )
        .children([
            nav_label("01  OVERVIEW", true),
            nav_label("02  HIERARCHY", false),
            nav_label("03  ASSETS", false),
            nav_label("04  STYLING", false),
            nav_label("05  CONTROLS", false),
            nav_label("06  EVENTS", false),
            nav_label("07  RENDER MODES", false),
        ])
}

fn nav_label(text: &str, active: bool) -> Label {
    Label::new(text).style(
        Style::new()
            .color(if active {
                Color::rgb(0.95, 0.68, 0.22)
            } else {
                Color::rgb(0.42, 0.58, 0.64)
            })
            .font_size(15.0)
            .margin(9.0),
    )
}

fn canvas() -> VisualElement {
    VisualElement::new()
        .name("specimen-canvas")
        .style(
            Style::new()
                .background_color(Color::rgb(0.012, 0.025, 0.045))
                .flex_grow(1.0)
                .padding(28.0),
        )
        .child(
            Label::new("UI FOUNDATION / OVERVIEW").style(
                Style::new()
                    .font_size(14.0)
                    .color(Color::rgb(0.95, 0.68, 0.22)),
            ),
        )
        .child(
            Label::new("COMMAND DECK").style(
                Style::new()
                    .font_size(38.0)
                    .color(Color::rgb(0.86, 0.95, 0.97)),
            ),
        )
        .child(
            Box::new()
                .name("first-specimen")
                .style(
                    Style::new()
                        .background_color(Color::rgb(0.035, 0.09, 0.115))
                        .padding(24.0)
                        .margin(18.0),
                )
                .child(
                    Label::new("FIRST RUST-AUTHORED LABEL").style(
                        Style::new()
                            .font_size(22.0)
                            .color(Color::rgb(0.18, 0.9, 0.95)),
                    ),
                )
                .child(Label::new(
                    "VisualElement → Box → Label\nScreen-space document online",
                )),
        )
}

fn inspector() -> Box {
    Box::new()
        .name("inspector")
        .style(
            Style::new()
                .width(310.0)
                .background_color(Color::rgb(0.018, 0.045, 0.06))
                .padding(22.0),
        )
        .child(
            Label::new("STATE / EVENT / COMMAND").style(
                Style::new()
                    .font_size(14.0)
                    .color(Color::rgb(0.95, 0.68, 0.22)),
            ),
        )
        .child(Label::new("DOCUMENT ROOT").style(Style::new().font_size(20.0)))
        .child(Label::new(ROOT_ID.to_string()).style(Style::new().font_size(12.0)))
        .child(Label::new(
            "type   VisualElement\nmode   ScreenSpaceOverlay\nsource Rust snapshot",
        ))
}

battlement_native::export_engine!(create_engine);

#[cfg(test)]
mod tests {
    use battlement::{SessionId, Validate};

    #[test]
    fn ui_only_snapshot_includes_valid_camera_and_input_setup() {
        assert!(crate::snapshot(SessionId::new_v4()).validate().is_ok());
    }
}
