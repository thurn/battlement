use battlement::{
    BackgroundPositionKeyword, BackgroundRepeatMode, BackgroundSize, BackgroundSource, Color,
    Cursor, Display, FlexDirection, FlexWrap, ImageSource, ObjectId, Overflow, Position,
    StyleValue, UiElementKind, Visibility, object_id,
};
use battlement_fake::{
    assets::FakeAssetCatalog,
    client::{FakeClient, UiClient},
};
use battlement_rules::asset_catalog::ui::{self as ui_assets, assets};

const INTERACTIONS_BUTTON_ID: ObjectId = object_id!("4969d46f-c28c-4e5d-85a0-0321f9931f89");
const CALLBACK_BUTTON_ID: ObjectId = object_id!("7e0b078e-13d9-43c3-a491-84178e157fb2");
const LABEL_COMPONENT_ID: ObjectId = object_id!("5768cfee-a137-49c0-b76c-5ebfa6c227c1");
const GREETING_ID: ObjectId = object_id!("2d8ac61c-49bb-43ce-9656-faa11238351f");
const TRANSIENT_CARD_ID: ObjectId = object_id!("45a1a00c-2624-4e40-b675-3c5f59c62f53");
const COMPONENTS_BUTTON_ID: ObjectId = object_id!("0e95fbc2-b5e9-4e0f-937f-86aab38b6855");
const HIERARCHY_BUTTON_ID: ObjectId = object_id!("02e0f324-4781-4301-9502-93435d7eea7e");
const HIERARCHY_BRANCH_ID: ObjectId = object_id!("53e9582f-36c9-47fb-91c7-a6f7c7b3dd50");
const HIERARCHY_PRIMARY_ID: ObjectId = object_id!("f48e306d-ec3a-4881-abeb-ae685b0bb956");
const HIERARCHY_SECONDARY_ID: ObjectId = object_id!("45ee68d7-72bf-4d1b-bba3-e0a2834c5f06");
const HIERARCHY_MOVABLE_ID: ObjectId = object_id!("0121bbc8-ceb1-42ea-bea0-a7601543851e");
const HIERARCHY_DESTINATION_ID: ObjectId = object_id!("98ec6daa-7faa-41aa-a157-afb9beca284d");
const HIERARCHY_ACTION_ID: ObjectId = object_id!("51e73f5f-1af1-4f54-bcf6-288cde0f45ee");
const PAGE_ID: ObjectId = object_id!("28951e4f-6f61-491e-8548-84b9d4a356e4");
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
const LAYOUT_GAMMA_ID: ObjectId = object_id!("3dbc8a14-b4b2-42b5-83f0-f83f564dadc4");
const LAYOUT_ACTION_ID: ObjectId = object_id!("274aa2af-5b70-4079-a260-25fadd46f339");
const APPEARANCE_BUTTON_ID: ObjectId = object_id!("7237e7ab-178f-438e-a457-0106b1899f6d");
const APPEARANCE_SLICED_ID: ObjectId = object_id!("2b6868b0-042c-4258-b7fe-d594c788cf5d");
const APPEARANCE_CLIPPED_ID: ObjectId = object_id!("1da43df8-2db8-4975-b6a7-2f84abb9f5ae");
const APPEARANCE_HIDDEN_ID: ObjectId = object_id!("3658659b-69e6-4c1e-bf96-6ba1473d0ac2");
const APPEARANCE_REMOVED_ID: ObjectId = object_id!("f2360cdc-c121-41af-8ae2-486eb817669f");
const APPEARANCE_ACTION_ID: ObjectId = object_id!("876cec21-9d24-40e3-ba85-f27e0262112c");
const BACKGROUNDS_BUTTON_ID: ObjectId = object_id!("bbcd4be5-d6f3-46c3-8605-56fd4669eda0");
const BACKGROUND_TEXTURE_ID: ObjectId = object_id!("f7220234-b7ae-4dc1-adda-8b360959c718");
const BACKGROUND_SPRITE_ID: ObjectId = object_id!("e8209c63-12d6-4dcb-b225-2418727d02d6");
const BACKGROUND_VECTOR_ID: ObjectId = object_id!("f0612329-0788-46ad-a2cb-62243fd041c3");
const BACKGROUND_RENDER_ID: ObjectId = object_id!("3479b397-ae71-4b0e-8cdf-d43fd68449db");
const BACKGROUND_ACTION_ID: ObjectId = object_id!("62f5c910-67fa-4eb1-b54b-040022f63ab7");

