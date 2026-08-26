use battlement::{
    BackgroundPositionKeyword, BackgroundRepeatMode, BackgroundSize, BackgroundSource, Color,
    Cursor, Display, FlexDirection, FlexWrap, FocusDirection, FocusEvent, ImageSource, KeyModifier,
    KeyModifiers, ObjectId, Overflow, PanelPoint, PointerButton, PointerButtonEvent, PointerType,
    Position, StyleValue, TextGenerator, TransitionEvent, TransitionProperty, UiElement,
    UiElementKind, UiEvent, UiEventBody, Vector, Visibility, object_id,
};
use battlement_fake::{
    assets::FakeAssetCatalog,
    client::{FakeClient, ui::UiClient},
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
const TRANSFORMS_BUTTON_ID: ObjectId = object_id!("416cc818-7d31-4d01-8e39-712be437494b");
const TRANSFORM_TARGET_ID: ObjectId = object_id!("066af04d-a6d7-46e1-b7ac-a62001a90239");
const TRANSFORM_STATUS_ID: ObjectId = object_id!("6274737d-8539-4991-ad00-a20b3a5a9fc2");
const TRANSFORM_ACTION_ID: ObjectId = object_id!("6277a6b7-b774-4302-9d06-81c1991c214f");
const TYPOGRAPHY_BUTTON_ID: ObjectId = object_id!("879be431-2981-4aa0-8094-603f106bf067");
const COMPLEX_PARTS_BUTTON_ID: ObjectId = object_id!("8da1d1bd-f7a9-420b-a122-f5c75ca3b295");
const COMPLEX_PARTS_TOGGLE_ID: ObjectId = object_id!("9321c5a3-9b82-462d-9f68-26da56edcbb7");
const COMPLEX_PARTS_SLIDER_ID: ObjectId = object_id!("0121421b-c595-4eb8-9689-88e02dd62669");
const COMPLEX_PARTS_TITLE_ID: ObjectId = object_id!("139c41bc-e97b-4da9-9f70-1c58f9136953");
const CONTAINERS_BUTTON_ID: ObjectId = object_id!("b3858e8c-0b75-4c55-b5f1-d2e0a18cf1ef");
const TITLED_GROUP_ID: ObjectId = object_id!("9ab84d41-dd5f-4202-a62b-da4643222ac8");
const EMPTY_GROUP_ID: ObjectId = object_id!("05acfc99-c92d-46cd-93cd-3738ff025e62");
const DYNAMIC_GROUP_ID: ObjectId = object_id!("3a9d57df-b920-4ec3-b170-3afbc6ce0494");
const DYNAMIC_GROUP_CHILD_ID: ObjectId = object_id!("7ceac51e-b580-4e67-b995-191216cbff88");
const DYNAMIC_GROUP_ACTION_ID: ObjectId = object_id!("c21e285f-6999-4df7-8a6b-559339520962");
const POPUP_WINDOW_ID: ObjectId = object_id!("71347582-7a69-4270-a76f-c4c25546e086");
const SCROLL_BUTTON_ID: ObjectId = object_id!("b4baa362-1979-4bff-ae2d-d6a736ab4bb4");
const PRIMARY_SCROLL_ID: ObjectId = object_id!("d24fec17-cb8a-4b9c-a604-da4113d6ef9b");
const CONTROLLED_SCROLLER_ID: ObjectId = object_id!("df12adf3-3a6c-4900-bb15-1f53117f1a8e");
const SCROLL_STATUS_ID: ObjectId = object_id!("898a986b-893d-48d8-bd68-5d39ef58c086");
const SCROLLER_STATUS_ID: ObjectId = object_id!("a7338149-f968-40a3-9bdd-e7640546e2fe");
const TABS_BUTTON_ID: ObjectId = object_id!("0dbf590c-b821-4ba5-b4a7-426382a96a16");
const TEXT_FIELDS_BUTTON_ID: ObjectId = object_id!("d1810adf-f4fa-4eb7-8b44-46d60e22341d");
const TAB_VIEW_ID: ObjectId = object_id!("aa1bd60d-71e5-4f3a-a7ba-13f456621b9c");
const BOARD_TAB_ID: ObjectId = object_id!("e7491a26-c97e-4668-9b72-0aba2f8920c1");
const NOTES_TAB_ID: ObjectId = object_id!("1560af93-b7eb-489e-983b-768747b9db49");
const LOADOUT_TAB_ID: ObjectId = object_id!("9fca8e31-3f73-4245-8fbf-523b1094ef0a");
const TIMELINE_TAB_ID: ObjectId = object_id!("d3f27972-0998-4e83-ad01-3125540ad95a");
const SIGNAL_TAB_ID: ObjectId = object_id!("abbb5697-bb75-4f18-85ca-f5bb706dc59f");
const TAB_STATUS_ID: ObjectId = object_id!("752743e9-cb89-4148-ad40-e5076f78f6e1");
const ACCEPTED_TEXT_ID: ObjectId = object_id!("fd496f77-d46e-4bf9-8f5e-5cba8229d94f");
const NORMALIZED_TEXT_ID: ObjectId = object_id!("df0c6d77-9ff1-40cb-8ae3-a01353df5c73");
const REJECTED_TEXT_ID: ObjectId = object_id!("c20ac846-5730-48ab-89ea-9c943d5e385b");
const TEXT_STATUS_ID: ObjectId = object_id!("8a83987f-581f-4f32-8ce8-e0a99c70174d");
const TEXT_DRAFT_ID: ObjectId = object_id!("f93c739b-a044-44ed-89de-05a343937df6");
const TEXT_COMMITTED_ID: ObjectId = object_id!("b6ce5ac8-1923-4470-a2a1-b9d9ad8fe7d1");
const BOOLEAN_CONTROLS_BUTTON_ID: ObjectId = object_id!("b95de403-9b85-44a2-aebe-acd016c92fa6");
const ACCEPTED_TOGGLE_ID: ObjectId = object_id!("93ecbf8e-5be7-4087-b292-6f68903436c1");
const REJECTED_TOGGLE_ID: ObjectId = object_id!("d18a9439-619d-4ca8-ac58-d82d999b3bf1");
const ACCEPTED_RADIO_ID: ObjectId = object_id!("bfe98ac4-cfa5-4f56-8e6a-253837c66c05");
const REJECTED_RADIO_ID: ObjectId = object_id!("174b5d07-dd4f-4fe6-a264-3863ea6bc318");
const BOOLEAN_STATUS_ID: ObjectId = object_id!("1745a91d-06f7-460c-bd3b-bd1f432332c0");
const BOOLEAN_HISTORY_ID: ObjectId = object_id!("65cba5dd-fc33-49e3-a636-a6d4fc59e73d");
const CHOICE_GROUPS_BUTTON_ID: ObjectId = object_id!("bf246175-3572-4a9d-bd1b-fc91946f035e");
const FORMATION_ID: ObjectId = object_id!("34ee78d0-a503-4d77-b61d-bbd86cf39e41");
const FILTER_ID: ObjectId = object_id!("17805693-79d9-46ac-97db-1694047f8a9e");
const FILTER_SUMMARY_ID: ObjectId = object_id!("01d7f042-cdae-4e9c-8020-817d5e83ae18");
const CHOICE_STATUS_ID: ObjectId = object_id!("6553e506-c92a-4f50-995e-58380393bb6f");
const CHOICE_HISTORY_ID: ObjectId = object_id!("84a701b8-cce9-4165-9637-9b7a24856d7d");
const DROPDOWNS_BUTTON_ID: ObjectId = object_id!("feae3645-8809-42f3-b4f6-00afe473b2f4");
const THEME_DROPDOWN_ID: ObjectId = object_id!("ae31830c-672e-4e99-b409-02ba8383d452");
const LOADOUT_DROPDOWN_ID: ObjectId = object_id!("2d5a2b47-1e52-45c2-b454-a178157133f0");
const CLEAR_LOADOUT_ID: ObjectId = object_id!("c1834769-2048-40f4-953d-0268561883b5");
const THEME_SUMMARY_ID: ObjectId = object_id!("727e62a9-ebce-48cb-876f-20f86784b8cc");
const LOADOUT_SUMMARY_ID: ObjectId = object_id!("2e8fdeee-8310-4173-9daf-87905506c15c");
const DROPDOWN_STATUS_ID: ObjectId = object_id!("4c864234-bb34-43fd-bf0a-634a44111156");
const DROPDOWN_HISTORY_ID: ObjectId = object_id!("948fd3dd-ac76-4761-8831-e9abb02db7d5");
const SLIDERS_BUTTON_ID: ObjectId = object_id!("581694e0-ad9e-477d-a776-478169f39c45");
const CONTINUOUS_SLIDER_ID: ObjectId = object_id!("08e45324-236a-469d-a4f8-f2f40922a9b8");
const STEPPED_SLIDER_ID: ObjectId = object_id!("c1ad6472-f8ae-40cb-9d21-60f6e544db53");
const CONTINUOUS_VALUE_ID: ObjectId = object_id!("27420acd-df31-45fa-99c2-4bf6bde37f7e");
const STEPPED_VALUE_ID: ObjectId = object_id!("12988004-2b5a-4d6d-9eb6-4960f656394b");
const SLIDER_LIVE_STATUS_ID: ObjectId = object_id!("13ba592a-5f70-4a64-892a-21a919479e5d");
const SLIDER_COMMIT_STATUS_ID: ObjectId = object_id!("0d1be49a-b9fc-437d-8d48-d2724e7efe1f");
const RANGES_BUTTON_ID: ObjectId = object_id!("69c28345-59e0-4d2c-a374-b302421d3713");
const RESOURCE_RANGE_ID: ObjectId = object_id!("4be5cd99-a70d-4dca-af82-57dc73f91eea");
const RANGE_STATUS_ID: ObjectId = object_id!("cb0e1e49-857d-4a3b-a95e-f0dce69060d8");
const POINTER_ROUTING_BUTTON_ID: ObjectId = object_id!("8be537d2-16e7-47ee-9a50-31cd36a13522");
const POINTER_TARGET_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000003");
const POINTER_PAYLOAD_ID: ObjectId = object_id!("22100000-0000-4000-8000-000000000004");
const KEYBOARD_NAVIGATION_BUTTON_ID: ObjectId = object_id!("2db08d30-a377-40e6-b9a0-a0036833122a");
const KEYBOARD_ALPHA_ID: ObjectId = object_id!("23100000-0000-4000-8000-000000000001");
const KEYBOARD_BRAVO_ID: ObjectId = object_id!("23100000-0000-4000-8000-000000000002");
const KEYBOARD_INSPECTOR_ID: ObjectId = object_id!("23100000-0000-4000-8000-000000000005");

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
fn pointer_route_page_receives_one_complete_fake_event() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(POINTER_ROUTING_BUTTON_ID);
    client.ui().send_event(UiEvent {
        target_id: POINTER_TARGET_ID,
        body: UiEventBody::PointerDown(PointerButtonEvent {
            position: PanelPoint { x: 412.0, y: 288.0 },
            delta: Vector { x: 3.0, y: -2.0 },
            pointer_id: 4,
            button: PointerButton::Left,
            buttons: 1,
            pressure: 0.5,
            click_count: 1,
            modifiers: KeyModifiers::new(vec![KeyModifier::Shift])
                .expect("single modifier is canonical"),
            pointer_type: PointerType::Mouse,
        }),
    });

    let ui = client.ui();
    let payload = ui
        .element(POINTER_PAYLOAD_ID)
        .text()
        .expect("pointer payload should be rendered");
    assert!(payload.contains("POINTER DOWN"));
    assert!(payload.contains("412, 288"));
    assert!(payload.contains("Shift"));
}

