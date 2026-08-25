use battlement_types::{Color, ObjectId, Rect, SpriteAddress, TextureAddress};
use battlement_ui::{
    Box, DynamicAtlasSettings, FlexDirection, Image, ImageScaleMode, Label, LanguageDirection,
    PanelScaleMode, PanelSettings, PickingMode, Style, UiDocument, UiElement, UiNode,
    UiValidationError, UsageHint, VisualElement, validate_documents, validate_element_update,
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
        UiNode::new(
            id(BOX_ID),
            Box::new().name("canvas").style(
                Style::new()
                    .background_color(Color::rgb(0.02, 0.05, 0.08))
                    .flex_direction(FlexDirection::Row)
                    .padding(24.0),
            ),
        )
        .child(UiNode::new(id(LABEL_ID), Label::new("BATTLEMENT UI"))),
    );

    let value = serde_json::to_value(document).unwrap();
    assert_eq!(value["document_id"], DOCUMENT_ID);
    assert_eq!(value["root_id"], ROOT_ID);
    assert_eq!(value["children"][0]["object_id"], BOX_ID);
    assert_eq!(
        value["children"][0]["children"][0]["element"]["Label"]["text"],
        "BATTLEMENT UI"
    );

    let plain = serde_json::to_value(UiElement::from(VisualElement::new())).unwrap();
    assert_eq!(plain, serde_json::json!({"VisualElement": {}}));
    assert!(matches!(
        serde_json::from_value::<UiElement>(plain).unwrap(),
        UiElement::VisualElement(_)
    ));
}

#[test]
fn style_merge_preserves_base_values_and_overlays_authored_values() {
    let base = Style::new()
        .background_color(Color::rgb(0.02, 0.05, 0.08))
        .width(240.0)
        .padding(16.0);
    let merged = base.merge(Style::new().color(Color::rgb(0.8, 0.9, 1.0)).width(320.0));

    assert_eq!(merged.background_color, Some(Color::rgb(0.02, 0.05, 0.08)));
    assert_eq!(merged.color, Some(Color::rgb(0.8, 0.9, 1.0)));
    assert_eq!(merged.width, Some(320.0));
    assert_eq!(merged.padding, Some(16.0));
}

