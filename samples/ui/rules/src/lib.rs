//! Native Rust engine for the standalone Battlement UI lab.

use battlement::{
    ActionBody, BackgroundSource, Batch, BatchId, Box, Button, CameraState, ClientMessage, Color,
    Command, Connect, CoreErrorCode, GameObject, GroupBox, Image, Label, ObjectId, PanelScaleMode,
    PanelSettings, ParallelCommandGroup, ParentScene, PickingMode, Response, Scene, SceneId,
    ScreenSize, SessionId, Snapshot, Style, UiDocument, UiEventBody, UiNode, object_id, scene_id,
};
use battlement_native::{Engine, EngineError};

#[path = "assets.rs"]
pub mod asset_catalog;

mod appearance_styles;
mod asset_styles;
mod background_styles;
mod boolean_components;
mod boolean_styles;
mod button_styles;
mod choice_group_components;
mod choice_group_styles;
mod complex_part_components;
mod complex_part_styles;
mod component_styles;
mod components;
mod container_components;
mod container_styles;
mod design_system;
mod dropdown_components;
mod dropdown_styles;
mod hierarchy_styles;
mod interaction_styles;
mod layout_styles;
mod navigation;
mod part_components;
mod part_styles;
mod pointer_routing_components;
mod pointer_routing_styles;
mod range_components;
mod range_styles;
mod routing;
mod scroll_components;
mod scroll_styles;
mod slider_components;
mod slider_styles;
mod tab_components;
mod tab_styles;
mod text_field_components;
mod text_field_styles;
mod transform_styles;
mod typography_styles;