#[test]
fn keyboard_page_explains_focus_relation_and_submit_precedence() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );
    client.ui().click(KEYBOARD_NAVIGATION_BUTTON_ID);
    client.ui().send_event(UiEvent {
        target_id: KEYBOARD_BRAVO_ID,
        body: UiEventBody::Focus(FocusEvent {
            related_target_id: Some(KEYBOARD_ALPHA_ID),
            direction: FocusDirection::Right,
        }),
    });
    assert!(
        client
            .ui()
            .element(KEYBOARD_INSPECTOR_ID)
            .text()
            .expect("focus relation should be rendered")
            .contains("from ALPHA")
    );
    client.ui().click(KEYBOARD_BRAVO_ID);
    assert!(
        client
            .ui()
            .element(KEYBOARD_INSPECTOR_ID)
            .text()
            .expect("activation should be rendered")
            .contains("Pointer Click used the same Rust handler")
    );
}

#[test]
fn typography_page_covers_font_sources_text_styles_and_text_element_behavior() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(TYPOGRAPHY_BUTTON_ID);
    let ui = client.ui();
    assert_page_design_contract(&ui, 40);
    let mut saw_font_definition = false;
    let mut saw_advanced_generator = false;
    let mut saw_selectable_text = false;
    let mut pending = vec![PAGE_ID];
    while let Some(id) = pending.pop() {
        let element = ui.element(id);
        let style = element.style();
        saw_font_definition |= style.unity_font_definition.is_some();
        saw_advanced_generator |= matches!(
            style.unity_text_generator,
            Some(StyleValue::Value(TextGenerator::Advanced))
        );
        saw_selectable_text |= element.kind() == UiElementKind::TextElement;
        pending.extend(element.children());
    }
    assert!(saw_font_definition && saw_advanced_generator && saw_selectable_text);
}

