//! Native Rust engine for the standalone Battlement UI lab.

use battlement::{
    ActionBody, Batch, BatchId, Box, Button, CameraState, ClientMessage, Color, Command, Connect,
    CoreErrorCode, GameObject, Image, Label, ObjectId, ParallelCommandGroup, ParentScene,
    PickingMode, Response, Scene, SceneId, SessionId, Snapshot, Style, UiDocument, UiEventBody,
    UiNode, object_id, scene_id,
};
use battlement_native::{Engine, EngineError};

#[path = "assets.rs"]
pub mod asset_catalog;

mod appearance_styles;
mod asset_styles;
mod component_styles;
mod components;
mod design_system;
mod hierarchy_styles;
mod interaction_styles;
mod layout_styles;

use crate::asset_catalog::ui::{self as ui_assets, assets};

const SCENE_ID: SceneId = scene_id!("cf5dd2ef-7df2-414f-a616-cbae8b9462b5");
const DOCUMENT_ID: ObjectId = object_id!("1a7d999f-ceb2-40af-9267-3bff4628d7a5");
const ROOT_ID: ObjectId = object_id!("d463c180-1ecf-4b23-b205-9f3259aa2376");
const CAMERA_ID: ObjectId = object_id!("c097e11b-4ec3-43e1-9320-609ef0f61a12");
const COMPONENTS_BUTTON_ID: ObjectId = object_id!("0e95fbc2-b5e9-4e0f-937f-86aab38b6855");
const INTERACTIONS_BUTTON_ID: ObjectId = object_id!("4969d46f-c28c-4e5d-85a0-0321f9931f89");
const HIERARCHY_BUTTON_ID: ObjectId = object_id!("02e0f324-4781-4301-9502-93435d7eea7e");
const CANVAS_ID: ObjectId = object_id!("92a7f3b3-8c0e-41c2-b42d-291f0b937c0d");
const PAGE_ID: ObjectId = object_id!("28951e4f-6f61-491e-8548-84b9d4a356e4");
const LABEL_COMPONENT_ID: ObjectId = object_id!("5768cfee-a137-49c0-b76c-5ebfa6c227c1");
const CALLBACK_BUTTON_ID: ObjectId = object_id!("7e0b078e-13d9-43c3-a491-84178e157fb2");
const GREETING_ID: ObjectId = object_id!("2d8ac61c-49bb-43ce-9656-faa11238351f");
const TRANSIENT_CARD_ID: ObjectId = object_id!("45a1a00c-2624-4e40-b675-3c5f59c62f53");
const HIERARCHY_BRANCH_ID: ObjectId = object_id!("53e9582f-36c9-47fb-91c7-a6f7c7b3dd50");
const HIERARCHY_PRIMARY_ID: ObjectId = object_id!("f48e306d-ec3a-4881-abeb-ae685b0bb956");
const HIERARCHY_SECONDARY_ID: ObjectId = object_id!("45ee68d7-72bf-4d1b-bba3-e0a2834c5f06");
const HIERARCHY_MOVABLE_ID: ObjectId = object_id!("0121bbc8-ceb1-42ea-bea0-a7601543851e");
const HIERARCHY_DESTINATION_ID: ObjectId = object_id!("98ec6daa-7faa-41aa-a157-afb9beca284d");
const HIERARCHY_ACTION_ID: ObjectId = object_id!("51e73f5f-1af1-4f54-bcf6-288cde0f45ee");
const ASSETS_BUTTON_ID: ObjectId = object_id!("81083fd8-6546-4a11-8765-32592ede0a3e");
const TEXTURE_IMAGE_ID: ObjectId = object_id!("d4e9b4cf-cb57-4fd7-8d92-ee8420b095c4");
const SPRITE_IMAGE_ID: ObjectId = object_id!("0665cd59-2629-4ded-92eb-65413a5374ad");
const VECTOR_IMAGE_ID: ObjectId = object_id!("f48633c5-ca86-4c1c-a907-ae2eafa639ac");
const RENDER_IMAGE_ID: ObjectId = object_id!("41ce020f-64c1-4b6a-b8ee-b0d15115e958");
const SWITCHED_IMAGE_ID: ObjectId = object_id!("b64232bb-97c1-4a00-95cf-01b8bc8a27f8");
const ACTIVE_ADDRESS_ID: ObjectId = object_id!("4e0386da-f6ed-46fe-be94-5b1fd9f056e2");
const SOURCE_SWITCH_ID: ObjectId = object_id!("6a383965-6837-4898-946e-5aa76d49f193");
const LAYOUT_BUTTON_ID: ObjectId = object_id!("e100c957-35e6-456c-90ef-5b839424a5cf");
const LAYOUT_PLAYGROUND_ID: ObjectId = object_id!("419ee1dc-73f8-4968-a9ad-552d38592398");
const LAYOUT_ALPHA_ID: ObjectId = object_id!("9d2ae871-2ce9-4707-85a7-bc8263cb0e37");
const LAYOUT_BETA_ID: ObjectId = object_id!("eca45793-0262-46e4-9de3-4d833101b29d");
const LAYOUT_GAMMA_ID: ObjectId = object_id!("3dbc8a14-b4b2-42b5-83f0-f83f564dadc4");
const LAYOUT_ACTION_ID: ObjectId = object_id!("274aa2af-5b70-4079-a260-25fadd46f339");
const APPEARANCE_BUTTON_ID: ObjectId = object_id!("7237e7ab-178f-438e-a457-0106b1899f6d");
const APPEARANCE_SQUARE_ID: ObjectId = object_id!("398a0a4f-39d9-444c-9128-0b9c43cc4ce5");
const APPEARANCE_ROUNDED_ID: ObjectId = object_id!("b5a3be06-e219-4bbc-a18a-2c2d9ba8ee11");
const APPEARANCE_SLICED_ID: ObjectId = object_id!("2b6868b0-042c-4258-b7fe-d594c788cf5d");
const APPEARANCE_OPACITY_ID: ObjectId = object_id!("e5a079c6-6e94-4aaf-a84b-6ad1c461be8f");
const APPEARANCE_CLIPPED_ID: ObjectId = object_id!("1da43df8-2db8-4975-b6a7-2f84abb9f5ae");
const APPEARANCE_HIDDEN_ID: ObjectId = object_id!("3658659b-69e6-4c1e-bf96-6ba1473d0ac2");
const APPEARANCE_REMOVED_ID: ObjectId = object_id!("f2360cdc-c121-41af-8ae2-486eb817669f");
const APPEARANCE_ACTION_ID: ObjectId = object_id!("876cec21-9d24-40e3-ba85-f27e0262112c");

