use battlement::{ObjectId, object_id};
use battlement_fake::{
    assets::FakeAssetCatalog,
    client::{FakeClient, UiClient},
};

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

#[test]
fn ui_lab_clicks_dispatch_and_apply_all_ui_command_families() {
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(battlement_rules::CONTENT_SCENE);
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        assets,
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
    let mut assets = FakeAssetCatalog::new();
    assets.add_scene(battlement_rules::CONTENT_SCENE);
    let mut client = FakeClient::connect(
        battlement_rules::create_engine().expect("UI sample engine should initialize"),
        assets,
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

fn assert_hierarchy_design_contract(ui: &UiClient<'_, battlement_rules::UiLabEngine>) {
    let mut pending = vec![PAGE_ID];
    let mut words = 0;
    while let Some(object_id) = pending.pop() {
        let element = ui.element(object_id);
        if let Some(text) = element.text() {
            words += text.split_whitespace().count();
            assert!(element.style().font_size.is_some_and(|size| size >= 24.0));
        }
        pending.extend_from_slice(element.children());
    }
    assert!(words <= 8, "hierarchy sample renders {words} words");
}