#[test]
fn complex_parts_page_updates_conditional_parts_without_rebuilding() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(COMPLEX_PARTS_BUTTON_ID);
    assert_eq!(
        client.ui().element(COMPLEX_PARTS_TOGGLE_ID).text(),
        Some("Create conditional parts")
    );
    client.ui().click(COMPLEX_PARTS_TOGGLE_ID);
    assert_eq!(
        client.ui().element(COMPLEX_PARTS_TOGGLE_ID).text(),
        Some("Remove conditional parts")
    );
    assert!(matches!(
        client.ui().element(COMPLEX_PARTS_SLIDER_ID).element(),
        UiElement::Slider(value) if value.fill == Some(true) && value.show_input_field == Some(true)
    ));
    assert_eq!(
        client.ui().element(COMPLEX_PARTS_TITLE_ID).text(),
        Some("AUTHORED TITLE")
    );

    client.ui().click(COMPLEX_PARTS_TOGGLE_ID);
    assert_eq!(
        client.ui().element(COMPLEX_PARTS_TOGGLE_ID).text(),
        Some("Create conditional parts")
    );
    assert!(matches!(
        client.ui().element(COMPLEX_PARTS_SLIDER_ID).element(),
        UiElement::Slider(value) if value.fill == Some(false) && value.show_input_field == Some(false)
    ));
    assert_eq!(client.ui().element(COMPLEX_PARTS_TITLE_ID).text(), Some(""));
}