use crate::asset_catalog::ui::{self as ui_assets, assets};
use crate::routing::Page;

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
const BACKGROUNDS_BUTTON_ID: ObjectId = object_id!("bbcd4be5-d6f3-46c3-8605-56fd4669eda0");
const BACKGROUND_TEXTURE_ID: ObjectId = object_id!("f7220234-b7ae-4dc1-adda-8b360959c718");
const BACKGROUND_SPRITE_ID: ObjectId = object_id!("e8209c63-12d6-4dcb-b225-2418727d02d6");
const BACKGROUND_VECTOR_ID: ObjectId = object_id!("f0612329-0788-46ad-a2cb-62243fd041c3");
const BACKGROUND_RENDER_ID: ObjectId = object_id!("3479b397-ae71-4b0e-8cdf-d43fd68449db");
const BACKGROUND_CURSOR_PREVIEW_ID: ObjectId = object_id!("8a5bce3d-d8a5-4e3a-8b50-aa6f70f40b63");
const BACKGROUND_ACTION_ID: ObjectId = object_id!("62f5c910-67fa-4eb1-b54b-040022f63ab7");
const TRANSFORMS_BUTTON_ID: ObjectId = object_id!("416cc818-7d31-4d01-8e39-712be437494b");
const TRANSFORM_TARGET_ID: ObjectId = object_id!("066af04d-a6d7-46e1-b7ac-a62001a90239");
const TRANSFORM_STATUS_ID: ObjectId = object_id!("6274737d-8539-4991-ad00-a20b3a5a9fc2");
const TRANSFORM_ACTION_ID: ObjectId = object_id!("6277a6b7-b774-4302-9d06-81c1991c214f");
const TYPOGRAPHY_BUTTON_ID: ObjectId = object_id!("879be431-2981-4aa0-8094-603f106bf067");
const BUTTONS_BUTTON_ID: ObjectId = object_id!("b39e6ba8-aa92-4bc5-b52e-acde2cab1c3a");
const ORDINARY_BUTTON_ID: ObjectId = object_id!("4dd42b67-17e4-4a57-aaca-9716957aa8e0");
const ICON_BUTTON_ID: ObjectId = object_id!("ba3842ad-55f5-4ef9-b4bf-3918ac59c9e2");
const DISABLED_BUTTON_ID: ObjectId = object_id!("10e790f4-8ff9-43e0-9d50-89e7c995140c");
const NAVIGATION_BUTTON_ID: ObjectId = object_id!("5d24f2cb-6aae-469a-bf10-ae73331d95da");
const REPEAT_BUTTON_ID: ObjectId = object_id!("569f7875-a10a-428f-a727-960a0897fbd9");
const REPEAT_COUNTER_ID: ObjectId = object_id!("2510be34-7e20-4a3e-81ec-c3a48ec68ce0");
const BUTTON_STATUS_ID: ObjectId = object_id!("d96ccf2a-04ed-4de7-90b6-d5c0d43b2d62");
const CONTAINERS_BUTTON_ID: ObjectId = object_id!("b3858e8c-0b75-4c55-b5f1-d2e0a18cf1ef");
const TITLED_GROUP_ID: ObjectId = object_id!("9ab84d41-dd5f-4202-a62b-da4643222ac8");
const EMPTY_GROUP_ID: ObjectId = object_id!("05acfc99-c92d-46cd-93cd-3738ff025e62");
const DYNAMIC_GROUP_ID: ObjectId = object_id!("3a9d57df-b920-4ec3-b170-3afbc6ce0494");
const DYNAMIC_GROUP_CHILD_ID: ObjectId = object_id!("7ceac51e-b580-4e67-b995-191216cbff88");
const DYNAMIC_GROUP_ACTION_ID: ObjectId = object_id!("c21e285f-6999-4df7-8a6b-559339520962");
const POPUP_WINDOW_ID: ObjectId = object_id!("71347582-7a69-4270-a76f-c4c25546e086");
const SCROLL_BUTTON_ID: ObjectId = object_id!("b4baa362-1979-4bff-ae2d-d6a736ab4bb4");
const TABS_BUTTON_ID: ObjectId = object_id!("0dbf590c-b821-4ba5-b4a7-426382a96a16");
const TEXT_FIELDS_BUTTON_ID: ObjectId = object_id!("d1810adf-f4fa-4eb7-8b44-46d60e22341d");
const BOOLEAN_CONTROLS_BUTTON_ID: ObjectId = object_id!("b95de403-9b85-44a2-aebe-acd016c92fa6");
const CHOICE_GROUPS_BUTTON_ID: ObjectId = object_id!("bf246175-3572-4a9d-bd1b-fc91946f035e");
const DROPDOWNS_BUTTON_ID: ObjectId = object_id!("feae3645-8809-42f3-b4f6-00afe473b2f4");
const SLIDERS_BUTTON_ID: ObjectId = object_id!("581694e0-ad9e-477d-a776-478169f39c45");
const RANGES_BUTTON_ID: ObjectId = object_id!("69c28345-59e0-4d2c-a374-b302421d3713");
const PARTS_BUTTON_ID: ObjectId = object_id!("cbb9c6db-5248-48db-b150-029776faf162");
const COMPLEX_PARTS_BUTTON_ID: ObjectId = object_id!("8da1d1bd-f7a9-420b-a122-f5c75ca3b295");
const COMPLEX_PARTS_TOGGLE_ID: ObjectId = object_id!("9321c5a3-9b82-462d-9f68-26da56edcbb7");
const POINTER_ROUTING_BUTTON_ID: ObjectId = object_id!("8be537d2-16e7-47ee-9a50-31cd36a13522");

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
    background_adjusted: bool,
    transform_settled: bool,
    repeat_count: u32,
    container_title_visible: bool,
    complex_parts_revealed: bool,
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
        background_adjusted: false,
        transform_settled: false,
        repeat_count: 0,
        container_title_visible: false,
        complex_parts_revealed: false,
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
        self.background_adjusted = false;
        self.transform_settled = false;
        self.repeat_count = 0;
        self.container_title_visible = false;
        self.complex_parts_revealed = false;
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
        if event.target_id == TRANSFORM_TARGET_ID && self.page == Page::Transforms {
            let command = match &event.body {
                UiEventBody::TransitionStart(_) => Some(Command::update_visual_element(
                    TRANSFORM_STATUS_ID,
                    Label::new("Running"),
                )),
                UiEventBody::TransitionEnd(value) => Some(Command::update_visual_element(
                    TRANSFORM_STATUS_ID,
                    Label::new(if self.transform_settled {
                        transition_summary(value)
                    } else {
                        "Ready".to_owned()
                    }),
                )),
                UiEventBody::TransitionCancel(_) => Some(Command::update_visual_element(
                    TRANSFORM_STATUS_ID,
                    Label::new("Cancelled"),
                )),
                _ => None,
            };
            if let Some(command) = command {
                return Ok(Response::batch(
                    Batch::new(
                        BatchId::new_v4(),
                        self.session_id,
                        vec![ParallelCommandGroup::new(vec![command])],
                    )
                    .caused_by_action_id(action.action_id),
                ));
            }
        }
        if self.page == Page::Scroll
            && let Some(commands) = scroll_components::event_commands(&event)
        {
            return Ok(routing::single_ui_command_response(
                self.session_id,
                action.action_id,
                commands,
            ));
        }
        if self.page == Page::Tabs
            && let Some(commands) = tab_components::event_commands(&event)
        {
            return Ok(routing::single_ui_command_response(
                self.session_id,
                action.action_id,
                commands,
            ));
        }
        if self.page == Page::TextFields
            && let Some(commands) = text_field_components::event_commands(&event)
        {
            return Ok(routing::single_ui_command_response(
                self.session_id,
                action.action_id,
                commands,
            ));
        }
        if self.page == Page::BooleanControls
            && let Some(commands) = boolean_components::event_commands(&event)
        {
            return Ok(routing::single_ui_command_response(
                self.session_id,
                action.action_id,
                commands,
            ));
        }
        if self.page == Page::ChoiceGroups
            && let Some(commands) = choice_group_components::event_commands(&event)
        {
            return Ok(routing::single_ui_command_response(
                self.session_id,
                action.action_id,
                commands,
            ));
        }
        if self.page == Page::Dropdowns
            && let Some(commands) = dropdown_components::event_commands(&event)
        {
            return Ok(routing::single_ui_command_response(
                self.session_id,
                action.action_id,
                commands,
            ));
        }
        if self.page == Page::Sliders
            && let Some(commands) = slider_components::event_commands(&event)
        {
            return Ok(routing::single_ui_command_response(
                self.session_id,
                action.action_id,
                commands,
            ));
        }
        if self.page == Page::Ranges
            && let Some(commands) = range_components::event_commands(&event)
        {
            return Ok(routing::single_ui_command_response(
                self.session_id,
                action.action_id,
                commands,
            ));
        }
        if self.page == Page::PointerRouting
            && let Some(commands) = pointer_routing_components::event_commands(&event)
        {
            return Ok(routing::single_ui_command_response(
                self.session_id,
                action.action_id,
                commands,
            ));
        }
        let UiEventBody::Click(click) = event.body else {
            return Ok(Response::empty(self.session_id));
        };
        let commands = match event.target_id {
            COMPONENTS_BUTTON_ID if self.page != Page::Components => {
                self.page = Page::Components;
                self.greeting_visible = false;
                navigation::commands(Page::Components)
            }
            INTERACTIONS_BUTTON_ID if self.page != Page::Interactions => {
                self.page = Page::Interactions;
                self.greeting_visible = false;
                navigation::commands(Page::Interactions)
            }
            HIERARCHY_BUTTON_ID if self.page != Page::Hierarchy => {
                self.page = Page::Hierarchy;
                self.greeting_visible = false;
                self.hierarchy_applied = false;
                navigation::commands(Page::Hierarchy)
            }
            ASSETS_BUTTON_ID if self.page != Page::Assets => {
                self.page = Page::Assets;
                self.greeting_visible = false;
                self.sprite_source_active = false;
                navigation::commands(Page::Assets)
            }
            LAYOUT_BUTTON_ID if self.page != Page::Layout => {
                self.page = Page::Layout;
                self.greeting_visible = false;
                self.layout_adjusted = false;
                navigation::commands(Page::Layout)
            }
            APPEARANCE_BUTTON_ID if self.page != Page::Appearance => {
                self.page = Page::Appearance;
                self.greeting_visible = false;
                self.appearance_revealed = false;
                navigation::commands(Page::Appearance)
            }
            BACKGROUNDS_BUTTON_ID if self.page != Page::Backgrounds => {
                self.page = Page::Backgrounds;
                self.greeting_visible = false;
                self.background_adjusted = false;
                navigation::commands(Page::Backgrounds)
            }
            TRANSFORMS_BUTTON_ID if self.page != Page::Transforms => {
                self.page = Page::Transforms;
                self.greeting_visible = false;
                self.transform_settled = false;
                navigation::commands(Page::Transforms)
            }
            TYPOGRAPHY_BUTTON_ID if self.page != Page::Typography => {
                self.page = Page::Typography;
                self.greeting_visible = false;
                navigation::commands(Page::Typography)
            }
            BUTTONS_BUTTON_ID if self.page != Page::Buttons => {
                self.page = Page::Buttons;
                self.greeting_visible = false;
                self.repeat_count = 0;
                navigation::commands(Page::Buttons)
            }
            CONTAINERS_BUTTON_ID if self.page != Page::Containers => {
                self.page = Page::Containers;
                self.greeting_visible = false;
                self.container_title_visible = false;
                navigation::commands(Page::Containers)
            }
            SCROLL_BUTTON_ID if self.page != Page::Scroll => {
                self.page = Page::Scroll;
                self.greeting_visible = false;
                navigation::commands(Page::Scroll)
            }
            TABS_BUTTON_ID if self.page != Page::Tabs => {
                self.page = Page::Tabs;
                self.greeting_visible = false;
                navigation::commands(Page::Tabs)
            }
            TEXT_FIELDS_BUTTON_ID if self.page != Page::TextFields => {
                self.page = Page::TextFields;
                self.greeting_visible = false;
                navigation::commands(Page::TextFields)
            }
            BOOLEAN_CONTROLS_BUTTON_ID if self.page != Page::BooleanControls => {
                self.page = Page::BooleanControls;
                self.greeting_visible = false;
                navigation::commands(Page::BooleanControls)
            }
            CHOICE_GROUPS_BUTTON_ID if self.page != Page::ChoiceGroups => {
                self.page = Page::ChoiceGroups;
                self.greeting_visible = false;
                navigation::commands(Page::ChoiceGroups)
            }
            DROPDOWNS_BUTTON_ID if self.page != Page::Dropdowns => {
                self.page = Page::Dropdowns;
                self.greeting_visible = false;
                navigation::commands(Page::Dropdowns)
            }
            SLIDERS_BUTTON_ID if self.page != Page::Sliders => {
                self.page = Page::Sliders;
                self.greeting_visible = false;
                navigation::commands(Page::Sliders)
            }
            RANGES_BUTTON_ID if self.page != Page::Ranges => {
                self.page = Page::Ranges;
                self.greeting_visible = false;
                navigation::commands(Page::Ranges)
            }
            PARTS_BUTTON_ID if self.page != Page::Parts => {
                self.page = Page::Parts;
                self.greeting_visible = false;
                navigation::commands(Page::Parts)
            }
            COMPLEX_PARTS_BUTTON_ID if self.page != Page::ComplexParts => {
                self.page = Page::ComplexParts;
                self.greeting_visible = false;
                self.complex_parts_revealed = false;
                navigation::commands(Page::ComplexParts)
            }
            POINTER_ROUTING_BUTTON_ID if self.page != Page::PointerRouting => {
                self.page = Page::PointerRouting;
                self.greeting_visible = false;
                navigation::commands(Page::PointerRouting)
            }
            COMPLEX_PARTS_TOGGLE_ID if self.page == Page::ComplexParts => {
                self.complex_parts_revealed = !self.complex_parts_revealed;
                vec![ParallelCommandGroup::new(
                    complex_part_components::update_commands(
                        COMPLEX_PARTS_TOGGLE_ID,
                        self.complex_parts_revealed,
                    ),
                )]
            }
            ORDINARY_BUTTON_ID if self.page == Page::Buttons => {
                button_status_commands("Pointer command submitted once")
            }
            ICON_BUTTON_ID if self.page == Page::Buttons => {
                button_status_commands("Prepared vector icon command submitted")
            }
            NAVIGATION_BUTTON_ID if self.page == Page::Buttons => {
                button_status_commands(match click {
                    battlement::ClickEvent::NavigationSubmit => {
                        "Navigation submit won Click precedence"
                    }
                    battlement::ClickEvent::Pointer { .. } => "Pointer command submitted once",
                    battlement::ClickEvent::Repeat => "Unexpected repeat activation",
                })
            }
            REPEAT_BUTTON_ID if self.page == Page::Buttons => {
                self.repeat_count += 1;
                repeat_commands(self.repeat_count)
            }
            DYNAMIC_GROUP_ACTION_ID if self.page == Page::Containers => {
                self.container_title_visible = !self.container_title_visible;
                container_title_commands(self.container_title_visible)
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
            BACKGROUND_ACTION_ID if self.page == Page::Backgrounds && !self.background_adjusted => {
                self.background_adjusted = true;
                adjust_background_commands()
            }
            BACKGROUND_ACTION_ID if self.page == Page::Backgrounds => {
                self.background_adjusted = false;
                reset_background_commands()
            }
            TRANSFORM_ACTION_ID if self.page == Page::Transforms && !self.transform_settled => {
                self.transform_settled = true;
                settle_transform_commands()
            }
            TRANSFORM_ACTION_ID if self.page == Page::Transforms => {
                self.transform_settled = false;
                reset_transform_commands()
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
        .child(components::navigation(&navigation_ids()))
        .child(components::canvas(CANVAS_ID, PAGE_ID, LABEL_COMPONENT_ID));
    Snapshot::new(
        session_id,
        asset_catalog::ASSET_CATALOG.to_vec(),
        vec![Scene::new(SCENE_ID, ui_assets::CONTENT.clone())],
        vec![camera],
        CAMERA_ID,
    )
    .ui_document_with(ui, ParentScene::Persistent, |state| {
        state.panel_settings(
            PanelSettings::new()
                .scale_mode(PanelScaleMode::ScaleWithScreenSize)
                .reference_resolution(ScreenSize::new(1280, 720))
                .match_factor(0.5),
        )
    })
}

fn settle_transform_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(
            TRANSFORM_TARGET_ID,
            Box::default().style(transform_styles::transition_settled()),
        ),
        Command::update_visual_element(TRANSFORM_STATUS_ID, Label::new("Running")),
        Command::update_visual_element(TRANSFORM_ACTION_ID, Button::new("Reset")),
    ])]
}

