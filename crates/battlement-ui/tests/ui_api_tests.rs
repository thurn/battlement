use std::num::NonZeroU32;

use battlement_types::{Color, MaterialAddress, ObjectId, Rect, SpriteAddress, TextureAddress};
use battlement_ui::{
  Align, AspectRatio, BackgroundPosition, BackgroundPositionKeyword, BackgroundRepeat,
  BackgroundRepeatMode, BackgroundSize, BackgroundSource, Choice, Cursor, CursorHotspot, Display,
  DynamicAtlasSettings, FlexDirection, FlexWrap, ImageScaleMode, InlineKeyword,
  InteractionDistance, InteractionLayerMask, Justify, LanguageDirection, Length, LengthOrAuto,
  LengthUnits, LowerLimit, Overflow, OverflowClipBox, PaintLayer, PaintStyle,
  PanelInputConfiguration, PanelInputRedirection, PanelScaleMode, PanelSettings, PickingMode,
  Position, Prop, ScrollViewMode, ScrollerVisibility, SliceType, SliderDirection, Style,
  StyleValue, TouchScrollBehavior, Translate, UiBox, UiButton, UiDocument, UiDropdownField,
  UiElement, UiEventKind, UiGroupBox, UiImage, UiLabel, UiMinMaxSlider, UiNode, UiPopupWindow,
  UiProgressBar, UiRadioButton, UiRadioButtonGroup, UiRepeatButton, UiScrollView, UiScroller,
  UiSlider, UiSliderInt, UiTab, UiTabView, UiTextElement, UiTextField, UiToggle,
  UiToggleButtonGroup, UiValidationError, UiVisualElement, UpperLimit, UsageHint, Vector,
  Visibility, validate_documents, validate_element_update, validate_panel_input_configuration,
  validate_panel_settings,
};

#[test]
fn panel_input_configuration_serializes_exact_native_contract() {
  assert_eq!(
    serde_json::to_value(PanelInputConfiguration::new()).unwrap(),
    serde_json::json!({})
  );
  let configured = PanelInputConfiguration::new()
    .interaction_layers(InteractionLayerMask::new(0x8000_0005))
    .maximum_interaction_distance(InteractionDistance::Inclusive(12.5))
    .input_redirection(PanelInputRedirection::Always);
  assert_eq!(
    serde_json::to_value(&configured).unwrap(),
    serde_json::json!({
        "interaction_layers": 2147483653_u32,
        "maximum_interaction_distance": {"Inclusive": 12.5},
        "input_redirection": "Always"
    })
  );
  assert_eq!(
    serde_json::to_value(InteractionDistance::Unbounded).unwrap(),
    serde_json::json!("Unbounded")
  );
  assert_eq!(validate_panel_input_configuration(&configured), Ok(()));
}

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
fn min_max_slider_and_progress_bar_encode_ranges_and_validate_leaf_state() {
  let range = UiMinMaxSlider::new()
    .min_value(20.0)
    .max_value(80.0)
    .low_limit(LowerLimit::Inclusive(0.0))
    .high_limit(UpperLimit::Inclusive(100.0))
    .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted]);
  assert_eq!(
    serde_json::to_value(UiElement::from(range)).unwrap(),
    serde_json::json!({"MinMaxSlider": {
        "events": ["ValueChanging", "ValueCommitted"],
        "min_value": 20.0,
        "max_value": 80.0,
        "low_limit": {"Inclusive": 0.0},
        "high_limit": {"Inclusive": 100.0}
    }})
  );
  assert_eq!(
    serde_json::to_value(UiElement::from(
      UiMinMaxSlider::new()
        .low_limit(LowerLimit::Unbounded)
        .high_limit(UpperLimit::Unbounded)
    ))
    .unwrap(),
    serde_json::json!({"MinMaxSlider": {
        "low_limit": "Unbounded",
        "high_limit": "Unbounded"
    }})
  );
  assert_eq!(
    serde_json::to_value(UiElement::from(
      UiProgressBar::new()
        .low_value(0.0)
        .high_value(1.0)
        .value(0.5)
        .title("DEPLOYING")
    ))
    .unwrap(),
    serde_json::json!({"ProgressBar": {
        "low_value": 0.0,
        "high_value": 1.0,
        "value": 0.5,
        "title": "DEPLOYING"
    }})
  );
  for element in [
    UiElement::from(UiMinMaxSlider::new()),
    UiElement::from(UiProgressBar::new()),
  ] {
    let document = UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(ObjectId::new_v4(), element)
        .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("invalid"))),
    );
    assert_eq!(
      validate_documents(&[document]),
      Err(UiValidationError::InvalidHierarchy)
    );
  }
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiMinMaxSlider::new()
        .min_value(8.0)
        .max_value(2.0)
        .low_limit(LowerLimit::Inclusive(0.0))
        .high_limit(UpperLimit::Inclusive(10.0)),
    ))]),
    Err(UiValidationError::InvalidProperty)
  );
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiProgressBar::new()
        .low_value(0.0)
        .high_value(10.0)
        .value(11.0),
    ))]),
    Err(UiValidationError::InvalidProperty)
  );
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiGroupBox::new()
        .text("")
        .title_style(Style::new().color(Color::rgb(0.8, 0.9, 1.0))),
    )),]),
    Err(UiValidationError::InvalidProperty)
  );
}