#[test]
fn containers_page_preserves_logical_children_across_conditional_titles() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(CONTAINERS_BUTTON_ID);
    {
        let ui = client.ui();
        assert_eq!(ui.element(TITLED_GROUP_ID).text(), Some("AUDIO SETTINGS"));
        assert_eq!(ui.element(TITLED_GROUP_ID).children().len(), 2);
        assert_eq!(ui.element(EMPTY_GROUP_ID).text(), None);
        assert!(ui.element(EMPTY_GROUP_ID).children().is_empty());
        assert_eq!(
            ui.element(DYNAMIC_GROUP_ID).children(),
            [DYNAMIC_GROUP_CHILD_ID, DYNAMIC_GROUP_ACTION_ID]
        );
        assert_eq!(ui.element(DYNAMIC_GROUP_ID).text(), Some(""));
        let popup_children = ui.element(POPUP_WINDOW_ID).children();
        assert_eq!(popup_children.len(), 2);
        assert_eq!(
            ui.element(popup_children[0]).text(),
            Some("Sector 7  /  clear")
        );
        assert_eq!(
            ui.element(popup_children[1]).text(),
            Some("Squad ETA  /  04:20")
        );
    }

    client.ui().click(DYNAMIC_GROUP_ACTION_ID);
    {
        let ui = client.ui();
        assert_eq!(
            ui.element(DYNAMIC_GROUP_ID).text(),
            Some("TACTICAL OVERRIDES")
        );
        assert_eq!(
            ui.element(DYNAMIC_GROUP_ID).children(),
            [DYNAMIC_GROUP_CHILD_ID, DYNAMIC_GROUP_ACTION_ID]
        );
        assert_eq!(
            ui.element(DYNAMIC_GROUP_CHILD_ID).text(),
            Some("Title created; authored content stayed in place.")
        );
    }

    client.ui().click(DYNAMIC_GROUP_ACTION_ID);
    let ui = client.ui();
    assert_eq!(ui.element(DYNAMIC_GROUP_ID).text(), Some(""));
    assert_eq!(
        ui.element(DYNAMIC_GROUP_ID).children(),
        [DYNAMIC_GROUP_CHILD_ID, DYNAMIC_GROUP_ACTION_ID]
    );
}