fn reset_transform_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(
            TRANSFORM_TARGET_ID,
            Box::default().style(transform_styles::transition_initial()),
        ),
        Command::update_visual_element(TRANSFORM_STATUS_ID, Label::new("Resetting")),
        Command::update_visual_element(TRANSFORM_ACTION_ID, Button::new("Launch")),
    ])]
}

fn transform_ids() -> components::TransformIds {
    components::TransformIds {
        target: TRANSFORM_TARGET_ID,
        status: TRANSFORM_STATUS_ID,
        action: TRANSFORM_ACTION_ID,
    }
}

fn navigation_ids() -> components::NavigationIds {
    components::NavigationIds {
        components: COMPONENTS_BUTTON_ID,
        interactions: INTERACTIONS_BUTTON_ID,
        hierarchy: HIERARCHY_BUTTON_ID,
        assets: ASSETS_BUTTON_ID,
        layout: LAYOUT_BUTTON_ID,
        appearance: APPEARANCE_BUTTON_ID,
        backgrounds: BACKGROUNDS_BUTTON_ID,
        transforms: TRANSFORMS_BUTTON_ID,
        typography: TYPOGRAPHY_BUTTON_ID,
        buttons: BUTTONS_BUTTON_ID,
        containers: CONTAINERS_BUTTON_ID,
        scroll: SCROLL_BUTTON_ID,
        tabs: TABS_BUTTON_ID,
        text_fields: TEXT_FIELDS_BUTTON_ID,
        boolean_controls: BOOLEAN_CONTROLS_BUTTON_ID,
        choice_groups: CHOICE_GROUPS_BUTTON_ID,
        dropdowns: DROPDOWNS_BUTTON_ID,
        sliders: SLIDERS_BUTTON_ID,
        ranges: RANGES_BUTTON_ID,
        parts: PARTS_BUTTON_ID,
        complex_parts: COMPLEX_PARTS_BUTTON_ID,
        pointer_routing: POINTER_ROUTING_BUTTON_ID,
    }
}