#[test]
fn ui_lab_clicks_dispatch_and_apply_all_ui_command_families() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    assert_eq!(
        client.ui().element(LABEL_COMPONENT_ID).text(),
        Some("Hello from Rust")
    );

    client.ui().click(INTERACTIONS_BUTTON_ID);
    assert_eq!(
        client.ui().element(CALLBACK_BUTTON_ID).text(),
        Some("Click to run a Rust callback")
    );

    client.ui().click(CALLBACK_BUTTON_ID);
    assert_eq!(client.ui().element(CALLBACK_BUTTON_ID).text(), Some("Hide"));
    assert!(
        client
            .ui()
            .element(GREETING_ID)
            .is_enabled()
            .unwrap_or(true)
    );
    assert!(client.ui().journal().iter().any(|entry| {
        matches!(
            entry,
            battlement_fake::battlement_ui_fake::UiJournalEntry::Destroy(id)
                if *id == TRANSIENT_CARD_ID
        )
    }));

    client.ui().click(CALLBACK_BUTTON_ID);
    assert_eq!(
        client.ui().element(CALLBACK_BUTTON_ID).text(),
        Some("Click to run a Rust callback")
    );
    assert!(client.ui().journal().iter().any(|entry| {
        matches!(
            entry,
            battlement_fake::battlement_ui_fake::UiJournalEntry::Destroy(id)
                if *id == GREETING_ID
        )
    }));
}

#[test]
fn hierarchy_explorer_applies_common_state_and_independent_placements() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(HIERARCHY_BUTTON_ID);
    {
        let ui = client.ui();
        assert_hierarchy_design_contract(&ui);
        let branch = ui.element(HIERARCHY_BRANCH_ID);
        assert_eq!(branch.name(), Some("logical-branch-a"));
        assert_eq!(branch.classes().unwrap(), ["hierarchy-branch"]);
        assert_eq!(branch.delegates_focus(), Some(true));
        assert_eq!(
            branch.document_root_id(),
            ui.element(HIERARCHY_PRIMARY_ID).document_root_id()
        );
        assert_eq!(
            branch.children(),
            [
                HIERARCHY_PRIMARY_ID,
                HIERARCHY_SECONDARY_ID,
                HIERARCHY_MOVABLE_ID,
            ]
        );
        assert_eq!(ui.element(HIERARCHY_PRIMARY_ID).tab_index(), Some(1));
        assert_eq!(
            ui.element(HIERARCHY_PRIMARY_ID).classes().unwrap(),
            ["ready"]
        );
        assert_eq!(
            ui.element(HIERARCHY_MOVABLE_ID).picking_mode(),
            Some(battlement::PickingMode::Ignore)
        );
    }

    client.ui().click(HIERARCHY_ACTION_ID);

    {
        let ui = client.ui();
        assert_hierarchy_design_contract(&ui);
        let primary = ui.element(HIERARCHY_PRIMARY_ID);
        assert_eq!(primary.is_enabled(), Some(false));
        assert_eq!(
            primary.picking_mode(),
            Some(battlement::PickingMode::Ignore)
        );
        assert_eq!(primary.classes().unwrap(), ["changed"]);
        assert_eq!(
            ui.element(HIERARCHY_BRANCH_ID).delegates_focus(),
            Some(false)
        );
        assert_eq!(
            ui.element(HIERARCHY_BRANCH_ID).children(),
            [HIERARCHY_SECONDARY_ID, HIERARCHY_PRIMARY_ID]
        );
        assert_eq!(
            ui.element(HIERARCHY_MOVABLE_ID).parent_id(),
            Some(HIERARCHY_DESTINATION_ID)
        );
        assert!(
            ui.element(HIERARCHY_DESTINATION_ID)
                .children()
                .contains(&HIERARCHY_MOVABLE_ID)
        );
        assert_eq!(ui.element(HIERARCHY_ACTION_ID).text(), Some("Reset"));
    }

    client.ui().click(HIERARCHY_ACTION_ID);

    {
        let ui = client.ui();
        assert_hierarchy_design_contract(&ui);
        let primary = ui.element(HIERARCHY_PRIMARY_ID);
        assert_eq!(primary.is_enabled(), Some(true));
        assert_eq!(
            primary.picking_mode(),
            Some(battlement::PickingMode::Position)
        );
        assert_eq!(primary.classes().unwrap(), ["ready"]);
        assert_eq!(
            ui.element(HIERARCHY_BRANCH_ID).delegates_focus(),
            Some(true)
        );
        assert_eq!(
            ui.element(HIERARCHY_BRANCH_ID).children(),
            [
                HIERARCHY_PRIMARY_ID,
                HIERARCHY_SECONDARY_ID,
                HIERARCHY_MOVABLE_ID,
            ]
        );
        assert_eq!(
            ui.element(HIERARCHY_MOVABLE_ID).parent_id(),
            Some(HIERARCHY_BRANCH_ID)
        );
        assert_eq!(
            ui.element(HIERARCHY_ACTION_ID).text(),
            Some("Reorder children")
        );
    }

    client.ui().click(COMPONENTS_BUTTON_ID);
    assert!(!client.ui().contains(HIERARCHY_BRANCH_ID));
    assert!(!client.ui().contains(HIERARCHY_MOVABLE_ID));
}

