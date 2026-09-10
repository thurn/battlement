use super::*;

#[test]
fn release_coverage_navigation_exposes_each_category() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );
  client.ui().click(COVERAGE_BUTTON_ID);
  let ui = client.ui();
  let text = collect_text(&ui, PAGE_ID);
  for category in [
    "ELEMENTS",
    "OUTER STYLE",
    "NATIVE PARTS",
    "EVENTS",
    "ACTIONS",
    "ASSET SOURCES",
    "DOCUMENT MODES",
  ] {
    assert!(
      text.contains(category),
      "coverage dashboard misses category {category}"
    );
  }
  let categories = [
    (COVERAGE_GROUP_IDS[0], "ELEMENTS"),
    (COVERAGE_GROUP_IDS[1], "OUTER STYLE"),
    (COVERAGE_GROUP_IDS[2], "NATIVE PARTS"),
    (COVERAGE_GROUP_IDS[3], "EVENTS"),
    (COVERAGE_GROUP_IDS[4], "ACTIONS"),
    (COVERAGE_GROUP_IDS[5], "ASSET SOURCES"),
    (COVERAGE_GROUP_IDS[6], "DOCUMENT MODES"),
  ];
  for (group_id, category) in categories {
    client.ui().click(group_id);
    let detail = collect_text(&client.ui(), PAGE_ID);
    assert!(
      detail.contains(category),
      "coverage detail misses {category}"
    );
    client.ui().click(COVERAGE_BACK_ID);
  }
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
    saw_font_definition |= matches!(style.unity_font_definition, Prop::Set(_));
    saw_advanced_generator |= matches!(
      style.unity_text_generator,
      Prop::Set(StyleValue::Value(TextGenerator::Advanced))
    );
    saw_selectable_text |= element.kind() == UiElementKind::TextElement;
    pending.extend(element.children());
  }
  assert!(saw_font_definition && saw_advanced_generator && saw_selectable_text);
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
      Prop::Set(StyleValue::Value(FlexDirection::Row))
    );
    assert_eq!(
      ui.element(LAYOUT_PLAYGROUND_ID).style().flex_wrap,
      Prop::Set(StyleValue::Value(FlexWrap::Wrap))
    );
    assert_eq!(ui.element(LAYOUT_ACTION_ID).text(), Some("Column layout"));
  }

  client.ui().click(LAYOUT_ACTION_ID);
  {
    let ui = client.ui();
    assert_page_design_contract(&ui, 6);
    assert_eq!(
      ui.element(LAYOUT_PLAYGROUND_ID).style().flex_direction,
      Prop::Set(StyleValue::Value(FlexDirection::ColumnReverse))
    );
    assert_eq!(
      ui.element(LAYOUT_GAMMA_ID).style().position,
      Prop::Set(StyleValue::Value(Position::Absolute))
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
      Prop::Set(StyleValue::Value(Overflow::Hidden))
    );
    assert_eq!(
      ui.element(APPEARANCE_HIDDEN_ID).style().visibility,
      Prop::Set(StyleValue::Value(Visibility::Hidden))
    );
    assert_eq!(
      ui.element(APPEARANCE_REMOVED_ID).style().display,
      Prop::Set(StyleValue::Value(Display::None))
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
      Prop::Set(StyleValue::Value(Visibility::Visible))
    );
    assert_eq!(
      ui.element(APPEARANCE_REMOVED_ID).style().display,
      Prop::Set(StyleValue::Value(Display::Flex))
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
        Prop::Set(StyleValue::Value(value)) if value.keyword == BackgroundPositionKeyword::Left
    ));
    assert!(matches!(
        texture.background_repeat,
        Prop::Set(StyleValue::Value(value))
            if value.x == BackgroundRepeatMode::Repeat
                && value.y == BackgroundRepeatMode::NoRepeat
    ));
    assert_eq!(
      texture.background_size,
      Prop::Set(StyleValue::Value(BackgroundSize::Auto))
    );
    assert!(matches!(
        texture.cursor,
        Prop::Set(StyleValue::Value(Cursor::Texture { ref address, .. }))
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
      Prop::Set(StyleValue::Value(BackgroundSize::Contain))
    );
    assert_eq!(
      adjusted.style().cursor,
      Prop::Set(StyleValue::Value(Cursor::Default))
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
    assert_eq!(ui.element(TRANSFORM_STATUS_ID).text(), Some("Ready"));
    assert_eq!(ui.element(TRANSFORM_ACTION_ID).text(), Some("Launch"));
    assert!(matches!(initial.transition_property, Prop::Set(_)));
    assert!(matches!(initial.transition_duration, Prop::Set(_)));
    assert!(matches!(initial.transition_delay, Prop::Set(_)));
    assert!(matches!(initial.transition_timing_function, Prop::Set(_)));
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
      Some("Transform complete")
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

fn collect_text(ui: &UiClient<'_, battlement_rules::UiLabEngine>, object_id: ObjectId) -> String {
  let element = ui.element(object_id);
  let mut values = element
    .text()
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
  values.extend(
    element
      .children()
      .iter()
      .map(|child| collect_text(ui, *child)),
  );
  values.join(" | ")
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
      Prop::Set(StyleValue::Value(value)) => *value,
      Prop::Set(StyleValue::Keyword { .. }) | Prop::Unset | Prop::Reset => inherited_background,
    };
    let foreground = match &style.color {
      Prop::Set(StyleValue::Value(value)) => *value,
      Prop::Set(StyleValue::Keyword { .. }) | Prop::Unset | Prop::Reset => inherited_foreground,
    };
    if element.kind() == UiElementKind::Box {
      assert!(
        matches!(style.background_color, Prop::Set(StyleValue::Value(_))),
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
          Prop::Set(StyleValue::Value(battlement::Length::Px(size))) if size >= 24.0
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