#[test]
fn simple_part_builders_encode_private_keys_and_reject_duplicate_or_missing_parts() {
  let toggle = UiToggle::new()
    .text("Ready")
    .input_style(Style::new().background_color(Color::rgb(0.1, 0.2, 0.3)))
    .checkmark_style(Style::new().width(18));
  assert_eq!(
    serde_json::to_value(UiElement::from(toggle)).unwrap(),
    serde_json::json!({"Toggle": {
        "text": "Ready",
        "parts": [
            {"part": "ToggleInput", "style": {"background_color": {"r": 0.1, "g": 0.2, "b": 0.3}}},
            {"part": "ToggleCheckmark", "style": {"width": {"Px": 18.0}}}
        ]
    }})
  );
  assert_eq!(
    validate_element_update(&UiElement::from(
      UiToggle::new()
        .input_style(Style::new().width(10))
        .input_style(Style::new().height(10))
    )),
    Err(UiValidationError::InvalidProperty)
  );
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiButton::new("No icon").icon_style(Style::new().width(20)),
    )),]),
    Err(UiValidationError::InvalidProperty)
  );
}

#[test]
fn complex_part_builders_encode_indexed_options_and_validate_conditional_parts() {
  let group = UiRadioButtonGroup::new()
    .choices(["Alpha", "Beta"])
    .option_text_style(1, Style::new().color(Color::rgb(0.9, 0.8, 0.2)))
    .all_options_style(Style::new().height(32));
  assert_eq!(
    serde_json::to_value(UiElement::from(group)).unwrap(),
    serde_json::json!({"RadioButtonGroup": {
        "choices": ["Alpha", "Beta"],
        "parts": [
            {"part": "RadioButtonGroupOptionText", "index": 1,
                "style": {"color": {"r": 0.9, "g": 0.8, "b": 0.2}}},
            {"part": "RadioButtonGroupAllOptions", "style": {"height": {"Px": 32.0}}}
        ]
    }})
  );
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiRadioButtonGroup::new()
        .choices(["Only"])
        .option_style(1, Style::new().width(20)),
    ))]),
    Err(UiValidationError::InvalidProperty)
  );
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiSlider::new().fill_style(Style::new().height(6)),
    ))]),
    Err(UiValidationError::InvalidProperty)
  );
  assert!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiTextField::new()
        .multiline(true)
        .vertical_dragger_style(Style::new().width(12)),
    ))])
    .is_ok()
  );
}

#[test]
fn sliders_encode_float_and_integer_ranges_and_validate_bounds() {
  let slider = UiSlider::new()
    .label("Intensity")
    .low_value(-1.0)
    .high_value(1.0)
    .value(0.25)
    .fill(true)
    .page_size(0.125)
    .show_input_field(true)
    .direction(SliderDirection::Horizontal)
    .inverted(true)
    .events([UiEventKind::ValueChanging, UiEventKind::ValueCommitted]);
  assert_eq!(
    serde_json::to_value(UiElement::from(slider)).unwrap(),
    serde_json::json!({"Slider": {
        "events": ["ValueChanging", "ValueCommitted"],
        "label": "Intensity",
        "low_value": -1.0,
        "high_value": 1.0,
        "value": 0.25,
        "fill": true,
        "page_size": 0.125,
        "show_input_field": true,
        "direction": "Horizontal",
        "inverted": true
    }})
  );
  assert!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiSliderInt::new().low_value(1).high_value(9).value(4),
    ))])
    .is_ok()
  );
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiSlider::new().low_value(2.0).high_value(1.0),
    ))]),
    Err(UiValidationError::InvalidProperty)
  );
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
      ObjectId::new_v4(),
      UiSliderInt::new().low_value(0).high_value(5).value(6),
    ))]),
    Err(UiValidationError::InvalidProperty)
  );
  for element in [
    UiElement::from(UiSlider::new()),
    UiElement::from(UiSliderInt::new()),
  ] {
    let document = UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(ObjectId::new_v4(), element)
        .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("invalid"))),
    );
    assert_eq!(
      validate_documents(&[document]),
      Err(UiValidationError::InvalidHierarchy)
    );
  }
}