#[test]
fn addressed_gallery_switches_source_kind_and_restores_initial_state() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(ASSETS_BUTTON_ID);
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 12);
        assert_eq!(
            ui.element(TEXTURE_IMAGE_ID).image_source(),
            Some(&ImageSource::Texture(assets::TEXTURE.clone()))
        );
        assert_eq!(
            ui.element(SPRITE_IMAGE_ID).image_source(),
            Some(&ImageSource::Sprite(assets::SPRITE.clone()))
        );
        assert_eq!(
            ui.element(VECTOR_IMAGE_ID).image_source(),
            Some(&ImageSource::VectorImage(assets::VECTOR.clone()))
        );
        assert_eq!(
            ui.element(RENDER_IMAGE_ID).image_source(),
            Some(&ImageSource::RenderTexture(assets::RENDER_TEXTURE.clone()))
        );
        assert_eq!(
            ui.element(SWITCHED_IMAGE_ID).image_source(),
            Some(&ImageSource::Texture(assets::TEXTURE.clone()))
        );
        assert_eq!(
            ui.element(ACTIVE_ADDRESS_ID).text(),
            Some("ui/assets/texture")
        );
        assert_eq!(ui.element(SOURCE_SWITCH_ID).text(), Some("Show sprite"));
    }

    client.ui().click(SOURCE_SWITCH_ID);
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 12);
        assert_eq!(
            ui.element(SWITCHED_IMAGE_ID).image_source(),
            Some(&ImageSource::Sprite(assets::SPRITE.clone()))
        );
        assert_eq!(
            ui.element(ACTIVE_ADDRESS_ID).text(),
            Some("ui/assets/sprite")
        );
        assert_eq!(ui.element(SOURCE_SWITCH_ID).text(), Some("Show texture"));
    }

    client.ui().click(SOURCE_SWITCH_ID);
    let ui = client.ui();
    assert_page_design_contract(&ui, 12);
    assert_eq!(
        ui.element(SWITCHED_IMAGE_ID).image_source(),
        Some(&ImageSource::Texture(assets::TEXTURE.clone()))
    );
    assert_eq!(
        ui.element(ACTIVE_ADDRESS_ID).text(),
        Some("ui/assets/texture")
    );
    assert_eq!(ui.element(SOURCE_SWITCH_ID).text(), Some("Show sprite"));
}

#[test]
fn layout_playground_adjusts_and_restores_the_complete_authored_style() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(LAYOUT_BUTTON_ID);
    let initial_playground = client.ui().element(LAYOUT_PLAYGROUND_ID).style().clone();
    let initial_alpha = client.ui().element(LAYOUT_ALPHA_ID).style().clone();
    let initial_gamma = client.ui().element(LAYOUT_GAMMA_ID).style().clone();
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 6);
        assert_eq!(
            ui.element(LAYOUT_PLAYGROUND_ID).style().flex_direction,
            Some(StyleValue::Value(FlexDirection::Row))
        );
        assert_eq!(
            ui.element(LAYOUT_PLAYGROUND_ID).style().flex_wrap,
            Some(StyleValue::Value(FlexWrap::Wrap))
        );
        assert_eq!(ui.element(LAYOUT_ACTION_ID).text(), Some("Column layout"));
    }

    client.ui().click(LAYOUT_ACTION_ID);
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 6);
        assert_eq!(
            ui.element(LAYOUT_PLAYGROUND_ID).style().flex_direction,
            Some(StyleValue::Value(FlexDirection::ColumnReverse))
        );
        assert_eq!(
            ui.element(LAYOUT_GAMMA_ID).style().position,
            Some(StyleValue::Value(Position::Absolute))
        );
        assert_eq!(ui.element(LAYOUT_ACTION_ID).text(), Some("Reset layout"));
    }

    client.ui().click(LAYOUT_ACTION_ID);
    let ui = client.ui();
    assert_page_design_contract(&ui, 6);
    assert_eq!(
        ui.element(LAYOUT_PLAYGROUND_ID).style(),
        &initial_playground
    );
    assert_eq!(ui.element(LAYOUT_ALPHA_ID).style(), &initial_alpha);
    assert_eq!(ui.element(LAYOUT_GAMMA_ID).style(), &initial_gamma);
    assert_eq!(ui.element(LAYOUT_ACTION_ID).text(), Some("Column layout"));
}

