use battlement_types::{Color, MaterialAddress, ObjectId, Rect, SpriteAddress, TextureAddress};
use battlement_ui::{
    Align, AspectRatio, BackgroundPosition, BackgroundPositionKeyword, BackgroundRepeat,
    BackgroundRepeatMode, BackgroundSize, BackgroundSource, Box, Cursor, CursorHotspot, Display,
    DynamicAtlasSettings, FlexDirection, FlexWrap, Image, ImageScaleMode, InlineKeyword, Justify,
    Label, LanguageDirection, Length, LengthOrAuto, LengthUnits, Overflow, OverflowClipBox,
    PanelScaleMode, PanelSettings, PickingMode, Position, SliceType, Style, StyleValue, UiDocument,
    UiElement, UiNode, UiValidationError, UsageHint, Visibility, VisualElement, validate_documents,
    validate_element_update, validate_panel_settings,
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

    assert_eq!(
        merged.background_color,
        Some(StyleValue::Value(Color::rgb(0.02, 0.05, 0.08)))
    );
    assert_eq!(
        merged.color,
        Some(StyleValue::Value(Color::rgb(0.8, 0.9, 1.0)))
    );
    assert_eq!(
        merged.width,
        Some(StyleValue::Value(LengthOrAuto::Px(320.0)))
    );
    assert_eq!(
        merged.padding_top,
        Some(StyleValue::Value(Length::Px(16.0)))
    );
    assert_eq!(
        merged.padding_right,
        Some(StyleValue::Value(Length::Px(16.0)))
    );
    assert_eq!(
        merged.padding_bottom,
        Some(StyleValue::Value(Length::Px(16.0)))
    );
    assert_eq!(
        merged.padding_left,
        Some(StyleValue::Value(Length::Px(16.0)))
    );
}

#[test]
fn appearance_style_catalog_serializes_values_shorthands_and_keywords() {
    let style = Style::new()
        .background_color(InlineKeyword::Initial)
        .background_image(BackgroundSource::Sprite(SpriteAddress::new(
            "ui/sliced-panel",
        )))
        .border_color((
            Color::rgb(0.04, 0.08, 0.12),
            Color::rgb(0.16, 0.20, 0.24),
            Color::rgb(0.27, 0.31, 0.35),
            Color::rgb(0.39, 0.43, 0.47),
        ))
        .border_radius((4, 8, 12))
        .border_width((1, 2))
        .color(Color::rgb(0.86, 0.90, 0.94))
        .display(Display::Flex)
        .opacity(0.75)
        .overflow(Overflow::Hidden)
        .unity_background_image_tint_color(Color::rgba(0.5, 0.75, 1.0, 0.8))
        .unity_material(MaterialAddress::new("ui/material"))
        .unity_overflow_clip_box(OverflowClipBox::ContentBox)
        .unity_slice_bottom(5)
        .unity_slice_left(6)
        .unity_slice_right(7)
        .unity_slice_scale(2)
        .unity_slice_top(8)
        .unity_slice_type(SliceType::Tiled)
        .visibility(Visibility::Hidden);

    let value = serde_json::to_value(style).unwrap();
    assert_eq!(
        value["background_color"],
        serde_json::json!({"Keyword": "Initial"})
    );
    assert_eq!(value["border_top_width"], 1.0);
    assert_eq!(value["border_right_width"], 2.0);
    assert_eq!(value["border_bottom_width"], 1.0);
    assert_eq!(value["border_left_width"], 2.0);
    assert_eq!(
        value["border_top_left_radius"],
        serde_json::json!({"Px": 4.0})
    );
    assert_eq!(
        value["border_top_right_radius"],
        serde_json::json!({"Px": 8.0})
    );
    assert_eq!(
        value["border_bottom_right_radius"],
        serde_json::json!({"Px": 12.0})
    );
    assert_eq!(
        value["border_bottom_left_radius"],
        serde_json::json!({"Px": 8.0})
    );
    assert_eq!(value["display"], "Flex");
    assert_eq!(
        value["background_image"],
        serde_json::json!({"Sprite": "ui/sliced-panel"})
    );
    assert_eq!(value["overflow"], "Hidden");
    assert_eq!(value["unity_material"], "ui/material");
    assert_eq!(value["unity_slice_type"], "Tiled");
    assert_eq!(value["visibility"], "Hidden");
}

