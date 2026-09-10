use super::*;

#[test]
fn panel_input_configuration_rejects_nonfinite_and_negative_distance() {
  for value in [f32::NAN, f32::INFINITY, -0.01] {
    assert_eq!(
      validate_panel_input_configuration(
        &PanelInputConfiguration::new()
          .maximum_interaction_distance(InteractionDistance::Inclusive(value))
      ),
      Err(UiValidationError::InvalidProperty)
    );
  }
}

#[test]
fn paint_validation_rejects_polygons_with_fewer_than_three_vertices() {
  let polygons = [
    PaintStyle::fill(Color::WHITE).clip_polygon([
      [Length::Px(0.0), Length::Px(0.0)],
      [Length::Px(10.0), Length::Px(10.0)],
    ]),
    PaintStyle::new().layer(PaintLayer::new(Color::WHITE).clip_polygon([
      [Length::Px(0.0), Length::Px(0.0)],
      [Length::Px(10.0), Length::Px(10.0)],
    ])),
  ];

  for paint in polygons {
    assert_eq!(
      validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
        ObjectId::new_v4(),
        UiVisualElement::new().paint(paint),
      ))]),
      Err(UiValidationError::InvalidProperty)
    );
  }
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
        .child(UiNode::new(ObjectId::new_v4(), UiBox::new().style(style)))]),
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
        .child(UiNode::new(ObjectId::new_v4(), UiBox::new().style(style)))]),
      Err(UiValidationError::InvalidProperty)
    );
  }
}

#[test]
fn background_and_cursor_validation_rejects_invalid_native_inputs() {
  for style in [
    Style::new().background_position_x(BackgroundPosition::new(BackgroundPositionKeyword::Top, 0)),
    Style::new().background_position_y(BackgroundPosition::new(BackgroundPositionKeyword::Left, 0)),
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
        .child(UiNode::new(ObjectId::new_v4(), UiBox::new().style(style)))]),
      Err(UiValidationError::InvalidProperty)
    );
  }
}

#[test]
fn validation_reserves_all_identities_and_rejects_duplicates() {
  let document = UiDocument::with_root_id(id(DOCUMENT_ID), id(ROOT_ID)).child(
    UiNode::new(id(BOX_ID), UiVisualElement::new())
      .child(UiNode::new(id(LABEL_ID), UiLabel::new("root"))),
  );
  assert_eq!(
    validate_documents(std::slice::from_ref(&document))
      .unwrap()
      .len(),
    4
  );

  let duplicate = UiDocument::with_root_id(id(DOCUMENT_ID), id(ROOT_ID))
    .child(UiNode::new(id(ROOT_ID), UiLabel::new("duplicate")));
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
  assert_eq!(
    validate_panel_settings(
      &PanelSettings::new()
        .target_texture("ui/panel-target")
        .target_display(1)
    ),
    Err(UiValidationError::InvalidProperty)
  );
  assert!(
    validate_panel_settings(
      &PanelSettings::new()
        .scale_mode(PanelScaleMode::ConstantPixelSize)
        .target_texture("ui/panel-target")
    )
    .is_ok()
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
  let element = UiBox::new()
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
  let image = UiImage::new()
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
  let sprite_with_source_rect = UiImage::new()
    .source(SpriteAddress::new("ui/gallery/sprite"))
    .source_rect(Rect::new(0.0, 0.0, 16.0, 16.0));
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4())
      .child(UiNode::new(ObjectId::new_v4(), sprite_with_source_rect,))]),
    Err(UiValidationError::InvalidProperty)
  );

  let invalid_uv = UiImage::new()
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
    UiNode::new(ObjectId::new_v4(), UiImage::new())
      .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("overlay"))),
  );

  assert_eq!(
    validate_documents(&[image_with_child]),
    Err(UiValidationError::InvalidHierarchy)
  );
}

#[test]
fn text_element_is_a_validated_logical_leaf() {
  let text_with_child = UiDocument::new(ObjectId::new_v4()).child(
    UiNode::new(ObjectId::new_v4(), UiTextElement::new("parent"))
      .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("child"))),
  );

  assert_eq!(
    validate_documents(&[text_with_child]),
    Err(UiValidationError::InvalidHierarchy)
  );
  assert_eq!(
    validate_element_update(&UiTextElement::new("x".repeat(65_537)).into()),
    Err(UiValidationError::InvalidProperty)
  );
}

