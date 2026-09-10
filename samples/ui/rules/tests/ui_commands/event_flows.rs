use super::*;

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
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::PointerDown(PointerButtonEvent {
      position: PanelPoint { x: 412.0, y: 288.0 },
      delta: Vector { x: 3.0, y: -2.0 },
      pointer_id: 4,
      button: PointerButton::Left,
      buttons: 1,
      pressure: 0.5,
      click_count: 1,
      modifiers: KeyModifiers::new(vec![KeyModifier::Shift]).expect("single modifier is canonical"),
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

  client.ui().send_event(UiEvent {
    target_id: POINTER_TARGET_ID,
    cancelable: false,
    default_prevented: false,
    body: UiEventBody::PointerCapture(PointerCaptureEvent { pointer_id: 4 }),
  });
  assert_eq!(
    client.ui().element(POINTER_CAPTURE_ID).text(),
    Some("● CAPTURED · POINTER OWNED BY TARGET")
  );
  client.ui().send_event(UiEvent {
    target_id: POINTER_TARGET_ID,
    cancelable: false,
    default_prevented: false,
    body: UiEventBody::PointerCaptureOut(PointerCaptureEvent { pointer_id: 4 }),
  });
  assert!(
    client
      .ui()
      .element(POINTER_CAPTURE_ID)
      .text()
      .expect("capture completion should render")
      .contains("RELEASE OBSERVED")
  );
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
    cancelable: false,
    default_prevented: false,
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

  client.ui().send_event(UiEvent {
    target_id: KEYBOARD_BRAVO_ID,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::KeyDown(KeyEvent {
      physical_key: Some(PhysicalKey::KeyA),
      text: "a".to_owned(),
      modifiers: KeyModifiers::default(),
    }),
  });
  assert!(
    client
      .ui()
      .element(KEYBOARD_INSPECTOR_ID)
      .text()
      .expect("physical key should render")
      .contains("PHYSICAL KEY DOWN")
  );
  client.ui().send_event(UiEvent {
    target_id: KEYBOARD_BRAVO_ID,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::NavigationMove(NavigationMoveEvent {
      direction: NavigationDirection::Right,
      move_vector: Vector { x: 1.0, y: 0.0 },
    }),
  });
  assert!(
    client
      .ui()
      .element(KEYBOARD_INSPECTOR_ID)
      .text()
      .expect("navigation move should render")
      .contains("NAVIGATION MOVE")
  );
  client.ui().navigation_submit(KEYBOARD_BRAVO_ID);
  assert!(
    client
      .ui()
      .element(KEYBOARD_INSPECTOR_ID)
      .text()
      .expect("navigation submit should render")
      .contains("Navigation submit became exactly one Click")
  );
  client.ui().send_event(UiEvent {
    target_id: KEYBOARD_BRAVO_ID,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::NavigationCancel(NavigationEvent::default()),
  });
  assert!(
    client
      .ui()
      .element(KEYBOARD_INSPECTOR_ID)
      .text()
      .expect("navigation cancel should render")
      .contains("NAVIGATION CANCEL")
  );
}

#[test]
fn remaining_events_page_explains_link_identity_and_layout_lifecycle() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );
  client.ui().click(REMAINING_EVENTS_BUTTON_ID);
  let point = PanelPoint { x: 420.0, y: 280.0 };
  client
    .ui()
    .link_enter(REMAINING_LINK_ID, 7, point, "field-guide", "FIELD GUIDE");
  client
    .ui()
    .link_enter(REMAINING_LINK_ID, 8, point, "appendix", "APPENDIX");
  client.ui().link_down(
    REMAINING_LINK_ID,
    7,
    point,
    "field-guide",
    "FIELD GUIDE",
    PointerButton::Left,
  );
  client.ui().link_up(
    REMAINING_LINK_ID,
    7,
    point,
    "field-guide",
    "FIELD GUIDE",
    PointerButton::Left,
  );
  client.ui().link_leave(REMAINING_LINK_ID, 7, point);
  assert!(
    client
      .ui()
      .element(REMAINING_LINK_INSPECTOR_ID)
      .text()
      .expect("link timeline should render")
      .contains("field-guide · FIELD GUIDE")
  );
  client.ui().link_leave(REMAINING_LINK_ID, 8, point);
  assert!(
    client
      .ui()
      .element(REMAINING_LINK_INSPECTOR_ID)
      .text()
      .expect("second pointer identity should render")
      .contains("appendix · APPENDIX")
  );
  client.ui().text_selection(REMAINING_LINK_ID, 8, 3);
  assert!(
    client
      .ui()
      .element(REMAINING_LINK_INSPECTOR_ID)
      .text()
      .expect("selection should render")
      .contains("SELECTION  observed")
  );
  client
    .ui()
    .link_enter(REMAINING_LINK_ID, 9, point, "stale", "STALE");
  client.ui().detach_from_panel(REMAINING_LINK_ID);
  client.ui().link_leave(REMAINING_LINK_ID, 9, point);

  client.ui().geometry_changed(
    REMAINING_TARGET_ID,
    Rect::new(10.0, 10.0, 168.0, 62.0),
    Rect::new(10.0, 10.0, 224.0, 78.0),
  );
  client.ui().attach_to_panel(REMAINING_TARGET_ID);
  client.ui().transition_start(
    REMAINING_TARGET_ID,
    TransitionEvent::new(vec![TransitionProperty::Width], 20.0),
  );
  client.ui().transition_cancel(
    REMAINING_TARGET_ID,
    TransitionEvent::new(vec![TransitionProperty::Width], 40.0),
  );
  client.ui().transition_end(
    REMAINING_TARGET_ID,
    TransitionEvent::new(
      vec![
        TransitionProperty::Width,
        TransitionProperty::Height,
        TransitionProperty::Rotate,
        TransitionProperty::Scale,
        TransitionProperty::BackgroundColor,
      ],
      420.0,
    ),
  );
  client.ui().detach_from_panel(REMAINING_TARGET_ID);
  let ui = client.ui();
  let timeline = ui
    .element(REMAINING_LIFECYCLE_ID)
    .text()
    .expect("lifecycle timeline should render");
  assert!(timeline.contains("finite old → new rect"));
  assert!(timeline.contains("04  END"));
  assert!(timeline.contains("05  CANCEL    observed"));
  assert!(timeline.contains("06  DETACH    observed"));

  client.ui().click(REMAINING_ACTION_ID);
  assert_eq!(
    client.ui().element(REMAINING_TARGET_LABEL_ID).text(),
    Some("SETTLED")
  );
  assert_eq!(
    client.ui().element(REMAINING_ACTION_ID).is_enabled(),
    Some(false)
  );
  client.ui().transition_end(
    REMAINING_TARGET_ID,
    TransitionEvent::new(
      vec![
        TransitionProperty::Width,
        TransitionProperty::Height,
        TransitionProperty::Rotate,
        TransitionProperty::Scale,
        TransitionProperty::BackgroundColor,
      ],
      420.0,
    ),
  );
  assert_eq!(
    client.ui().element(REMAINING_ACTION_ID).is_enabled(),
    Some(true)
  );
  client.ui().click(REMAINING_ACTION_ID);
  assert_eq!(
    client.ui().element(REMAINING_TARGET_LABEL_ID).text(),
    Some("READY")
  );
}