#[test]
fn layout_style_catalog_serializes_typed_values_and_expanded_shorthands() {
    let style = Style::new()
        .align_content(Align::Center)
        .align_items(Align::Stretch)
        .align_self(Align::FlexEnd)
        .aspect_ratio(AspectRatio::new(16.0, 9.0))
        .flex_basis(LengthOrAuto::Auto)
        .flex_direction(FlexDirection::RowReverse)
        .flex_grow(2)
        .flex_shrink(1)
        .flex_wrap(FlexWrap::WrapReverse)
        .height(LengthOrAuto::Auto)
        .justify_content(Justify::SpaceEvenly)
        .margin((8, 16, 24, 32))
        .max_width(90.pct())
        .min_height(48)
        .padding((4, 8, 12))
        .position(Position::Absolute)
        .right(5.pct())
        .top(12)
        .width(InlineKeyword::Initial);

    assert_eq!(
        serde_json::to_value(style).unwrap(),
        serde_json::json!({
            "align_content": "Center",
            "align_items": "Stretch",
            "align_self": "FlexEnd",
            "aspect_ratio": {"Ratio": {"width": 16.0, "height": 9.0}},
            "flex_basis": "Auto",
            "flex_direction": "RowReverse",
            "flex_grow": 2.0,
            "flex_shrink": 1.0,
            "flex_wrap": "WrapReverse",
            "height": "Auto",
            "justify_content": "SpaceEvenly",
            "margin_top": {"Px": 8.0},
            "margin_right": {"Px": 16.0},
            "margin_bottom": {"Px": 24.0},
            "margin_left": {"Px": 32.0},
            "max_width": {"Percent": 90.0},
            "min_height": {"Px": 48.0},
            "padding_top": {"Px": 4.0},
            "padding_right": {"Px": 8.0},
            "padding_bottom": {"Px": 12.0},
            "padding_left": {"Px": 8.0},
            "position": "Absolute",
            "right": {"Percent": 5.0},
            "top": {"Px": 12.0},
            "width": {"Keyword": "Initial"}
        })
    );
}

#[test]
fn layout_validation_rejects_invalid_bounds_without_mutating_fake_state() {
    for style in [
        Style::new().width(-1),
        Style::new().padding_left(-1),
        Style::new().padding_top(f32::NAN),
        Style::new().flex_grow(-0.5),
        Style::new().aspect_ratio(AspectRatio::new(0.0, 1.0)),
    ] {
        assert_eq!(
            validate_documents(&[UiDocument::new(ObjectId::new_v4())
                .child(UiNode::new(ObjectId::new_v4(), Box::new().style(style)))]),
            Err(UiValidationError::InvalidProperty)
        );
    }
}

#[test]
fn appearance_validation_rejects_invalid_values() {
    for style in [
        Style::new().background_color(Color::rgba(-0.1, 0.0, 0.0, 1.0)),
        Style::new().border_left_width(-1),
        Style::new().border_top_left_radius(-1),
        Style::new().opacity(1.1),
        Style::new().unity_slice_bottom(-1),
        Style::new().unity_slice_scale(0),
    ] {
        assert_eq!(
            validate_documents(&[UiDocument::new(ObjectId::new_v4())
                .child(UiNode::new(ObjectId::new_v4(), Box::new().style(style)))]),
            Err(UiValidationError::InvalidProperty)
        );
    }
}

#[test]
fn background_and_cursor_styles_serialize_the_native_value_shapes() {
    let style = Style::new()
        .background_position_x(BackgroundPosition::new(
            BackgroundPositionKeyword::Right,
            12.pct(),
        ))
        .background_position_y(BackgroundPosition::new(
            BackgroundPositionKeyword::Bottom,
            8,
        ))
        .background_repeat(BackgroundRepeat::new(
            BackgroundRepeatMode::Round,
            BackgroundRepeatMode::Space,
        ))
        .background_size(BackgroundSize::axes(50.pct(), LengthOrAuto::Auto))
        .cursor(Cursor::texture(
            TextureAddress::new("ui/cursor"),
            CursorHotspot::new(3.0, 5.0),
        ));

    assert_eq!(
        serde_json::to_value(style).unwrap(),
        serde_json::json!({
            "background_position_x": {
                "keyword": "Right",
                "offset": {"Percent": 12.0}
            },
            "background_position_y": {
                "keyword": "Bottom",
                "offset": {"Px": 8.0}
            },
            "background_repeat": {"x": "Round", "y": "Space"},
            "background_size": {
                "Axes": {"x": {"Percent": 50.0}, "y": "Auto"}
            },
            "cursor": {
                "Texture": {
                    "address": "ui/cursor",
                    "hotspot": {"x": 3.0, "y": 5.0}
                }
            }
        })
    );
}

#[test]
fn background_and_cursor_validation_rejects_invalid_native_inputs() {
    for style in [
        Style::new()
            .background_position_x(BackgroundPosition::new(BackgroundPositionKeyword::Top, 0)),
        Style::new()
            .background_position_y(BackgroundPosition::new(BackgroundPositionKeyword::Left, 0)),
        Style::new().background_position_x(BackgroundPosition::new(
            BackgroundPositionKeyword::Center,
            f32::NAN,
        )),
        Style::new().background_size(BackgroundSize::axes(-1, LengthOrAuto::Auto)),
        Style::new().cursor(Cursor::texture(
            TextureAddress::new("ui/cursor"),
            CursorHotspot::new(-1.0, 0.0),
        )),
        Style::new().cursor(Cursor::texture(
            TextureAddress::new("ui/cursor"),
            CursorHotspot::new(0.0, f32::INFINITY),
        )),
    ] {
        assert_eq!(
            validate_documents(&[UiDocument::new(ObjectId::new_v4())
                .child(UiNode::new(ObjectId::new_v4(), Box::new().style(style)))]),
            Err(UiValidationError::InvalidProperty)
        );
    }
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