#[test]
fn scroll_page_matches_manual_settlement_and_controlled_value_round_trip() {
    let (mut client, clock) = FakeClient::connect_clocked(
        |_| battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(SCROLL_BUTTON_ID);
    {
        let ui = client.ui();
        assert_eq!(
            ui.element(PRIMARY_SCROLL_ID).kind(),
            UiElementKind::ScrollView
        );
        let UiElement::ScrollView(scroll) = ui.element(PRIMARY_SCROLL_ID).element() else {
            unreachable!("scroll specimen kind changed")
        };
        assert_eq!(scroll.scroll_offset, None);
        assert_eq!(scroll.horizontal_page_size, None);
        assert_eq!(scroll.vertical_page_size, None);
        assert_eq!(scroll.mouse_wheel_scroll_size, Some(1.0));
        assert_eq!(scroll.touch_scroll_behavior, None);
        assert_eq!(scroll.scroll_deceleration_rate, None);
        assert_eq!(scroll.elasticity, None);
        assert_eq!(scroll.elastic_animation_interval, None);
        assert_eq!(
            ui.element(CONTROLLED_SCROLLER_ID).kind(),
            UiElementKind::Scroller
        );
        assert_eq!(ui.element(SCROLL_STATUS_ID).text(), Some("Settled 0 × 0"));
        assert_eq!(ui.element(SCROLLER_STATUS_ID).text(), Some("Committed 42"));
    }

    client.ui().scroll_begin(PRIMARY_SCROLL_ID);
    client
        .ui()
        .scroll_change(PRIMARY_SCROLL_ID, Vector::new(72.0, 204.0));
    assert_eq!(client.ui().element(SCROLL_STATUS_ID).text(), Some("Moving"));
    clock.advance(std::time::Duration::from_millis(100));
    client.ui().scroll_end(PRIMARY_SCROLL_ID);
    client.ui().advance();
    assert_eq!(
        client.ui().element(SCROLL_STATUS_ID).text(),
        Some("Settled 72 × 204")
    );

    client.ui().scroller_begin(CONTROLLED_SCROLLER_ID);
    client.ui().scroller_change(CONTROLLED_SCROLLER_ID, 68.0);
    assert_eq!(
        client.ui().element(SCROLLER_STATUS_ID).text(),
        Some("Preview 68")
    );
    client.ui().scroller_commit(CONTROLLED_SCROLLER_ID);
    assert_eq!(
        client.ui().element(SCROLLER_STATUS_ID).text(),
        Some("Committed 68")
    );
    let ui = client.ui();
    let battlement::UiElement::Scroller(value) = ui.element(CONTROLLED_SCROLLER_ID).element()
    else {
        panic!("controlled element changed kind");
    };
    assert_eq!(value.value, Some(68.0));
}

#[test]
fn tabs_page_round_trips_selection_reorder_and_close_veto() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(TABS_BUTTON_ID);
    assert_eq!(
        client.ui().element(TAB_VIEW_ID).children(),
        [
            BOARD_TAB_ID,
            NOTES_TAB_ID,
            LOADOUT_TAB_ID,
            TIMELINE_TAB_ID,
            SIGNAL_TAB_ID,
        ]
    );

    client.ui().tab_select(TAB_VIEW_ID, 3);
    let ui = client.ui();
    let battlement::UiElement::TabView(view) = ui.element(TAB_VIEW_ID).element() else {
        panic!("workspace changed kind");
    };
    assert_eq!(view.selected_tab_index, Some(3));

    client.ui().tab_reorder(TAB_VIEW_ID, 3, 1);
    assert_eq!(
        client.ui().element(TAB_VIEW_ID).children(),
        [
            BOARD_TAB_ID,
            TIMELINE_TAB_ID,
            NOTES_TAB_ID,
            LOADOUT_TAB_ID,
            SIGNAL_TAB_ID,
        ]
    );

    client.ui().tab_close(TAB_VIEW_ID, 0);
    assert!(client.ui().contains(BOARD_TAB_ID));
    assert_eq!(
        client.ui().element(TAB_STATUS_ID).text(),
        Some("Rejected close | BOARD is pinned")
    );

    client.ui().tab_close(TAB_VIEW_ID, 2);
    assert!(!client.ui().contains(NOTES_TAB_ID));
    assert_eq!(client.ui().element(TAB_VIEW_ID).children().len(), 4);
    assert_eq!(
        client.ui().element(TAB_STATUS_ID).text(),
        Some("Closed | 4 tabs remain")
    );
}