#[test]
fn appearance_page_reveals_and_restores_visibility_states() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(APPEARANCE_BUTTON_ID);
    let initial_hidden = client.ui().element(APPEARANCE_HIDDEN_ID).style().clone();
    let initial_removed = client.ui().element(APPEARANCE_REMOVED_ID).style().clone();
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 10);
        assert_eq!(
            ui.element(APPEARANCE_CLIPPED_ID).style().overflow,
            Some(StyleValue::Value(Overflow::Hidden))
        );
        assert_eq!(
            ui.element(APPEARANCE_HIDDEN_ID).style().visibility,
            Some(StyleValue::Value(Visibility::Hidden))
        );
        assert_eq!(
            ui.element(APPEARANCE_REMOVED_ID).style().display,
            Some(StyleValue::Value(Display::None))
        );
        assert_eq!(
            ui.element(APPEARANCE_SLICED_ID).background_source(),
            Some(&battlement::BackgroundSource::Sprite(
                assets::SPRITE.clone()
            ))
        );
        assert_eq!(
            ui.element(APPEARANCE_ACTION_ID).text(),
            Some("Show visibility")
        );
    }

    client.ui().click(APPEARANCE_ACTION_ID);
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 10);
        assert_eq!(
            ui.element(APPEARANCE_HIDDEN_ID).style().visibility,
            Some(StyleValue::Value(Visibility::Visible))
        );
        assert_eq!(
            ui.element(APPEARANCE_REMOVED_ID).style().display,
            Some(StyleValue::Value(Display::Flex))
        );
        assert_eq!(
            ui.element(APPEARANCE_ACTION_ID).text(),
            Some("Reset visibility")
        );
    }

    client.ui().click(APPEARANCE_ACTION_ID);
    let ui = client.ui();
    assert_page_design_contract(&ui, 10);
    assert_eq!(ui.element(APPEARANCE_HIDDEN_ID).style(), &initial_hidden);
    assert_eq!(ui.element(APPEARANCE_REMOVED_ID).style(), &initial_removed);
    assert_eq!(
        ui.element(APPEARANCE_ACTION_ID).text(),
        Some("Show visibility")
    );
}

#[test]
fn background_lab_exercises_native_modes_and_restores_the_complete_style() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(BACKGROUNDS_BUTTON_ID);
    let initial = client.ui().element(BACKGROUND_TEXTURE_ID).style().clone();
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 28);
        assert_eq!(
            ui.element(BACKGROUND_TEXTURE_ID).background_source(),
            Some(&BackgroundSource::Texture(assets::TEXTURE.clone()))
        );
        assert_eq!(
            ui.element(BACKGROUND_SPRITE_ID).background_source(),
            Some(&BackgroundSource::Sprite(assets::SPRITE.clone()))
        );
        assert_eq!(
            ui.element(BACKGROUND_VECTOR_ID).background_source(),
            Some(&BackgroundSource::VectorImage(assets::VECTOR.clone()))
        );
        assert_eq!(
            ui.element(BACKGROUND_RENDER_ID).background_source(),
            Some(&BackgroundSource::RenderTexture(
                assets::RENDER_TEXTURE.clone()
            ))
        );
        let texture = ui.element(BACKGROUND_TEXTURE_ID).style();
        assert!(matches!(
            texture.background_position_x,
            Some(StyleValue::Value(value)) if value.keyword == BackgroundPositionKeyword::Left
        ));
        assert!(matches!(
            texture.background_repeat,
            Some(StyleValue::Value(value))
                if value.x == BackgroundRepeatMode::Repeat
                    && value.y == BackgroundRepeatMode::NoRepeat
        ));
        assert_eq!(
            texture.background_size,
            Some(StyleValue::Value(BackgroundSize::Auto))
        );
        assert!(matches!(
            texture.cursor,
            Some(StyleValue::Value(Cursor::Texture { ref address, .. }))
                if address == &assets::CURSOR
        ));
    }

    client.ui().click(BACKGROUND_ACTION_ID);
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 28);
        let adjusted = ui.element(BACKGROUND_TEXTURE_ID);
        assert_eq!(
            adjusted.background_source(),
            Some(&BackgroundSource::RenderTexture(
                assets::RENDER_TEXTURE.clone()
            ))
        );
        assert_eq!(
            adjusted.style().background_size,
            Some(StyleValue::Value(BackgroundSize::Contain))
        );
        assert_eq!(
            adjusted.style().cursor,
            Some(StyleValue::Value(Cursor::Default))
        );
        assert_eq!(ui.element(BACKGROUND_ACTION_ID).text(), Some("Reset"));
    }

    client.ui().click(BACKGROUND_ACTION_ID);
    let ui = client.ui();
    assert_page_design_contract(&ui, 28);
    assert_eq!(ui.element(BACKGROUND_TEXTURE_ID).style(), &initial);
    assert_eq!(ui.element(BACKGROUND_ACTION_ID).text(), Some("Apply"));
}