#[test]
fn dropdown_encodes_coherent_selected_and_empty_choices() {
  let selected = UiDropdownField::new()
    .label("Theme")
    .choices(["Dusk", "Dawn"])
    .selection(1, "Dawn")
    .show_mixed_value(true)
    .events([UiEventKind::ValueCommitted]);
  assert_eq!(
    serde_json::to_value(UiElement::from(selected)).unwrap(),
    serde_json::json!({"DropdownField": {
        "events": ["ValueCommitted"],
        "label": "Theme",
        "show_mixed_value": true,
        "choices": ["Dusk", "Dawn"],
        "selection": {"index": 1, "value": "Dawn"}
    }})
  );
  assert_eq!(
    serde_json::to_value(UiElement::from(UiDropdownField::new().clear_selection())).unwrap(),
    serde_json::json!({"DropdownField": {
        "selection": {"index": null, "value": null}
    }})
  );

  let mismatch = UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
    ObjectId::new_v4(),
    UiDropdownField::new()
      .choices(["Dusk", "Dawn"])
      .selection(1, "Dusk"),
  ));
  assert_eq!(
    validate_documents(&[mismatch]),
    Err(UiValidationError::InvalidProperty)
  );
  let invalid_child = UiDocument::new(ObjectId::new_v4()).child(
    UiNode::new(
      ObjectId::new_v4(),
      UiDropdownField::new()
        .choices(["Dusk"])
        .selection(0, "Dusk"),
    )
    .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("invalid"))),
  );
  assert_eq!(
    validate_documents(&[invalid_child]),
    Err(UiValidationError::InvalidHierarchy)
  );
  assert_eq!(Choice::none(), Choice::default());
}

#[test]
fn choice_groups_encode_indices_and_constrain_button_children() {
  let radio = UiRadioButtonGroup::new()
    .label("Formation")
    .choices(["Line", "Wedge", "Column"])
    .selected_index(1)
    .events([UiEventKind::ValueCommitted]);
  assert_eq!(
    serde_json::to_value(UiElement::from(radio)).unwrap(),
    serde_json::json!({"RadioButtonGroup": {
        "events": ["ValueCommitted"],
        "label": "Formation",
        "choices": ["Line", "Wedge", "Column"],
        "selected_index": 1
    }})
  );
  let toggle = UiToggleButtonGroup::new()
    .label("Filters")
    .multiple_selection(true)
    .allow_empty_selection(true)
    .selected_indices([0, 2]);
  let valid = UiDocument::new(ObjectId::new_v4()).child(
    UiNode::new(ObjectId::new_v4(), toggle)
      .child(UiNode::new(ObjectId::new_v4(), UiButton::new("Air")))
      .child(UiNode::new(ObjectId::new_v4(), UiButton::new("Sea")))
      .child(UiNode::new(ObjectId::new_v4(), UiButton::new("Land"))),
  );
  assert!(validate_documents(&[valid]).is_ok());

  let invalid_order = UiDocument::new(ObjectId::new_v4()).child(
    UiNode::new(
      ObjectId::new_v4(),
      UiToggleButtonGroup::new()
        .multiple_selection(true)
        .allow_empty_selection(true)
        .selected_indices([1, 0]),
    )
    .children([
      UiNode::new(ObjectId::new_v4(), UiButton::new("A")),
      UiNode::new(ObjectId::new_v4(), UiButton::new("B")),
    ]),
  );
  assert_eq!(
    validate_documents(&[invalid_order]),
    Err(UiValidationError::InvalidProperty)
  );
  let invalid_child = UiDocument::new(ObjectId::new_v4()).child(
    UiNode::new(ObjectId::new_v4(), UiToggleButtonGroup::new())
      .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("invalid"))),
  );
  assert_eq!(
    validate_documents(&[invalid_child]),
    Err(UiValidationError::InvalidHierarchy)
  );
  let missing_choices = UiDocument::new(ObjectId::new_v4()).child(UiNode::new(
    ObjectId::new_v4(),
    UiRadioButtonGroup::new().selected_index(0),
  ));
  assert_eq!(
    validate_documents(&[missing_choices]),
    Err(UiValidationError::InvalidProperty)
  );
}