#[test]
fn container_builders_append_only_selected_children() {
    let box_element = UiNode::new(id(BOX_ID), Box::new())
        .optional_child(Some(UiNode::new(id(LABEL_ID), Label::new("optional"))))
        .optional_child(None)
        .children_if(
            true,
            [UiNode::new(ObjectId::new_v4(), Label::new("included"))],
        )
        .children_if(
            false,
            [UiNode::new(ObjectId::new_v4(), Label::new("excluded"))],
        );
    let box_value = serde_json::to_value(box_element).unwrap();
    assert_eq!(box_value["children"].as_array().unwrap().len(), 2);
    assert_eq!(
        box_value["children"][0]["element"]["Label"]["text"],
        "optional"
    );
    assert_eq!(
        box_value["children"][1]["element"]["Label"]["text"],
        "included"
    );

    let plain = UiNode::new(ObjectId::new_v4(), VisualElement::new())
        .optional_child(None)
        .children_if(true, [UiNode::new(ObjectId::new_v4(), Label::new("plain"))]);
    assert_eq!(
        serde_json::to_value(plain).unwrap()["children"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let document = UiDocument::with_root_id(id(DOCUMENT_ID), id(ROOT_ID))
        .optional_child(None)
        .children_if(
            true,
            [UiNode::new(ObjectId::new_v4(), Label::new("document"))],
        );
    assert_eq!(
        serde_json::to_value(document).unwrap()["children"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn validation_reserves_all_identities_and_rejects_duplicates() {
    let document = UiDocument::with_root_id(id(DOCUMENT_ID), id(ROOT_ID)).child(
        UiNode::new(id(BOX_ID), VisualElement::new())
            .child(UiNode::new(id(LABEL_ID), Label::new("root"))),
    );
    assert_eq!(
        validate_documents(std::slice::from_ref(&document))
            .unwrap()
            .len(),
        4
    );

    let duplicate = UiDocument::with_root_id(id(DOCUMENT_ID), id(ROOT_ID))
        .child(UiNode::new(id(ROOT_ID), Label::new("duplicate")));
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
            "children": [{
                "object_id": BOX_ID,
                "element": {"Box": {"classes": classes}}
            }]
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

#[test]
fn common_state_serializes_and_rejects_attached_usage_hint_updates() {
    let element = Box::new()
        .picking_mode(PickingMode::Ignore)
        .language_direction(LanguageDirection::Rtl)
        .focusable(true)
        .tab_index(-1)
        .delegates_focus(true)
        .usage_hints([UsageHint::DynamicTransform, UsageHint::DynamicColor]);
    let value = serde_json::to_value(UiElement::from(element.clone())).unwrap();

    assert_eq!(value["Box"]["picking_mode"], "Ignore");
    assert_eq!(value["Box"]["language_direction"], "Rtl");
    assert_eq!(value["Box"]["tab_index"], -1);
    assert_eq!(
        value["Box"]["usage_hints"],
        serde_json::json!(["DynamicTransform", "DynamicColor"])
    );
    assert_eq!(
        validate_element_update(&element.into()),
        Err(UiValidationError::InvalidProperty)
    );
}

#[test]
fn image_serialization_selects_one_prepared_native_source() {
    let image = Image::new()
        .source(TextureAddress::new("ui/gallery/texture"))
        .source_rect(Rect::new(4.0, 8.0, 64.0, 32.0))
        .tint_color(Color::rgba(0.25, 0.5, 0.75, 0.8))
        .scale_mode(ImageScaleMode::ScaleAndCrop)
        .uv(Rect::new(0.1, 0.2, 0.3, 0.4));

    assert_eq!(
        serde_json::to_value(UiElement::from(image)).unwrap(),
        serde_json::json!({
            "Image": {
                "source": {"Texture": "ui/gallery/texture"},
                "source_rect": {"x": 4.0, "y": 8.0, "width": 64.0, "height": 32.0},
                "tint_color": {"r": 0.25, "g": 0.5, "b": 0.75, "a": 0.8},
                "scale_mode": "ScaleAndCrop",
                "uv": {"x": 0.1, "y": 0.2, "width": 0.3, "height": 0.4}
            }
        })
    );
}

#[test]
fn image_validation_rejects_incompatible_and_out_of_range_sampling() {
    let sprite_with_source_rect = Image::new()
        .source(SpriteAddress::new("ui/gallery/sprite"))
        .source_rect(Rect::new(0.0, 0.0, 16.0, 16.0));
    assert_eq!(
        validate_documents(&[UiDocument::new(ObjectId::new_v4())
            .child(UiNode::new(ObjectId::new_v4(), sprite_with_source_rect,))]),
        Err(UiValidationError::InvalidProperty)
    );

    let invalid_uv = Image::new()
        .source(TextureAddress::new("ui/gallery/texture"))
        .uv(Rect::new(0.75, 0.0, 0.5, 1.0));
    assert_eq!(
        validate_element_update(&invalid_uv.into()),
        Err(UiValidationError::InvalidProperty)
    );
}

#[test]
fn image_is_a_logical_leaf() {
    let image_with_child = UiDocument::new(ObjectId::new_v4()).child(
        UiNode::new(ObjectId::new_v4(), Image::new())
            .child(UiNode::new(ObjectId::new_v4(), Label::new("overlay"))),
    );

    assert_eq!(
        validate_documents(&[image_with_child]),
        Err(UiValidationError::InvalidHierarchy)
    );
}

fn id(value: &str) -> ObjectId {
    value.parse().unwrap()
}