#[derive(Clone, Copy, Eq, PartialEq)]
enum Page {
    Components,
    Interactions,
    Hierarchy,
    Assets,
    Layout,
    Appearance,
}

/// Address of the sample's minimal content scene.
pub const CONTENT_SCENE: &str = "ui/content";

/// Native UI-lab rules engine.
pub struct UiLabEngine {
    session_id: SessionId,
    page: Page,
    greeting_visible: bool,
    hierarchy_applied: bool,
    sprite_source_active: bool,
    layout_adjusted: bool,
    appearance_revealed: bool,
}

/// Creates the engine used by the native sample.
pub fn create_engine() -> Result<UiLabEngine, EngineError> {
    Ok(UiLabEngine {
        session_id: SessionId::new_v4(),
        page: Page::Components,
        greeting_visible: false,
        hierarchy_applied: false,
        sprite_source_active: false,
        layout_adjusted: false,
        appearance_revealed: false,
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
        self.hierarchy_applied = false;
        self.sprite_source_active = false;
        self.layout_adjusted = false;
        self.appearance_revealed = false;
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
            HIERARCHY_BUTTON_ID if self.page != Page::Hierarchy => {
                self.page = Page::Hierarchy;
                self.greeting_visible = false;
                self.hierarchy_applied = false;
                navigation_commands(Page::Hierarchy)
            }
            ASSETS_BUTTON_ID if self.page != Page::Assets => {
                self.page = Page::Assets;
                self.greeting_visible = false;
                self.sprite_source_active = false;
                navigation_commands(Page::Assets)
            }
            LAYOUT_BUTTON_ID if self.page != Page::Layout => {
                self.page = Page::Layout;
                self.greeting_visible = false;
                self.layout_adjusted = false;
                navigation_commands(Page::Layout)
            }
            APPEARANCE_BUTTON_ID if self.page != Page::Appearance => {
                self.page = Page::Appearance;
                self.greeting_visible = false;
                self.appearance_revealed = false;
                navigation_commands(Page::Appearance)
            }
            CALLBACK_BUTTON_ID if self.page == Page::Interactions && !self.greeting_visible => {
                self.greeting_visible = true;
                show_greeting_commands()
            }
            CALLBACK_BUTTON_ID if self.page == Page::Interactions => {
                self.greeting_visible = false;
                hide_greeting_commands()
            }
            HIERARCHY_ACTION_ID if self.page == Page::Hierarchy && !self.hierarchy_applied => {
                self.hierarchy_applied = true;
                apply_hierarchy_commands()
            }
            HIERARCHY_ACTION_ID if self.page == Page::Hierarchy => {
                self.hierarchy_applied = false;
                reset_hierarchy_commands()
            }
            SOURCE_SWITCH_ID if self.page == Page::Assets => {
                self.sprite_source_active = !self.sprite_source_active;
                switch_source_commands(self.sprite_source_active)
            }
            LAYOUT_ACTION_ID if self.page == Page::Layout && !self.layout_adjusted => {
                self.layout_adjusted = true;
                adjust_layout_commands()
            }
            LAYOUT_ACTION_ID if self.page == Page::Layout => {
                self.layout_adjusted = false;
                reset_layout_commands()
            }
            APPEARANCE_ACTION_ID if self.page == Page::Appearance && !self.appearance_revealed => {
                self.appearance_revealed = true;
                reveal_appearance_commands()
            }
            APPEARANCE_ACTION_ID if self.page == Page::Appearance => {
                self.appearance_revealed = false;
                reset_appearance_commands()
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
            HIERARCHY_BUTTON_ID,
            ASSETS_BUTTON_ID,
            LAYOUT_BUTTON_ID,
            APPEARANCE_BUTTON_ID,
        ))
        .child(components::canvas(CANVAS_ID, PAGE_ID, LABEL_COMPONENT_ID));
    Snapshot::new(
        session_id,
        asset_catalog::ASSET_CATALOG.to_vec(),
        vec![Scene::new(SCENE_ID, ui_assets::CONTENT.clone())],
        vec![camera],
        CAMERA_ID,
    )
    .ui_document(ui)
}

fn navigation_commands(page: Page) -> Vec<ParallelCommandGroup<Command>> {
    let content = match page {
        Page::Components => components::components_page(PAGE_ID, LABEL_COMPONENT_ID),
        Page::Interactions => components::interactions_page(PAGE_ID, CALLBACK_BUTTON_ID),
        Page::Hierarchy => components::hierarchy_page(PAGE_ID, &hierarchy_ids()),
        Page::Assets => components::assets_page(PAGE_ID, &asset_ids()),
        Page::Layout => components::layout_page(PAGE_ID, &layout_ids()),
        Page::Appearance => components::appearance_page(PAGE_ID, &appearance_ids()),
    };
    let components_active = page == Page::Components;
    let interactions_active = page == Page::Interactions;
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
                Button::default().style(design_system::navigation_item(interactions_active)),
            ),
            Command::update_visual_element(
                HIERARCHY_BUTTON_ID,
                Button::default().style(design_system::navigation_item(page == Page::Hierarchy)),
            ),
            Command::update_visual_element(
                ASSETS_BUTTON_ID,
                Button::default().style(design_system::navigation_item(page == Page::Assets)),
            ),
            Command::update_visual_element(
                LAYOUT_BUTTON_ID,
                Button::default().style(design_system::navigation_item(page == Page::Layout)),
            ),
            Command::update_visual_element(
                APPEARANCE_BUTTON_ID,
                Button::default().style(design_system::navigation_item(page == Page::Appearance)),
            ),
        ]),
    ]
}