#[test]
fn toggle_and_radio_button_encode_sparse_controlled_boolean_contracts() {
  let toggle = UiToggle::new()
    .label("Settings")
    .text("Shield alerts")
    .value(true)
    .events([UiEventKind::ValueCommitted]);
  assert_eq!(
    serde_json::to_value(UiElement::from(toggle)).unwrap(),
    serde_json::json!({
        "Toggle": {
            "events": ["ValueCommitted"],
            "label": "Settings",
            "text": "Shield alerts",
            "value": true
        }
    })
  );
  assert_eq!(
    serde_json::to_value(UiElement::from(
      UiRadioButton::new()
        .label("Channel")
        .text("Command")
        .value(false)
    ))
    .unwrap(),
    serde_json::json!({
        "RadioButton": {
            "label": "Channel",
            "text": "Command",
            "value": false
        }
    })
  );
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4()).child(
      UiNode::new(ObjectId::new_v4(), UiToggle::new())
        .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("invalid"),)),
    )]),
    Err(UiValidationError::InvalidHierarchy)
  );
}

#[test]
fn button_and_repeat_button_encode_complete_control_contracts() {
  let button = UiButton::new("Launch")
    .rich_text(false)
    .emoji_fallback(false)
    .icon(SpriteAddress::new("ui/button-icon"));
  let value = serde_json::to_value(UiElement::from(button)).unwrap();
  assert_eq!(value["Button"]["text"], "Launch");
  assert_eq!(value["Button"]["enable_rich_text"], false);
  assert!(value["Button"].get("selectable").is_none());
  assert!(value["Button"].get("triple_click_selects_line").is_none());
  assert_eq!(
    value["Button"]["icon"],
    serde_json::json!({"Sprite": "ui/button-icon"})
  );

  let repeat = UiRepeatButton::new(
    "Hold",
    320,
    NonZeroU32::new(160).expect("constant interval is positive"),
  );
  assert!(
    validate_documents(&[
      UiDocument::new(ObjectId::new_v4()).child(UiNode::new(ObjectId::new_v4(), repeat,))
    ])
    .is_ok()
  );
  assert_eq!(
    validate_documents(&[UiDocument::new(ObjectId::new_v4())
      .child(UiNode::new(ObjectId::new_v4(), UiRepeatButton::default(),))]),
    Err(UiValidationError::InvalidProperty)
  );
  assert!(validate_element_update(&UiRepeatButton::default().into()).is_ok());
}

#[test]
fn group_box_and_popup_window_are_sparse_text_containers() {
  let group = UiGroupBox::new().text("Audio");
  let popup = UiPopupWindow::new()
    .text("<b>Deployment</b>")
    .rich_text(true)
    .selectable(true);
  assert_eq!(
    serde_json::to_value(UiElement::from(group.clone())).unwrap(),
    serde_json::json!({"GroupBox": {"text": "Audio"}})
  );
  assert_eq!(
    serde_json::to_value(UiElement::from(popup.clone())).unwrap(),
    serde_json::json!({
        "PopupWindow": {
            "text": "<b>Deployment</b>",
            "enable_rich_text": true,
            "selectable": true
        }
    })
  );

  let document = UiDocument::new(ObjectId::new_v4())
    .child(
      UiNode::new(ObjectId::new_v4(), group)
        .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("Music"))),
    )
    .child(
      UiNode::new(ObjectId::new_v4(), popup)
        .child(UiNode::new(ObjectId::new_v4(), UiLabel::new("Ready"))),
    );
  assert!(validate_documents(&[document]).is_ok());

  let mut group_value = UiElement::from(UiGroupBox::new().text("Before"));
  group_value.apply_update(&UiGroupBox::new().text("").into());
  assert_eq!(
    serde_json::to_value(group_value).unwrap(),
    serde_json::json!({"GroupBox": {"text": ""}})
  );
}

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
      UiBox::new().name("canvas").style(
        Style::new()
          .background_color(Color::rgb(0.02, 0.05, 0.08))
          .flex_direction(FlexDirection::Row)
          .padding(24.0),
      ),
    )
    .child(UiNode::new(id(LABEL_ID), UiLabel::new("BATTLEMENT UI"))),
  );

  let value = serde_json::to_value(document).unwrap();
  assert_eq!(value["document_id"], DOCUMENT_ID);
  assert_eq!(value["root_id"], ROOT_ID);
  assert_eq!(value["children"][0]["object_id"], BOX_ID);
  assert_eq!(
    value["children"][0]["children"][0]["element"]["Label"]["text"],
    "BATTLEMENT UI"
  );

  let plain = serde_json::to_value(UiElement::from(UiVisualElement::new())).unwrap();
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
    Prop::Set(StyleValue::Value(Color::rgb(0.02, 0.05, 0.08)))
  );
  assert_eq!(
    merged.color,
    Prop::Set(StyleValue::Value(Color::rgb(0.8, 0.9, 1.0)))
  );
  assert_eq!(
    merged.width,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(320.0)))
  );
  assert_eq!(
    merged.padding_top,
    Prop::Set(StyleValue::Value(Length::Px(16.0)))
  );
  assert_eq!(
    merged.padding_right,
    Prop::Set(StyleValue::Value(Length::Px(16.0)))
  );
  assert_eq!(
    merged.padding_bottom,
    Prop::Set(StyleValue::Value(Length::Px(16.0)))
  );
  assert_eq!(
    merged.padding_left,
    Prop::Set(StyleValue::Value(Length::Px(16.0)))
  );
}