#[test]
fn action_page_runs_every_action_and_proves_controlled_cleanup() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );
  client.ui().click(ACTIONS_BUTTON_ID);
  client.ui().click(ACTION_RUN_ID);
  {
    let ui = client.ui();
    assert_eq!(
      ui.element(ACTION_SELECTABLE_ID).kind(),
      UiElementKind::TextElement
    );
    assert_eq!(ui.selection(ACTION_SELECTABLE_ID), Some((11, 3)));
    assert_eq!(ui.focused(), Some(ACTION_SELECTABLE_ID));
    assert_eq!(ui.pointer_capture(17), None);
    assert!(
      ui.element(ACTION_STATUS_ID)
        .text()
        .expect("action status should render")
        .contains("Focus/Blur > ScrollTo > SelectText > Capture/Release")
    );
    assert!(ui.contains(ACTION_SCROLL_TARGET_ID));
  }

  client.ui().toggle_click(ACTION_ACCEPTED_ID);
  assert_eq!(
    client.ui().element(ACTION_ACCEPTED_ID).bool_value(),
    Some(true)
  );
  client.ui().toggle_click(ACTION_REJECTED_ID);
  assert_eq!(
    client.ui().element(ACTION_REJECTED_ID).bool_value(),
    Some(true)
  );
  assert!(
    client
      .ui()
      .element(ACTION_CONTROL_STATUS_ID)
      .text()
      .expect("rejection status should render")
      .contains("rolled back to ON")
  );

  client
    .ui()
    .text_input(ACTION_DRAFT_ID, "Uncommitted local draft");
  assert_eq!(
    client.ui().text_draft(ACTION_DRAFT_ID),
    "Committed: North Gate"
  );
  client.ui().slider_begin(ACTION_DRAG_ID);
  client.ui().slider_change(ACTION_DRAG_ID, 82.0);
  assert!(
    client
      .ui()
      .element(ACTION_CONTROL_STATUS_ID)
      .text()
      .expect("cleanup status should render")
      .contains("0 cleanup events")
  );
  assert!(client.world().input_enabled());
  client.ui().click(ACTION_CLEANUP_ID);
  assert!(
    client
      .ui()
      .element(ACTION_CONTROL_STATUS_ID)
      .text()
      .expect("cleanup reset should render")
      .contains("READY")
  );
}