fn reveal_appearance_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(
            APPEARANCE_HIDDEN_ID,
            Box::default().style(appearance_styles::visible()),
        ),
        Command::update_visual_element(
            APPEARANCE_REMOVED_ID,
            Box::default().style(appearance_styles::present()),
        ),
        Command::update_visual_element(APPEARANCE_ACTION_ID, Button::new("Reset visibility")),
    ])]
}

fn reset_appearance_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(
            APPEARANCE_HIDDEN_ID,
            Box::default().style(appearance_styles::hidden()),
        ),
        Command::update_visual_element(
            APPEARANCE_REMOVED_ID,
            Box::default().style(appearance_styles::removed()),
        ),
        Command::update_visual_element(APPEARANCE_ACTION_ID, Button::new("Show visibility")),
    ])]
}

fn appearance_ids() -> components::AppearanceIds {
    components::AppearanceIds {
        square: APPEARANCE_SQUARE_ID,
        rounded: APPEARANCE_ROUNDED_ID,
        sliced: APPEARANCE_SLICED_ID,
        opacity: APPEARANCE_OPACITY_ID,
        clipped: APPEARANCE_CLIPPED_ID,
        hidden: APPEARANCE_HIDDEN_ID,
        removed: APPEARANCE_REMOVED_ID,
        action: APPEARANCE_ACTION_ID,
    }
}

fn adjust_layout_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(
            LAYOUT_PLAYGROUND_ID,
            Box::default().style(layout_styles::column_playground()),
        ),
        Command::update_visual_element(
            LAYOUT_ALPHA_ID,
            Label::default().style(layout_styles::column_item()),
        ),
        Command::update_visual_element(
            LAYOUT_BETA_ID,
            Label::default().style(layout_styles::column_item()),
        ),
        Command::update_visual_element(
            LAYOUT_GAMMA_ID,
            Label::default().style(layout_styles::absolute_item()),
        ),
        Command::update_visual_element(LAYOUT_ACTION_ID, Button::new("Reset layout")),
    ])]
}