#[test]
fn text_field_page_separates_drafts_from_accepted_normalized_and_rejected_commits() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(TEXT_FIELDS_BUTTON_ID);
    client.ui().text_input(ACCEPTED_TEXT_ID, "Knight");
    assert_eq!(
        client.ui().element(TEXT_DRAFT_ID).text(),
        Some("LOCAL DRAFT  Knight")
    );
    assert_eq!(
        client.ui().element(TEXT_COMMITTED_ID).text(),
        Some("RUST COMMITTED  Rook")
    );
    assert_eq!(client.ui().element(ACCEPTED_TEXT_ID).text(), Some("Rook"));
    client.ui().text_selection(ACCEPTED_TEXT_ID, 6, 0);
    client.ui().text_commit(ACCEPTED_TEXT_ID);
    assert_eq!(client.ui().element(ACCEPTED_TEXT_ID).text(), Some("Knight"));
    assert_eq!(
        client.ui().element(TEXT_STATUS_ID).text(),
        Some("ACCEPTED · exact value authored")
    );

    client.ui().text_input(NORMALIZED_TEXT_ID, "  bravo-9  ");
    client.ui().text_commit(NORMALIZED_TEXT_ID);
    assert_eq!(
        client.ui().element(NORMALIZED_TEXT_ID).text(),
        Some("BRAVO-9")
    );
    assert_eq!(
        client.ui().element(TEXT_STATUS_ID).text(),
        Some("NORMALIZED · BRAVO-9")
    );

    client.ui().text_input(REJECTED_TEXT_ID, "South Gate");
    client.ui().text_commit(REJECTED_TEXT_ID);
    assert_eq!(
        client.ui().element(REJECTED_TEXT_ID).text(),
        Some("North Gate")
    );
    assert_eq!(
        client.ui().element(TEXT_STATUS_ID).text(),
        Some("REJECTED · kept prior value")
    );
}

#[test]
fn boolean_controls_restore_native_proposals_until_rust_authors_state() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(BOOLEAN_CONTROLS_BUTTON_ID);

    client.ui().toggle_click(ACCEPTED_TOGGLE_ID);
    assert_eq!(
        client.ui().element(ACCEPTED_TOGGLE_ID).bool_value(),
        Some(true)
    );
    assert_eq!(
        client.ui().element(BOOLEAN_STATUS_ID).text(),
        Some("ACCEPTED · threat alerts committed ON")
    );
    client.ui().toggle_click(ACCEPTED_TOGGLE_ID);
    assert_eq!(
        client.ui().element(ACCEPTED_TOGGLE_ID).bool_value(),
        Some(false)
    );
    assert_eq!(
        client.ui().element(BOOLEAN_STATUS_ID).text(),
        Some("ACCEPTED · threat alerts committed OFF")
    );

    client.ui().toggle_click(REJECTED_TOGGLE_ID);
    assert_eq!(
        client.ui().element(REJECTED_TOGGLE_ID).bool_value(),
        Some(true)
    );
    assert_eq!(
        client.ui().element(BOOLEAN_HISTORY_ID).text(),
        Some("PROPOSAL  ON → OFF  |  committed before callback: ON")
    );

    client.ui().radio_click(ACCEPTED_RADIO_ID);
    assert_eq!(
        client.ui().element(ACCEPTED_RADIO_ID).bool_value(),
        Some(true)
    );

    client.ui().radio_click(REJECTED_RADIO_ID);
    assert_eq!(
        client.ui().element(REJECTED_RADIO_ID).bool_value(),
        Some(false)
    );
    assert_eq!(
        client.ui().element(BOOLEAN_STATUS_ID).text(),
        Some("REJECTED · restricted channel stays OFF")
    );
    assert_eq!(
        client.ui().element(BOOLEAN_HISTORY_ID).text(),
        Some("PROPOSAL  OFF → ON  |  committed before callback: OFF")
    );
}

#[test]
fn choice_groups_commit_exclusive_and_sorted_multi_selection_indices() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(CHOICE_GROUPS_BUTTON_ID);
    assert_eq!(client.ui().element(FORMATION_ID).selected_index(), Some(0));
    assert_eq!(
        client.ui().element(FILTER_ID).selected_indices(),
        Some([0, 2].as_slice())
    );

    client.ui().radio_group_select(FORMATION_ID, 1);
    assert_eq!(client.ui().element(FORMATION_ID).selected_index(), Some(1));
    assert_eq!(
        client.ui().element(CHOICE_STATUS_ID).text(),
        Some("FORMATION · WEDGE committed")
    );
    assert_eq!(
        client.ui().element(CHOICE_HISTORY_ID).text(),
        Some("EXCLUSIVE  LINE → WEDGE  |  index 0 → 1")
    );

    client.ui().toggle_group_click(FILTER_ID, 1);
    assert_eq!(
        client.ui().element(FILTER_ID).selected_indices(),
        Some([0, 1, 2].as_slice())
    );
    assert_eq!(
        client.ui().element(FILTER_SUMMARY_ID).text(),
        Some("SELECTED INDICES · [0, 1, 2]")
    );
    assert_eq!(
        client.ui().element(CHOICE_STATUS_ID).text(),
        Some("FILTERS · AIR + LAND + SEA")
    );
}