#[test]
fn render_modes_page_identifies_the_active_contract_and_shows_the_live_panel_target() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );
  client.ui().click(RENDER_MODES_BUTTON_ID);
  let text = collect_text(&client.ui(), PAGE_ID);
  assert!(text.contains("DOCUMENT RENDERED TO TEXTURE"));
  assert!(text.contains("A separate UI document, displayed here as a live texture."));
  assert!(text.contains("CONSTANT PIXEL  ·  ACTIVE"));
  assert!(!text.contains("PHYSICAL SIZE"));
  assert!(!text.contains("SCREEN SIZE"));
  assert_eq!(
    client.ui().element(RENDER_MODE_DETAILS_ID).style().display,
    Prop::Set(StyleValue::Value(Display::None))
  );
  assert!(contains_render_texture(&client.ui(), PAGE_ID));

  client.ui().click(RENDER_MODE_DETAILS_BUTTON_ID);
  assert_eq!(
    client.ui().element(RENDER_MODE_DETAILS_ID).style().display,
    Prop::Set(StyleValue::Value(Display::Flex))
  );
  let expanded_text = collect_text(&client.ui(), PAGE_ID);
  assert!(expanded_text.contains("Physical Size · scales from display DPI"));
  assert!(expanded_text.contains("Screen Size · scales from viewport dimensions"));
  assert!(expanded_text.contains("Pointer input requires coordinate mapping"));
  client.ui().click(RENDER_MODE_DETAILS_BUTTON_ID);
  assert_eq!(
    client.ui().element(RENDER_MODE_DETAILS_ID).style().display,
    Prop::Set(StyleValue::Value(Display::None))
  );
}

#[test]
fn world_space_page_explains_three_modes_and_records_one_ui_action() {
  let mut client = FakeClient::connect(
    battlement_rules::create_engine().expect("UI sample engine should initialize"),
    sample_assets(),
  );
  let GameObjectKind::UiDocument(world) = client.assert_object(WORLD_DOCUMENT_ID).kind() else {
    panic!("world document must use UI document host state");
  };
  assert_eq!(
    world.panel_settings.render_mode,
    PanelRenderMode::WorldSpace
  );

  client.ui().click(WORLD_SPACE_BUTTON_ID);
  let text = collect_text(&client.ui(), PAGE_ID);
  assert!(text.contains("One input route. Three panel modes."));
  assert!(text.contains("Always + collider filtered"));
  client.ui().click(WORLD_BUTTON_ID);

  assert_eq!(
    client.ui().element(WORLD_STATUS_ID).text(),
    Some("UI action count  /  1")
  );
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

fn contains_render_texture(
  ui: &UiClient<'_, battlement_rules::UiLabEngine>,
  object_id: ObjectId,
) -> bool {
  let element = ui.element(object_id);
  element.image_source() == Some(&ImageSource::RenderTexture(assets::RENDER_TEXTURE.clone()))
    || element
      .children()
      .iter()
      .any(|child| contains_render_texture(ui, *child))
}