#[test]
fn scroll_controls_encode_sparse_native_properties() {
  let scroll = UiScrollView::new()
    .mode(ScrollViewMode::VerticalAndHorizontal)
    .horizontal_scroller_visibility(ScrollerVisibility::AlwaysVisible)
    .scroll_offset(Vector::new(24.0, 80.0))
    .horizontal_page_size(0.75)
    .vertical_page_size(1.25)
    .mouse_wheel_scroll_size(36.0)
    .touch_scroll_behavior(TouchScrollBehavior::Elastic)
    .scroll_deceleration_rate(0.125)
    .elasticity(0.25)
    .elastic_animation_interval(16);
  assert_eq!(
    serde_json::to_value(UiElement::from(scroll)).unwrap(),
    serde_json::json!({"ScrollView": {
        "mode": "VerticalAndHorizontal",
        "horizontal_scroller_visibility": "AlwaysVisible",
        "scroll_offset": {"x": 24.0, "y": 80.0},
        "horizontal_page_size": 0.75,
        "vertical_page_size": 1.25,
        "mouse_wheel_scroll_size": 36.0,
        "touch_scroll_behavior": "Elastic",
        "scroll_deceleration_rate": 0.125,
        "elasticity": 0.25,
        "elastic_animation_interval": 16
    }})
  );
  assert_eq!(
    serde_json::to_value(UiElement::from(
      UiScroller::new()
        .low_value(-10.0)
        .high_value(10.0)
        .direction(SliderDirection::Horizontal)
        .value(2.5)
    ))
    .unwrap(),
    serde_json::json!({"Scroller": {
        "low_value": -10.0,
        "high_value": 10.0,
        "direction": "Horizontal",
        "value": 2.5
    }})
  );
}

#[test]
fn scroll_control_validation_rejects_nonfinite_and_reversed_ranges() {
  assert_eq!(
    validate_element_update(
      &UiScrollView::new()
        .scroll_offset(Vector::new(f32::NAN, 0.0))
        .into()
    ),
    Err(UiValidationError::InvalidProperty)
  );
  assert_eq!(
    validate_element_update(&UiScroller::new().low_value(2.0).high_value(1.0).into()),
    Err(UiValidationError::InvalidProperty)
  );
}

#[test]
fn tab_view_serialization_and_hierarchy_are_constrained() {
  let tab = UiTab::new("Inspector")
    .icon(SpriteAddress::new("ui/tab-icon"))
    .closeable(true);
  assert_eq!(
    serde_json::to_value(UiElement::from(tab.clone())).unwrap(),
    serde_json::json!({"Tab": {
        "text": "Inspector",
        "icon": {"Sprite": "ui/tab-icon"},
        "closeable": true
    }})
  );
  let tab_view = UiTabView::new()
    .selected_tab_index(0)
    .reorderable(true)
    .events([
      UiEventKind::TabSelectionRequested,
      UiEventKind::TabCloseRequested,
      UiEventKind::TabReorderRequested,
    ]);
  let valid = UiDocument::new(ObjectId::new_v4())
    .child(UiNode::new(ObjectId::new_v4(), tab_view).child(UiNode::new(ObjectId::new_v4(), tab)));
  assert!(validate_documents(&[valid]).is_ok());

  let orphan = UiDocument::new(ObjectId::new_v4())
    .child(UiNode::new(ObjectId::new_v4(), UiTab::new("Orphan")));
  assert_eq!(
    validate_documents(&[orphan]),
    Err(UiValidationError::InvalidHierarchy)
  );
  let wrong_child = UiDocument::new(ObjectId::new_v4()).child(
    UiNode::new(ObjectId::new_v4(), UiTabView::new())
      .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("Not a tab"))),
  );
  assert_eq!(
    validate_documents(&[wrong_child]),
    Err(UiValidationError::InvalidHierarchy)
  );
  let invalid_selection = UiDocument::new(ObjectId::new_v4()).child(
    UiNode::new(ObjectId::new_v4(), UiTabView::new().selected_tab_index(1))
      .child(UiNode::new(ObjectId::new_v4(), UiTab::new("Only"))),
  );
  assert_eq!(
    validate_documents(&[invalid_selection]),
    Err(UiValidationError::InvalidProperty)
  );
}

#[test]
fn text_field_serialization_and_selection_validation_are_complete() {
  let field = UiTextField::new()
    .label("Call sign")
    .value("Rook")
    .multiline(false)
    .password(false)
    .read_only(false)
    .placeholder("Enter a name")
    .hide_placeholder_on_focus(true)
    .cursor_index(4)
    .select_index(1)
    .select_all_on_focus(false)
    .select_all_on_mouse_up(false)
    .events([
      UiEventKind::Input,
      UiEventKind::ValueCommitted,
      UiEventKind::SelectionChanged,
    ]);
  assert_eq!(
    serde_json::to_value(UiElement::from(field.clone())).unwrap(),
    serde_json::json!({"TextField": {
        "label": "Call sign",
        "value": "Rook",
        "multiline": false,
        "password": false,
        "read_only": false,
        "placeholder": "Enter a name",
        "hide_placeholder_on_focus": true,
        "cursor_index": 4,
        "select_index": 1,
        "select_all_on_focus": false,
        "select_all_on_mouse_up": false,
        "events": ["Input", "ValueCommitted", "SelectionChanged"]
    }})
  );
  assert!(validate_element_update(&field.into()).is_ok());
  assert_eq!(
    validate_element_update(&UiTextField::new().value("abc").cursor_index(4).into()),
    Err(UiValidationError::InvalidProperty)
  );
  assert!(
    validate_element_update(&UiTextField::new().value("🌟").cursor_index(2).into()).is_ok(),
    "selection indices follow Unity's UTF-16 model"
  );
}