#[test]
fn style_authoring_shortcuts_expand_to_ordinary_properties() {
  let fill = Style::new().full_size();
  assert_eq!(
    (fill.width, fill.height),
    (
      Prop::Set(StyleValue::Value(LengthOrAuto::Percent(100.0))),
      Prop::Set(StyleValue::Value(LengthOrAuto::Percent(100.0))),
    )
  );

  let positioned = Style::new().absolute_fill().inset((1, 2, 3, 4));
  assert_eq!(
    positioned.position,
    Prop::Set(StyleValue::Value(Position::Absolute))
  );
  assert_eq!(
    positioned.top,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(1.0)))
  );
  assert_eq!(
    positioned.right,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(2.0)))
  );
  assert_eq!(
    positioned.bottom,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(3.0)))
  );
  assert_eq!(
    positioned.left,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(4.0)))
  );

  let centered = Style::new().center_content().translate_y(6);
  assert_eq!(
    centered.align_items,
    Prop::Set(StyleValue::Value(Align::Center))
  );
  assert_eq!(
    centered.justify_content,
    Prop::Set(StyleValue::Value(Justify::Center))
  );
  assert_eq!(
    centered.translate,
    Prop::Set(StyleValue::Value(Translate::two_dimensional(
      Length::Px(0.0),
      Length::Px(6.0),
    )))
  );
}

#[test]
fn layered_paint_serializes_ordered_fills_and_css_insets() {
  let paint = PaintStyle::fill(Color::rgb(0.1, 0.2, 0.3))
    .clip_inset((1, 2))
    .layer(PaintLayer::new(Color::rgb(0.4, 0.5, 0.6)).bounds_inset((3, 4, 5)))
    .layer(PaintLayer::new(Color::rgb(0.7, 0.8, 0.9)).bounds_inset((6, 7, 8, 9)));

  assert_eq!(paint.paint_layers().len(), 2);
  let value = serde_json::to_value(paint).unwrap();
  assert_eq!(
    value["clip_inset"],
    serde_json::json!([{"Px": 1.0}, {"Px": 2.0}, {"Px": 1.0}, {"Px": 2.0}])
  );
  assert_eq!(value["layers"].as_array().unwrap().len(), 2);
  assert_eq!(
    value["layers"][0]["bounds_inset"],
    serde_json::json!([{"Px": 3.0}, {"Px": 4.0}, {"Px": 5.0}, {"Px": 4.0}])
  );
  assert_eq!(
    value["layers"][1]["bounds_inset"],
    serde_json::json!([{"Px": 6.0}, {"Px": 7.0}, {"Px": 8.0}, {"Px": 9.0}])
  );
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
fn layout_style_setters_map_options_to_sparse_operations() {
  let style = Style::new().width(Some(42)).height(Option::<i32>::None);

  assert_eq!(style.width, Prop::Set(StyleValue::from(42)));
  assert_eq!(style.height, Prop::Unset);
  assert_eq!(
    serde_json::to_value(style).unwrap(),
    serde_json::json!({"width": {"Px": 42.0}})
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
fn container_builders_append_only_selected_children() {
  let box_element = UiNode::new(id(BOX_ID), UiBox::new())
    .optional_child(Some(UiNode::new(id(LABEL_ID), UiLabel::new("optional"))))
    .optional_child(None)
    .children_if(
      true,
      [UiNode::new(ObjectId::new_v4(), UiLabel::new("included"))],
    )
    .children_if(
      false,
      [UiNode::new(ObjectId::new_v4(), UiLabel::new("excluded"))],
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

  let plain = UiNode::new(ObjectId::new_v4(), UiVisualElement::new())
    .optional_child(None)
    .children_if(
      true,
      [UiNode::new(ObjectId::new_v4(), UiLabel::new("plain"))],
    );
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
      [UiNode::new(ObjectId::new_v4(), UiLabel::new("document"))],
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

fn id(value: &str) -> ObjectId {
  value.parse().unwrap()
}
