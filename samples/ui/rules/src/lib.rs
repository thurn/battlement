//! Native Rust engine for the standalone Battlement UI lab.

use battlement::{
    ActionBody, Batch, BatchId, Box, Button, CameraState, ClientMessage, Color, Command, Connect,
    CoreErrorCode, GameObject, ObjectId, ParallelCommandGroup, ParentScene, PreparedAsset,
    Response, Scene, SceneId, SessionId, Snapshot, Style, UiDocument, UiEventBody, UiNode,
    object_id, scene_id,
};
use battlement_native::{Engine, EngineError};

mod components;
mod design_system;

const SCENE_ID: SceneId = scene_id!("cf5dd2ef-7df2-414f-a616-cbae8b9462b5");
const DOCUMENT_ID: ObjectId = object_id!("1a7d999f-ceb2-40af-9267-3bff4628d7a5");
const ROOT_ID: ObjectId = object_id!("d463c180-1ecf-4b23-b205-9f3259aa2376");
const CAMERA_ID: ObjectId = object_id!("c097e11b-4ec3-43e1-9320-609ef0f61a12");
const COMPONENTS_BUTTON_ID: ObjectId = object_id!("0e95fbc2-b5e9-4e0f-937f-86aab38b6855");
const INTERACTIONS_BUTTON_ID: ObjectId = object_id!("4969d46f-c28c-4e5d-85a0-0321f9931f89");
const CANVAS_ID: ObjectId = object_id!("92a7f3b3-8c0e-41c2-b42d-291f0b937c0d");
const PAGE_ID: ObjectId = object_id!("28951e4f-6f61-491e-8548-84b9d4a356e4");
const LABEL_COMPONENT_ID: ObjectId = object_id!("5768cfee-a137-49c0-b76c-5ebfa6c227c1");
const CALLBACK_BUTTON_ID: ObjectId = object_id!("7e0b078e-13d9-43c3-a491-84178e157fb2");
const GREETING_ID: ObjectId = object_id!("2d8ac61c-49bb-43ce-9656-faa11238351f");
const TRANSIENT_CARD_ID: ObjectId = object_id!("45a1a00c-2624-4e40-b675-3c5f59c62f53");

#[derive(Clone, Copy, Eq, PartialEq)]
enum Page {
    Components,
    Interactions,
}

/// Address of the sample's minimal content scene.
pub const CONTENT_SCENE: &str = "ui/content";

/// Native UI-lab rules engine.
pub struct UiLabEngine {
    session_id: SessionId,
    page: Page,
    greeting_visible: bool,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<UiLabEngine, EngineError> {
    Ok(UiLabEngine {
        session_id: SessionId::new_v4(),
        page: Page::Components,
        greeting_visible: false,
    })
}

impl Engine for UiLabEngine {
    type ActionPayload = ();
    type ErrorCode = CoreErrorCode;
    type Command = Command;

    fn connect(&mut self, _message: Connect) -> Result<Response<Self::Command>, EngineError> {
        self.session_id = SessionId::new_v4();
        self.page = Page::Components;
        self.greeting_visible = false;
        Ok(Response::snapshot(snapshot(self.session_id)))
    }

    fn submit(
        &mut self,
        message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
    ) -> Result<Response<Self::Command>, EngineError> {
        let ClientMessage::Action(action) = message else {
            return Ok(Response::empty(self.session_id));
        };
        let ActionBody::VisualElement(event) = action.body else {
            return Ok(Response::empty(self.session_id));
        };
        if !matches!(event.body, UiEventBody::Click(_)) {
            return Ok(Response::empty(self.session_id));
        }
        let commands = match event.target_id {
            COMPONENTS_BUTTON_ID if self.page != Page::Components => {
                self.page = Page::Components;
                self.greeting_visible = false;
                navigation_commands(Page::Components)
            }
            INTERACTIONS_BUTTON_ID if self.page != Page::Interactions => {
                self.page = Page::Interactions;
                self.greeting_visible = false;
                navigation_commands(Page::Interactions)
            }
            CALLBACK_BUTTON_ID if self.page == Page::Interactions && !self.greeting_visible => {
                self.greeting_visible = true;
                show_greeting_commands()
            }
            CALLBACK_BUTTON_ID if self.page == Page::Interactions => {
                self.greeting_visible = false;
                hide_greeting_commands()
            }
            _ => Vec::new(),
        };
        if commands.is_empty() {
            return Ok(Response::empty(self.session_id));
        }
        Ok(Response::batch(
            Batch::new(BatchId::new_v4(), self.session_id, commands)
                .caused_by_action_id(action.action_id),
        ))
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
        .child(components::navigation(
            COMPONENTS_BUTTON_ID,
            INTERACTIONS_BUTTON_ID,
        ))
        .child(components::canvas(CANVAS_ID, PAGE_ID, LABEL_COMPONENT_ID));
    Snapshot::new(
        session_id,
        vec![PreparedAsset::scene(CONTENT_SCENE)],
        vec![Scene::new(SCENE_ID, CONTENT_SCENE)],
        vec![camera],
        CAMERA_ID,
    )
    .ui_document(ui)
}

fn navigation_commands(page: Page) -> Vec<ParallelCommandGroup<Command>> {
    let content = match page {
        Page::Components => components::components_page(PAGE_ID, LABEL_COMPONENT_ID),
        Page::Interactions => components::interactions_page(PAGE_ID, CALLBACK_BUTTON_ID),
    };
    let components_active = page == Page::Components;
    vec![
        ParallelCommandGroup::new(vec![Command::destroy_visual_element(PAGE_ID)]),
        ParallelCommandGroup::new(vec![
            Command::create_visual_element(CANVAS_ID, content),
            Command::update_visual_element(
                COMPONENTS_BUTTON_ID,
                Button::default().style(design_system::navigation_item(components_active)),
            ),
            Command::update_visual_element(
                INTERACTIONS_BUTTON_ID,
                Button::default().style(design_system::navigation_item(!components_active)),
            ),
        ]),
    ]
}

fn show_greeting_commands() -> Vec<ParallelCommandGroup<Command>> {
    let transient = UiNode::new(
        TRANSIENT_CARD_ID,
        Box::new().style(Style::new().background_color(Color::rgb(0.08, 0.2, 0.24))),
    );
    vec![
        ParallelCommandGroup::new(vec![Command::create_visual_element(PAGE_ID, transient)]),
        ParallelCommandGroup::new(vec![Command::update_visual_element(
            TRANSIENT_CARD_ID,
            Box::default()
                .name("updated-callback-result")
                .style(Style::new().background_color(Color::rgb(0.1, 0.36, 0.4))),
        )]),
        ParallelCommandGroup::new(vec![Command::update_visual_element_parent(
            TRANSIENT_CARD_ID,
            PAGE_ID,
        )]),
        ParallelCommandGroup::new(vec![Command::update_visual_element_index(
            TRANSIENT_CARD_ID,
            0,
        )]),
        ParallelCommandGroup::new(vec![Command::destroy_visual_element(TRANSIENT_CARD_ID)]),
        ParallelCommandGroup::new(vec![
            Command::create_visual_element(PAGE_ID, components::greeting(GREETING_ID)),
            Command::update_visual_element(CALLBACK_BUTTON_ID, Button::new("Hide")),
        ]),
    ]
}

fn hide_greeting_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::destroy_visual_element(GREETING_ID),
        Command::update_visual_element(
            CALLBACK_BUTTON_ID,
            Button::new("Click to run a Rust callback"),
        ),
    ])]
}

battlement_native::export_engine!(create_engine);