fn container_ids() -> container_components::ContainerIds {
    container_components::ContainerIds {
        titled_group: TITLED_GROUP_ID,
        empty_group: EMPTY_GROUP_ID,
        dynamic_group: DYNAMIC_GROUP_ID,
        dynamic_child: DYNAMIC_GROUP_CHILD_ID,
        dynamic_action: DYNAMIC_GROUP_ACTION_ID,
        popup: POPUP_WINDOW_ID,
    }
}

fn container_title_commands(visible: bool) -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(
            DYNAMIC_GROUP_ID,
            GroupBox::new().text(if visible { "TACTICAL OVERRIDES" } else { "" }),
        ),
        Command::update_visual_element(
            DYNAMIC_GROUP_ACTION_ID,
            Button::new(if visible { "Remove title" } else { "Add title" }),
        ),
        Command::update_visual_element(
            DYNAMIC_GROUP_CHILD_ID,
            Label::new(if visible {
                "Title created; authored content stayed in place."
            } else {
                "No internal title label; content stays mounted."
            }),
        ),
    ])]
}

fn button_ids() -> components::ButtonIds {
    components::ButtonIds {
        ordinary: ORDINARY_BUTTON_ID,
        icon: ICON_BUTTON_ID,
        disabled: DISABLED_BUTTON_ID,
        navigation: NAVIGATION_BUTTON_ID,
        repeat: REPEAT_BUTTON_ID,
        counter: REPEAT_COUNTER_ID,
        status: BUTTON_STATUS_ID,
    }
}