fn reset_layout_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(
            LAYOUT_PLAYGROUND_ID,
            Box::default().style(layout_styles::playground()),
        ),
        Command::update_visual_element(
            LAYOUT_ALPHA_ID,
            Label::default().style(layout_styles::item()),
        ),
        Command::update_visual_element(
            LAYOUT_BETA_ID,
            Label::default().style(layout_styles::item()),
        ),
        Command::update_visual_element(
            LAYOUT_GAMMA_ID,
            Label::default().style(layout_styles::item()),
        ),
        Command::update_visual_element(LAYOUT_ACTION_ID, Button::new("Column layout")),
    ])]
}

fn layout_ids() -> components::LayoutIds {
    components::LayoutIds {
        playground: LAYOUT_PLAYGROUND_ID,
        alpha: LAYOUT_ALPHA_ID,
        beta: LAYOUT_BETA_ID,
        gamma: LAYOUT_GAMMA_ID,
        action: LAYOUT_ACTION_ID,
    }
}

fn switch_source_commands(sprite_active: bool) -> Vec<ParallelCommandGroup<Command>> {
    let (image, address, action) = if sprite_active {
        (
            Image::new().source(assets::SPRITE.clone()),
            assets::SPRITE.as_str().to_owned(),
            "Show texture",
        )
    } else {
        (
            Image::new().source(assets::TEXTURE.clone()),
            assets::TEXTURE.as_str().to_owned(),
            "Show sprite",
        )
    };
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(SWITCHED_IMAGE_ID, image),
        Command::update_visual_element(ACTIVE_ADDRESS_ID, Label::new(address)),
        Command::update_visual_element(SOURCE_SWITCH_ID, Button::new(action)),
    ])]
}

fn asset_ids() -> components::AssetIds {
    components::AssetIds {
        texture: TEXTURE_IMAGE_ID,
        sprite: SPRITE_IMAGE_ID,
        vector: VECTOR_IMAGE_ID,
        render_texture: RENDER_IMAGE_ID,
        switched: SWITCHED_IMAGE_ID,
        active_address: ACTIVE_ADDRESS_ID,
        switch_action: SOURCE_SWITCH_ID,
    }
}

fn apply_hierarchy_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![
        ParallelCommandGroup::new(vec![Command::update_visual_element_index(
            HIERARCHY_SECONDARY_ID,
            0,
        )]),
        ParallelCommandGroup::new(vec![Command::update_visual_element(
            HIERARCHY_PRIMARY_ID,
            Label::default()
                .enabled(false)
                .picking_mode(PickingMode::Ignore)
                .class("changed"),
        )]),
        ParallelCommandGroup::new(vec![Command::update_visual_element_parent(
            HIERARCHY_MOVABLE_ID,
            HIERARCHY_DESTINATION_ID,
        )]),
        ParallelCommandGroup::new(vec![
            Command::update_visual_element(
                HIERARCHY_BRANCH_ID,
                Box::default().delegates_focus(false),
            ),
            Command::update_visual_element(HIERARCHY_ACTION_ID, Button::new("Reset")),
        ]),
    ]
}

fn reset_hierarchy_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![
        ParallelCommandGroup::new(vec![Command::update_visual_element_parent(
            HIERARCHY_MOVABLE_ID,
            HIERARCHY_BRANCH_ID,
        )]),
        ParallelCommandGroup::new(vec![Command::update_visual_element_index(
            HIERARCHY_PRIMARY_ID,
            0,
        )]),
        ParallelCommandGroup::new(vec![Command::update_visual_element(
            HIERARCHY_PRIMARY_ID,
            Label::default()
                .enabled(true)
                .picking_mode(PickingMode::Position)
                .class("ready"),
        )]),
        ParallelCommandGroup::new(vec![
            Command::update_visual_element(
                HIERARCHY_BRANCH_ID,
                Box::default().delegates_focus(true),
            ),
            Command::update_visual_element(HIERARCHY_ACTION_ID, Button::new("Reorder children")),
        ]),
    ]
}

fn hierarchy_ids() -> components::HierarchyIds {
    components::HierarchyIds {
        branch: HIERARCHY_BRANCH_ID,
        primary: HIERARCHY_PRIMARY_ID,
        secondary: HIERARCHY_SECONDARY_ID,
        movable: HIERARCHY_MOVABLE_ID,
        destination: HIERARCHY_DESTINATION_ID,
        action: HIERARCHY_ACTION_ID,
    }
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
