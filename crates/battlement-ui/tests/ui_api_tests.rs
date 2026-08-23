use battlement_types::{Color, ObjectId};
use battlement_ui::{
    Box, DynamicAtlasSettings, FlexDirection, Label, PanelScaleMode, PanelSettings, Style,
    UiDocument, UiElement, UiValidationError, VisualElement, validate_documents,
    validate_panel_settings,
};

const DOCUMENT_ID: &str = "3b5fe431-f332-4314-a0f6-a7353fa17622";
const ROOT_ID: &str = "471834d0-8abc-4964-a3da-f8bc61de7c16";
const BOX_ID: &str = "fc59ba64-b70c-4a20-83fd-1852b1cb4995";
const LABEL_ID: &str = "a9e0ac34-da16-4d33-8952-b6541ef075e8";

#[test]
fn panel_defaults_are_omitted() {
    assert_eq!(serde_json::to_string(&PanelSettings::new()).unwrap(), "{}");
    assert_eq!(
        serde_json::from_str::<PanelSettings>("{}").unwrap(),
        PanelSettings::new()
    );
}

#[test]
fn document_and_supported_elements_have_the_declared_shape() {
    let document = UiDocument::with_root_id(id(DOCUMENT_ID), id(ROOT_ID)).child(
        Box::with_id(id(BOX_ID))
            .name("canvas")
            .style(
                Style::new()
                    .background_color(Color::rgb(0.02, 0.05, 0.08))
                    .flex_direction(FlexDirection::Row)
                    .padding(24.0),
            )
            .child(Label::with_id(id(LABEL_ID), "BATTLEMENT UI")),
    );

    let value = serde_json::to_value(document).unwrap();
    assert_eq!(value["document_id"], DOCUMENT_ID);
    assert_eq!(value["root_id"], ROOT_ID);
    assert_eq!(value["children"][0]["Box"]["object_id"], BOX_ID);
    assert_eq!(
        value["children"][0]["Box"]["children"][0]["Label"]["text"],
        "BATTLEMENT UI"
    );

    let plain = serde_json::to_value(UiElement::from(VisualElement::with_id(id(BOX_ID)))).unwrap();
    assert_eq!(plain["VisualElement"]["object_id"], BOX_ID);
    assert!(matches!(
        serde_json::from_value::<UiElement>(plain).unwrap(),
        UiElement::VisualElement(_)
    ));
}

#[test]
fn validation_reserves_all_identities_and_rejects_duplicates() {
    let document = UiDocument::with_root_id(id(DOCUMENT_ID), id(ROOT_ID))
        .child(VisualElement::with_id(id(BOX_ID)).child(Label::with_id(id(LABEL_ID), "root")));
    assert_eq!(
        validate_documents(std::slice::from_ref(&document))
            .unwrap()
            .len(),
        4
    );

    let duplicate = UiDocument::with_root_id(id(DOCUMENT_ID), id(ROOT_ID))
        .child(Label::with_id(id(ROOT_ID), "duplicate"));
    assert_eq!(
        validate_documents(&[duplicate]),
        Err(UiValidationError::DuplicateObject)
    );
}

#[test]
fn panel_validation_rejects_cross_mode_and_atlas_mismatches() {
    assert_eq!(
        validate_panel_settings(
            &PanelSettings::new()
                .scale_mode(PanelScaleMode::ConstantPixelSize)
                .reference_dpi(144.0)
        ),
        Err(UiValidationError::InvalidProperty)
    );
    let atlas = DynamicAtlasSettings {
        max_sub_texture_size: 0,
        ..DynamicAtlasSettings::default()
    };
    assert_eq!(
        validate_panel_settings(&PanelSettings::new().dynamic_atlas(atlas)),
        Err(UiValidationError::InvalidProperty)
    );
}

#[test]
fn document_validation_rejects_empty_and_duplicate_classes() {
    let with_classes = |classes: serde_json::Value| {
        serde_json::from_value::<UiDocument>(serde_json::json!({
            "document_id": DOCUMENT_ID,
            "root_id": ROOT_ID,
            "children": [{"Box": {"object_id": BOX_ID, "classes": classes}}]
        }))
        .unwrap()
    };
    let empty = with_classes(serde_json::json!([""]));
    assert_eq!(
        validate_documents(&[empty]),
        Err(UiValidationError::InvalidProperty)
    );

    let duplicate = with_classes(serde_json::json!(["card", "card"]));
    assert_eq!(
        validate_documents(&[duplicate]),
        Err(UiValidationError::InvalidProperty)
    );
}

fn id(value: &str) -> ObjectId {
    value.parse().unwrap()
}