#[test]
fn dropdowns_accept_reject_and_clear_coherent_choices() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(DROPDOWNS_BUTTON_ID);
    client.ui().dropdown_select(THEME_DROPDOWN_ID, 1);
    assert_eq!(
        client.ui().element(THEME_DROPDOWN_ID).choice(),
        Some(&battlement::Choice::selected(1, "SOLAR"))
    );
    assert_eq!(
        client.ui().element(THEME_SUMMARY_ID).text(),
        Some("COMMITTED · SOLAR (index 1)")
    );

    client.ui().dropdown_select(LOADOUT_DROPDOWN_ID, 1);
    assert_eq!(
        client.ui().element(LOADOUT_DROPDOWN_ID).choice(),
        Some(&battlement::Choice::selected(0, "SCOUT"))
    );
    assert_eq!(
        client.ui().element(DROPDOWN_STATUS_ID).text(),
        Some("REJECTED · HEAVY remains uncommitted")
    );
    assert_eq!(
        client.ui().element(DROPDOWN_HISTORY_ID).text(),
        Some("REJECTED  SCOUT → HEAVY  |  native proposal rolled back")
    );

    client.ui().click(CLEAR_LOADOUT_ID);
    assert_eq!(
        client.ui().element(LOADOUT_DROPDOWN_ID).choice(),
        Some(&battlement::Choice::none())
    );
    assert_eq!(
        client.ui().element(LOADOUT_SUMMARY_ID).text(),
        Some("CLEARED · no selected index or value")
    );
}

#[test]
fn sliders_keep_drag_values_transient_and_author_one_typed_release_value() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(SLIDERS_BUTTON_ID);
    {
        let ui = client.ui();
        let UiElement::Slider(continuous) = ui.element(CONTINUOUS_SLIDER_ID).element() else {
            unreachable!("continuous specimen kind changed")
        };
        assert_eq!(continuous.value, Some(42.0));
        assert_eq!(continuous.fill, Some(true));
        assert_eq!(continuous.show_input_field, Some(true));
    }

    client.ui().slider_begin(CONTINUOUS_SLIDER_ID);
    client.ui().slider_change(CONTINUOUS_SLIDER_ID, 73.5);
    {
        let ui = client.ui();
        let UiElement::Slider(continuous) = ui.element(CONTINUOUS_SLIDER_ID).element() else {
            unreachable!("continuous specimen kind changed")
        };
        assert_eq!(
            continuous.value,
            Some(42.0),
            "drag state remains native-local"
        );
    }
    assert_eq!(
        client.ui().element(SLIDER_LIVE_STATUS_ID).text(),
        Some("LIVE  thrust trim  73.5%")
    );
    client.ui().slider_commit(CONTINUOUS_SLIDER_ID);
    {
        let ui = client.ui();
        let UiElement::Slider(continuous) = ui.element(CONTINUOUS_SLIDER_ID).element() else {
            unreachable!("continuous specimen kind changed")
        };
        assert_eq!(continuous.value, Some(73.5));
    }
    assert_eq!(
        client.ui().element(CONTINUOUS_VALUE_ID).text(),
        Some("FINAL · 73.5%")
    );

    client.ui().slider_int_begin(STEPPED_SLIDER_ID);
    client.ui().slider_int_change(STEPPED_SLIDER_ID, 6.6);
    client.ui().slider_int_commit(STEPPED_SLIDER_ID);
    {
        let ui = client.ui();
        let UiElement::SliderInt(stepped) = ui.element(STEPPED_SLIDER_ID).element() else {
            unreachable!("stepped specimen kind changed")
        };
        assert_eq!(stepped.value, Some(7));
        assert_eq!(stepped.inverted, Some(true));
    }
    assert_eq!(
        client.ui().element(STEPPED_VALUE_ID).text(),
        Some("FINAL · STEP 7")
    );
    assert_eq!(
        client.ui().element(SLIDER_COMMIT_STATUS_ID).text(),
        Some("COMMITTED  vertical integer 7")
    );
}

