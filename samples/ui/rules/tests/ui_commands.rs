use battlement::{ObjectId, object_id};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};

const INTERACTIONS_BUTTON_ID: ObjectId = object_id!("4969d46f-c28c-4e5d-85a0-0321f9931f89");
const CALLBACK_BUTTON_ID: ObjectId = object_id!("7e0b078e-13d9-43c3-a491-84178e157fb2");
const LABEL_COMPONENT_ID: ObjectId = object_id!("5768cfee-a137-49c0-b76c-5ebfa6c227c1");
const GREETING_ID: ObjectId = object_id!("2d8ac61c-49bb-43ce-9656-faa11238351f");
const TRANSIENT_CARD_ID: ObjectId = object_id!("45a1a00c-2624-4e40-b675-3c5f59c62f53");

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