fn assert_hierarchy_design_contract(ui: &UiClient<'_, battlement_rules::UiLabEngine>) {
    assert_page_design_contract(ui, 8);
}

fn assert_page_design_contract(
    ui: &UiClient<'_, battlement_rules::UiLabEngine>,
    word_budget: usize,
) {
    let background = Color::rgb(0.012, 0.025, 0.045);
    let foreground = Color::rgb(0.86, 0.93, 0.95);
    let mut pending = vec![(PAGE_ID, background, foreground)];
    let mut words = 0;
    while let Some((object_id, inherited_background, inherited_foreground)) = pending.pop() {
        let element = ui.element(object_id);
        let style = element.style();
        let background = match &style.background_color {
            Some(StyleValue::Value(value)) => *value,
            Some(StyleValue::Keyword { .. }) | None => inherited_background,
        };
        let foreground = match &style.color {
            Some(StyleValue::Value(value)) => *value,
            Some(StyleValue::Keyword { .. }) | None => inherited_foreground,
        };
        if element.kind() == UiElementKind::Box {
            assert!(
                matches!(style.background_color, Some(StyleValue::Value(_))),
                "sample Box {object_id} does not select an explicit dark surface"
            );
            assert!(
                relative_luminance(background) < 0.5
                    || maximum_channel(background) - minimum_channel(background) >= 0.18,
                "sample Box {object_id} uses a forbidden light surface"
            );
        }
        if let Some(text) = element.text() {
            words += text.split_whitespace().count();
            assert!(style.font_size.is_some_and(|size| size >= 24.0));
            assert!(
                contrast_ratio(foreground, background) >= 4.5,
                "sample text '{text}' does not meet the 4.5:1 contrast requirement"
            );
        }
        pending.extend(
            element
                .children()
                .iter()
                .map(|child| (*child, background, foreground)),
        );
    }
    assert!(
        words <= word_budget,
        "sample renders {words} words above its {word_budget}-word budget"
    );
}

fn contrast_ratio(first: Color, second: Color) -> f64 {
    let first = relative_luminance(first);
    let second = relative_luminance(second);
    (first.max(second) + 0.05) / (first.min(second) + 0.05)
}

fn relative_luminance(color: Color) -> f64 {
    0.2126 * linear_channel(color.r)
        + 0.7152 * linear_channel(color.g)
        + 0.0722 * linear_channel(color.b)
}

fn linear_channel(value: f64) -> f64 {
    if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn maximum_channel(color: Color) -> f64 {
    color.r.max(color.g).max(color.b)
}

fn minimum_channel(color: Color) -> f64 {
    color.r.min(color.g).min(color.b)
}

fn sample_assets() -> FakeAssetCatalog {
    let mut catalog = FakeAssetCatalog::new();
    catalog.add_scene(ui_assets::CONTENT.clone());
    catalog.add_texture(assets::TEXTURE.clone());
    catalog.add_sprite(assets::SPRITE.clone());
    catalog.add_vector_image(assets::VECTOR.clone());
    catalog.add_render_texture(assets::RENDER_TEXTURE.clone());
    catalog.add_texture(assets::CURSOR.clone());
    catalog.add_ui_font(assets::UI_FONT.clone());
    catalog
}