fn button_status_commands(message: &str) -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(BUTTON_STATUS_ID, Label::new(message)),
    ])]
}

fn repeat_commands(count: u32) -> Vec<ParallelCommandGroup<Command>> {
    let mut commands = vec![
        Command::update_visual_element(REPEAT_COUNTER_ID, Label::new(count.to_string())),
        Command::update_visual_element(
            BUTTON_STATUS_ID,
            Label::new(format!("Repeat callback {count} | release adds no click")),
        ),
    ];
    if count == 4 {
        commands.push(Command::update_visual_element(
            REPEAT_BUTTON_ID,
            battlement::RepeatButton::default().timing(
                200,
                std::num::NonZeroU32::new(100).expect("constant interval is positive"),
            ),
        ));
    }
    vec![ParallelCommandGroup::new(commands)]
}

fn transition_summary(value: &battlement::TransitionEvent) -> String {
    let properties = value
        .properties
        .iter()
        .filter_map(|property| match property {
            battlement::TransitionProperty::Rotate => Some("Rotate"),
            battlement::TransitionProperty::Scale => Some("Scale"),
            battlement::TransitionProperty::Translate => Some("Translate"),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{properties} {:.0} ms", value.elapsed_ms)
}

fn adjust_background_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(
            BACKGROUND_TEXTURE_ID,
            Box::default().style(background_styles::adjusted(
                BackgroundSource::RenderTexture(assets::RENDER_TEXTURE.clone()),
            )),
        ),
        Command::update_visual_element(BACKGROUND_ACTION_ID, Button::new("Reset")),
    ])]
}

fn reset_background_commands() -> Vec<ParallelCommandGroup<Command>> {
    vec![ParallelCommandGroup::new(vec![
        Command::update_visual_element(
            BACKGROUND_TEXTURE_ID,
            Box::default().style(background_styles::interactive(
                BackgroundSource::Texture(assets::TEXTURE.clone()),
                assets::CURSOR.clone(),
            )),
        ),
        Command::update_visual_element(BACKGROUND_ACTION_ID, Button::new("Apply")),
    ])]
}

fn background_ids() -> components::BackgroundIds {
    components::BackgroundIds {
        texture: BACKGROUND_TEXTURE_ID,
        sprite: BACKGROUND_SPRITE_ID,
        vector: BACKGROUND_VECTOR_ID,
        render_texture: BACKGROUND_RENDER_ID,
        cursor_preview: BACKGROUND_CURSOR_PREVIEW_ID,
        action: BACKGROUND_ACTION_ID,
    }
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