#[test]
fn range_sample_previews_and_authors_one_ordered_release_value() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(RANGES_BUTTON_ID);
    client.ui().min_max_slider_begin(RESOURCE_RANGE_ID);
    client
        .ui()
        .min_max_slider_change(RESOURCE_RANGE_ID, 31.0, 68.0);
    assert_eq!(
        client.ui().element(RANGE_STATUS_ID).text(),
        Some("LIVE  reserve 31-68%")
    );
    {
        let ui = client.ui();
        let UiElement::MinMaxSlider(range) = ui.element(RESOURCE_RANGE_ID).element() else {
            unreachable!("resource range kind changed")
        };
        assert_eq!((range.min_value, range.max_value), (Some(24.0), Some(76.0)));
    }

    client.ui().min_max_slider_commit(RESOURCE_RANGE_ID);
    {
        let ui = client.ui();
        let UiElement::MinMaxSlider(range) = ui.element(RESOURCE_RANGE_ID).element() else {
            unreachable!("resource range kind changed")
        };
        assert_eq!((range.min_value, range.max_value), (Some(31.0), Some(68.0)));
    }
    assert_eq!(
        client.ui().element(RANGE_STATUS_ID).text(),
        Some("COMMITTED  reserve 31-68%")
    );
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

#[test]
fn transforms_page_reports_transition_payload_and_restores_initial_state() {
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        sample_assets(),
    );

    client.ui().click(TRANSFORMS_BUTTON_ID);
    let initial = client.ui().element(TRANSFORM_TARGET_ID).style().clone();
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 24);
        assert_eq!(filter_function_count(&ui, PAGE_ID), 8);
        assert_eq!(ui.element(TRANSFORM_STATUS_ID).text(), Some("Ready"));
        assert_eq!(ui.element(TRANSFORM_ACTION_ID).text(), Some("Launch"));
        assert!(initial.transition_property.is_some());
        assert!(initial.transition_duration.is_some());
        assert!(initial.transition_delay.is_some());
        assert!(initial.transition_timing_function.is_some());
    }

    client.ui().click(TRANSFORM_ACTION_ID);
    client.ui().transition_start(
        TRANSFORM_TARGET_ID,
        TransitionEvent::new(vec![TransitionProperty::Rotate], 0.0),
    );
    assert_eq!(
        client.ui().element(TRANSFORM_STATUS_ID).text(),
        Some("Running")
    );
    client.ui().transition_end(
        TRANSFORM_TARGET_ID,
        TransitionEvent::new(
            vec![
                TransitionProperty::Rotate,
                TransitionProperty::Scale,
                TransitionProperty::Translate,
            ],
            480.0,
        ),
    );
    {
        let ui = client.ui();
        assert_page_design_contract(&ui, 24);
        assert_eq!(
            ui.element(TRANSFORM_STATUS_ID).text(),
            Some("Rotate Scale Translate 480 ms")
        );
        assert_eq!(ui.element(TRANSFORM_ACTION_ID).text(), Some("Reset"));
        assert_ne!(ui.element(TRANSFORM_TARGET_ID).style(), &initial);
    }

    client.ui().click(TRANSFORM_ACTION_ID);
    client.ui().transition_cancel(
        TRANSFORM_TARGET_ID,
        TransitionEvent::new(vec![TransitionProperty::Rotate], 100.0),
    );
    assert_eq!(
        client.ui().element(TRANSFORM_STATUS_ID).text(),
        Some("Cancelled")
    );
    client.ui().transition_end(
        TRANSFORM_TARGET_ID,
        TransitionEvent::new(
            vec![
                TransitionProperty::Rotate,
                TransitionProperty::Scale,
                TransitionProperty::Translate,
            ],
            480.0,
        ),
    );
    let ui = client.ui();
    assert_page_design_contract(&ui, 24);
    assert_eq!(ui.element(TRANSFORM_TARGET_ID).style(), &initial);
    assert_eq!(ui.element(TRANSFORM_STATUS_ID).text(), Some("Ready"));
    assert_eq!(ui.element(TRANSFORM_ACTION_ID).text(), Some("Launch"));
}

fn filter_function_count(
    ui: &UiClient<'_, battlement_rules::UiLabEngine>,
    object_id: ObjectId,
) -> usize {
    let element = ui.element(object_id);
    let current = match &element.style().filter {
        Some(StyleValue::Value(values)) => values.as_slice().len(),
        Some(StyleValue::Keyword { .. }) | None => 0,
    };
    current
        + element
            .children()
            .iter()
            .map(|child| filter_function_count(ui, *child))
            .sum::<usize>()
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
            assert!(matches!(
                style.font_size,
                Some(StyleValue::Value(battlement::Length::Px(size))) if size >= 24.0
            ));
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
