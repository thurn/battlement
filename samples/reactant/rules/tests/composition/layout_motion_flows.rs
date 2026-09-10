use super::*;

#[test]
fn sample_opens_on_an_accessible_composition_screen() {
  let engine = create_engine();
  assert_eq!(engine.model().screen(), Screen::Composition);
  let mut client = FakeClient::connect(engine, catalog());
  let ui = client.ui();
  let navigation = find_named(&ui, ROOT_ID, "navigation");
  assert_eq!(
    ui.element(find_named(&ui, navigation, "composition-navigation"))
      .text(),
    Some("01  COMPOSITION")
  );
  assert_accessible_text(&ui, ROOT_ID, None, None, None);
}

#[test]
fn sample_uses_top_navigation_for_narrow_connections() {
  let engine = create_engine();
  let mut client = FakeClient::connect_with(
    engine,
    catalog(),
    Connect::new("test", "test", ScreenSize::new(900, 720)),
  );
  let ui = client.ui();
  let shell = find_named(&ui, ROOT_ID, "sample-shell");
  let navigation = find_named(&ui, ROOT_ID, "navigation");
  let items = find_named(&ui, navigation, "navigation-items");
  assert_eq!(
    ui.element(shell).style().flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::Column))
  );
  assert_eq!(
    ui.element(items).style().flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::Row))
  );
}

#[test]
fn resources_screen_uses_phone_safe_navigation_and_cards() {
  let engine = create_engine();
  let mut client = FakeClient::connect_with(
    engine,
    catalog(),
    Connect::new("test", "test", ScreenSize::new(360, 800)),
  );
  let navigation = find_named(&client.ui(), ROOT_ID, "next-navigation");
  assert_eq!(
    style_length_or_auto(&client.ui().element(navigation).style().height),
    Some(44.0)
  );
  let current = find_named(&client.ui(), ROOT_ID, "phone-current-screen");
  assert_eq!(client.ui().element(current).text(), Some("01 COMPOSITION"));
  for _ in 0..5 {
    client.ui().click(navigation);
  }
  assert_eq!(client.ui().element(current).text(), Some("06 RESOURCES"));
  let canvas = find_named(&client.ui(), ROOT_ID, "resources-canvas");
  let group = find_named(&client.ui(), canvas, "resources-card-group");
  assert_eq!(
    client.ui().element(group).style().flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::Column))
  );
}

#[test]
fn sample_recomposes_when_the_viewport_crosses_the_compact_breakpoint() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let shell = find_named(&client.ui(), ROOT_ID, "sample-shell");

  self::resize_viewport(&mut client, 1, 500.0, 700.0);
  assert_eq!(
    client.ui().element(shell).style().flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::Column))
  );
  let navigation = find_named(&client.ui(), shell, "navigation");
  let current = find_named(&client.ui(), navigation, "phone-current-screen");
  assert_eq!(client.ui().element(current).text(), Some("01 COMPOSITION"));

  self::resize_viewport(&mut client, 2, 1_280.0, 720.0);
  assert_eq!(
    client.ui().element(shell).style().flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::Row))
  );
  let navigation = find_named(&client.ui(), shell, "navigation");
  let items = find_named(&client.ui(), navigation, "navigation-items");
  assert_eq!(client.ui().element(items).children().len(), 8);
}

#[test]
fn assets_screen_prepares_mockup_paint_and_resizes_then_restores_the_action_frame() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let addresses = generated_asset_addresses();
  assert_eq!(addresses.len(), 18);
  assert_eq!(addresses.iter().cloned().collect::<BTreeSet<_>>().len(), 18);
  for address in &addresses {
    assert!(
      client
        .world()
        .prepared_assets()
        .contains(&PreparedAsset::texture(address.clone())),
      "initial snapshot omitted linked asset {address}"
    );
  }

  let navigation = find_named(&client.ui(), ROOT_ID, "assets-navigation");
  client.ui().click(navigation);
  let canvas = find_named(&client.ui(), ROOT_ID, "assets-canvas");
  let action = find_named(&client.ui(), canvas, "assets-resize-action");
  let initial = visible_text(&client.ui(), canvas);
  for name in [
    "assets-game-logo",
    "assets-label-play",
    "assets-label-settings",
    "assets-label-about",
    "assets-label-quit",
    "assets-label-return",
    "assets-arcade-screen-frame",
    "assets-settings-panel-frame",
    "assets-small-control-frame",
    "assets-settings-tab-active",
    "assets-settings-tab-inactive",
    "assets-checkbox-unchecked",
    "assets-checkbox-check",
    "assets-volume-slider-track",
    "assets-volume-slider-fill",
    "assets-volume-slider-ticks",
    "assets-volume-slider-handle",
  ] {
    find_named(&client.ui(), canvas, name);
  }
  assert_eq!(
    client.ui().element(action).text(),
    Some("STRETCH ACTION FRAME")
  );
  assert_eq!(
    style_length_or_auto(&client.ui().element(action).style().width),
    Some(420.0)
  );
  assert!(matches!(
    client.ui().element(action).style().unity_slice_top,
    Prop::Set(StyleValue::Value(value)) if value == 48
  ));

  client.ui().click(action);
  let canvas = find_named(&client.ui(), ROOT_ID, "assets-canvas");
  let action = find_named(&client.ui(), canvas, "assets-resize-action");
  assert_eq!(
    client.ui().element(action).text(),
    Some("RESTORE ACTION FRAME")
  );
  assert_eq!(
    style_length_or_auto(&client.ui().element(action).style().width),
    Some(610.0)
  );

  client.ui().click(action);
  let canvas = find_named(&client.ui(), ROOT_ID, "assets-canvas");
  assert_eq!(visible_text(&client.ui(), canvas), initial);
}

#[test]
fn variants_screen_propagates_ordered_snapshotted_targets_and_reverses_cleanly() {
  let engine = create_engine();
  let mut client = FakeClient::connect_with(
    engine,
    catalog(),
    Connect::new("test", "test", ScreenSize::new(360, 800)),
  );
  let next = find_named(&client.ui(), ROOT_ID, "next-navigation");
  for _ in 0..11 {
    client.ui().click(next);
  }

  let canvas = find_named(&client.ui(), ROOT_ID, "variants-orchestration-canvas");
  let title = find_named(&client.ui(), canvas, "page-title");
  assert_eq!(
    client.ui().element(title).text(),
    Some("Variants & Orchestration")
  );
  let first = find_named(&client.ui(), canvas, "variant-child-0");
  let opted_out = find_named(&client.ui(), canvas, "variant-child-2");
  let descriptor = motion_descriptor(&client.ui(), first);
  let facts = descriptor.variants.as_ref().unwrap();
  assert_eq!(facts.names, ["East", "Custom", "Forward"]);
  assert!(facts.inherited);
  assert_eq!(facts.child_index, 0);
  assert_eq!(facts.delay_micros, 320_000);
  assert_eq!(facts.when, VariantWhen::BeforeChildren);
  assert_eq!(facts.stagger_direction, StaggerDirection::Forward);
  assert!(
    motion_descriptor(&client.ui(), opted_out)
      .variants
      .is_none()
  );
  assert_eq!(
    motion_scalar(&descriptor, battlement::MotionProperty::Opacity),
    1.0,
    "the last selected variant must win ordered property conflicts"
  );

  let custom = find_named(&client.ui(), canvas, "variants-custom");
  client.ui().click(custom);
  let after_custom = motion_descriptor(&client.ui(), first);
  assert_eq!(after_custom.generation, descriptor.generation);
  assert_eq!(
    after_custom.variants.as_ref().unwrap().custom_snapshot,
    facts.custom_snapshot
  );

  let route = find_named(&client.ui(), canvas, "variants-route");
  client.ui().click(route);
  let west = motion_descriptor(&client.ui(), first);
  assert_eq!(west.variants.as_ref().unwrap().names[0], "West");
  assert!(west.generation.0 > descriptor.generation.0);
  assert!(motion_scalar(&west, battlement::MotionProperty::X) < 0.0);

  let stagger = find_named(&client.ui(), canvas, "variants-stagger");
  client.ui().click(stagger);
  let reverse = motion_descriptor(&client.ui(), first);
  let facts = reverse.variants.as_ref().unwrap();
  assert_eq!(facts.names[2], "Reverse");
  assert_eq!(facts.delay_micros, 500_000);
  assert_eq!(facts.stagger_direction, StaggerDirection::Reverse);
}

#[test]
fn composition_action_reorders_and_restores_the_badges() {
  let correlations = Rc::new(RefCell::new(Vec::new()));
  let engine = CorrelationEngine {
    inner: create_engine(),
    correlations: Rc::clone(&correlations),
  };
  let mut client = FakeClient::connect(engine, catalog());
  let action = find_named(&client.ui(), ROOT_ID, "composition-action");
  let badges = find_named(&client.ui(), ROOT_ID, "composition-badges");
  let initial = self::child_text(&client.ui(), badges);

  client.ui().click(action);
  assert_eq!(client.ui().element(action).text(), Some("RESTORE"));
  assert_eq!(
    self::child_text(&client.ui(), badges),
    initial.iter().rev().cloned().collect::<Vec<_>>()
  );

  client.ui().send_event(UiEvent {
    target_id: action,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::PointerDown(self::pointer_button_event()),
  });
  let pressed = style_color(&client.ui().element(action).style().background_color)
    .expect("pressed state action background should be authored");
  client.ui().click(action);
  assert_eq!(client.ui().element(action).text(), Some("REORDER"));
  assert_ne!(
    style_color(&client.ui().element(action).style().background_color),
    Some(pressed)
  );
  assert_eq!(self::child_text(&client.ui(), badges), initial);
  for (action_id, causes) in correlations.borrow().iter() {
    assert_eq!(causes, &[Some(*action_id)]);
  }
  assert_eq!(correlations.borrow().len(), 3);
}

#[test]
fn buttons_render_distinct_hover_pressed_and_focus_states() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "composition-navigation");
  let selected = style_color(&client.ui().element(navigation).style().background_color)
    .expect("selected navigation background should be authored");
  client.ui().send_event(UiEvent {
    target_id: navigation,
    cancelable: false,
    default_prevented: false,
    body: UiEventBody::Focus(FocusEvent::default()),
  });
  assert_eq!(
    style_color(&client.ui().element(navigation).style().background_color),
    Some(selected)
  );
  assert_ne!(
    client.ui().element(navigation).style().border_top_width,
    Prop::Unset
  );
  client.ui().send_event(UiEvent {
    target_id: navigation,
    cancelable: false,
    default_prevented: false,
    body: UiEventBody::Blur(FocusEvent::default()),
  });
  let action = find_named(&client.ui(), ROOT_ID, "composition-action");
  let resting = style_color(&client.ui().element(action).style().background_color)
    .expect("resting action background should be authored");
  let resting_border = client.ui().element(action).style().border_top_width;

  client.ui().send_event(UiEvent {
    target_id: action,
    cancelable: false,
    default_prevented: false,
    body: UiEventBody::PointerEnter(PointerBoundaryEvent {
      pointer_id: 4,
      position: PanelPoint::default(),
      pointer_type: PointerType::Mouse,
    }),
  });
  let hovered = style_color(&client.ui().element(action).style().background_color)
    .expect("hovered action background should be authored");
  assert_ne!(hovered, resting);

  client.ui().send_event(UiEvent {
    target_id: action,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::PointerDown(self::pointer_button_event()),
  });
  let pressed = style_color(&client.ui().element(action).style().background_color)
    .expect("pressed action background should be authored");
  assert_ne!(pressed, hovered);

  client.ui().send_event(UiEvent {
    target_id: action,
    cancelable: true,
    default_prevented: false,
    body: UiEventBody::PointerUp(self::pointer_button_event()),
  });
  client.ui().send_event(UiEvent {
    target_id: action,
    cancelable: false,
    default_prevented: false,
    body: UiEventBody::Focus(FocusEvent::default()),
  });
  assert_ne!(
    client.ui().element(action).style().border_top_width,
    resting_border
  );

  client.ui().send_event(UiEvent {
    target_id: action,
    cancelable: false,
    default_prevented: false,
    body: UiEventBody::Blur(FocusEvent::default()),
  });
  client.ui().send_event(UiEvent {
    target_id: action,
    cancelable: false,
    default_prevented: false,
    body: UiEventBody::PointerLeave(PointerBoundaryEvent {
      pointer_id: 4,
      position: PanelPoint::default(),
      pointer_type: PointerType::Mouse,
    }),
  });
  assert_eq!(
    style_color(&client.ui().element(action).style().background_color),
    Some(resting)
  );
}

#[test]
fn composed_effects_preserve_finite_ambient_reduced_and_reconnect_contracts() {
  let engine = create_engine();
  let mut client = FakeClient::connect_with(
    engine,
    catalog(),
    Connect::new("test", "test", ScreenSize::new(360, 800)),
  );
  let next = find_named(&client.ui(), ROOT_ID, "next-navigation");
  for _ in 0..17 {
    client.ui().click(next);
  }

  let canvas = find_named(&client.ui(), ROOT_ID, "composed-effects-canvas");
  let option = find_named(&client.ui(), canvas, "composed-option-0");
  let finite = motion_descriptor(&client.ui(), option);
  assert_eq!(finite.reduced_motion, battlement::ReducedMotionPolicy::User);
  let x = finite.slots[0]
    .target
    .tracks
    .iter()
    .find(|track| track.property == battlement::MotionProperty::X)
    .expect("staggered option should animate x");
  assert!(matches!(
    x.transition.generator,
    battlement::TransitionGenerator::Tween {
      duration_micros: 180_000,
      ..
    }
  ));

  let grid = find_named(&client.ui(), canvas, "composed-grid");
  let ambient = motion_descriptor(&client.ui(), grid);
  assert!(matches!(
    ambient.animations[0].tracks[0].transition.repeat,
    battlement::MotionRepeat::Forever
  ));
  let host_id = ambient.host_id;

  let reduced = find_named(&client.ui(), canvas, "composed-reduced");
  client.ui().click(reduced);
  let grid = find_named(&client.ui(), ROOT_ID, "composed-grid");
  let reduced_descriptor = motion_descriptor(&client.ui(), grid);
  assert_eq!(
    reduced_descriptor.reduced_motion,
    battlement::ReducedMotionPolicy::Always
  );
  let generation = reduced_descriptor.generation;

  let reconnect = find_named(&client.ui(), ROOT_ID, "composed-reconnect");
  client.ui().click(reconnect);
  let restored_grid = find_named(&client.ui(), ROOT_ID, "composed-grid");
  let restored = motion_descriptor(&client.ui(), restored_grid);
  assert_eq!(restored.generation, generation);
  assert_eq!(restored.host_id, host_id);
  let restored_status = find_named(&client.ui(), ROOT_ID, "composed-status");
  assert!(
    client
      .ui()
      .element(restored_status)
      .text()
      .is_some_and(|text| text.contains("RECONNECTS 1"))
  );
}

#[test]
fn layout_gallery_preserves_state_routes_portals_and_authors_modal_focus() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  navigate_brand(
    &mut client,
    &[
      "targets-timelines-navigation",
      "values-navigation",
      "gestures-navigation",
      "layout-gallery-navigation",
    ],
  );

  let canvas = find_named(&client.ui(), ROOT_ID, "layout-gallery-canvas");
  let tabs = find_named(&client.ui(), canvas, "layout-gallery-tabs");
  let selected_tab = {
    let ui = client.ui();
    let UiElement::TabView(tabs_element) = ui.element(tabs).element() else {
      panic!("gallery tabs should use the native TabView host");
    };
    tabs_element.selected_tab_index
  };
  assert_eq!(selected_tab, Prop::Set(0));

  let value = find_named(&client.ui(), canvas, "layout-setting-value-music");
  client.ui().click(value);
  assert_eq!(client.ui().element(value).text(), Some("VALUE 1"));
  let settings = find_named(&client.ui(), canvas, "layout-gallery-settings");
  let initial_columns = {
    let ui = client.ui();
    let UiElement::Grid(initial_settings) = ui.element(settings).element() else {
      panic!("gallery settings should use the public Grid host");
    };
    initial_settings.columns.clone()
  };
  assert!(matches!(
    initial_columns,
    Prop::Set(ref columns) if columns.len() == 3
  ));

  let tracks = find_named(&client.ui(), canvas, "layout-gallery-tracks");
  assert_eq!(
    client
      .ui()
      .element(tracks)
      .element()
      .visual_element()
      .auto_focus,
    Prop::Set(true)
  );
  let inert_region = find_named(&client.ui(), canvas, "layout-gallery-inert-region");
  assert_eq!(
    client
      .ui()
      .element(inert_region)
      .element()
      .visual_element()
      .inert,
    Prop::Set(false)
  );
  let inert_toggle = find_named(&client.ui(), canvas, "layout-gallery-inert-toggle");
  client.ui().click(inert_toggle);
  assert_eq!(
    client
      .ui()
      .element(inert_region)
      .element()
      .visual_element()
      .inert,
    Prop::Set(true)
  );
  client.ui().click(tracks);
  assert_eq!(
    find_named(&client.ui(), ROOT_ID, "layout-setting-value-music"),
    value,
    "responsive tracks must preserve keyed component state"
  );
  assert_eq!(client.ui().element(value).text(), Some("VALUE 1"));
  let settings = find_named(&client.ui(), ROOT_ID, "layout-gallery-settings");
  let compact_columns = {
    let ui = client.ui();
    let UiElement::Grid(compact_settings) = ui.element(settings).element() else {
      panic!("gallery settings should remain a Grid host");
    };
    compact_settings.columns.clone()
  };
  assert!(matches!(
    compact_columns,
    Prop::Set(ref columns) if columns.len() == 2
  ));

  let header = find_named(&client.ui(), ROOT_ID, "layout-gallery-table-header");
  assert!(matches!(
    client.ui().element(header).element().visual_element().sticky,
    Prop::Set(ref sticky) if sticky.top == Some(0.0) && sticky.order == 4
  ));

  let trigger = find_named(&client.ui(), ROOT_ID, "layout-gallery-menu-trigger");
  client.ui().click(trigger);
  let menu = find_named(&client.ui(), ROOT_ID, "layout-gallery-menu");
  assert!(matches!(
    client.ui().element(menu).element().visual_element().overlay_placement,
    Prop::Set(OverlayPlacement::Popover { anchor, .. }) if anchor == trigger
  ));
  let status = find_named(&client.ui(), ROOT_ID, "layout-gallery-status");
  assert!(
    client
      .ui()
      .element(status)
      .text()
      .is_some_and(|text| text.contains("CAPTURE > ANCHOR > BUBBLE"))
  );
  let action = find_named(&client.ui(), menu, "layout-gallery-menu-action");
  client.ui().click(action);
  assert!(!client.ui().contains(menu));
  let status = find_named(&client.ui(), ROOT_ID, "layout-gallery-status");
  assert!(
    client
      .ui()
      .element(status)
      .text()
      .is_some_and(|text| text.contains("CAPTURE > TARGET > BUBBLE"))
  );

  let modal_trigger = find_named(&client.ui(), ROOT_ID, "layout-gallery-modal");
  client.ui().click(modal_trigger);
  let modal = find_named(&client.ui(), ROOT_ID, "layout-gallery-modal-scope");
  let close = find_named(&client.ui(), modal, "layout-gallery-modal-close");
  assert!(matches!(
    client.ui().element(modal).element().visual_element().overlay_placement,
    Prop::Set(OverlayPlacement::Modal {
      initial_focus: Some(initial),
      restore_focus: Some(restore),
    }) if initial == close && restore == modal_trigger
  ));
  assert!(
    motion_descriptor(&client.ui(), close)
      .slots
      .iter()
      .any(|slot| slot.layer == battlement::MotionLayer::FocusVisible)
  );
  client.ui().click(close);
  assert!(!client.ui().contains(modal));

  let reconnect = find_named(&client.ui(), ROOT_ID, "layout-gallery-reconnect");
  client.ui().click(reconnect);
  assert_eq!(
    find_named(&client.ui(), ROOT_ID, "layout-setting-value-music"),
    value
  );
  let status = find_named(&client.ui(), ROOT_ID, "layout-gallery-status");
  assert!(
    client
      .ui()
      .element(status)
      .text()
      .is_some_and(|text| text.contains("RECONNECTS 1"))
  );
}

#[test]
fn layout_performance_builds_the_exact_mixed_workload() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  navigate_brand(
    &mut client,
    &[
      "targets-timelines-navigation",
      "values-navigation",
      "gestures-navigation",
      "layout-gallery-navigation",
      "layout-reorder-navigation",
      "composed-effects-navigation",
      "layout-performance-navigation",
    ],
  );

  let grid = find_named(&client.ui(), ROOT_ID, "layout-performance-grid");
  assert_eq!(client.ui().element(grid).kind(), UiElementKind::Grid);
  assert_eq!(client.ui().element(grid).children().len(), 1_000);
  let stacks = find_named(&client.ui(), ROOT_ID, "layout-performance-stacks");
  assert_eq!(client.ui().element(stacks).children().len(), 12);
  let sticky_scroll = find_named(&client.ui(), ROOT_ID, "layout-performance-sticky-scroll");
  assert_eq!(count_sticky(&client.ui(), sticky_scroll), 100);
  for index in 0..10 {
    let overlay = find_named(
      &client.ui(),
      ROOT_ID,
      &format!("layout-performance-overlay-{index}"),
    );
    assert!(matches!(
      client
        .ui()
        .element(overlay)
        .element()
        .visual_element()
        .overlay_placement,
      Prop::Set(OverlayPlacement::Popover { .. })
    ));
  }
}

#[test]
fn motion_performance_builds_the_exact_transform_workload() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  for navigation in [
    "targets-timelines-navigation",
    "values-navigation",
    "gestures-navigation",
    "layout-gallery-navigation",
    "layout-reorder-navigation",
    "composed-effects-navigation",
    "layout-performance-navigation",
    "motion-performance-navigation",
  ] {
    let target = find_named(&client.ui(), ROOT_ID, navigation);
    client.ui().send_event(UiEvent::click(
      target,
      battlement::ClickEvent::pointer(
        0,
        PanelPoint::default(),
        PointerButton::Left,
        1,
        KeyModifiers::default(),
      ),
    ));
  }

  let grid = find_named(&client.ui(), ROOT_ID, "motion-performance-grid");
  let hosts = client.ui().element(grid).children().to_vec();
  assert_eq!(hosts.len(), 200);
  let descriptors = hosts
    .iter()
    .map(|host| motion_descriptor(&client.ui(), *host))
    .collect::<Vec<_>>();
  assert_eq!(
    descriptors
      .iter()
      .map(|descriptor| descriptor.values.len())
      .sum::<usize>(),
    120
  );
  assert_eq!(
    descriptors
      .iter()
      .map(|descriptor| descriptor.value_subscriptions.len())
      .sum::<usize>(),
    0
  );
  assert_eq!(
    descriptors
      .iter()
      .map(|descriptor| descriptor.slots.len())
      .sum::<usize>(),
    320
  );
  for scenario in ["performance-mixed", "performance-interaction"] {
    let target = find_named(&client.ui(), ROOT_ID, scenario);
    client.ui().send_event(UiEvent::click(
      target,
      battlement::ClickEvent::pointer(
        0,
        PanelPoint::default(),
        PointerButton::Left,
        1,
        KeyModifiers::default(),
      ),
    ));
    let grid = find_named(&client.ui(), ROOT_ID, "motion-performance-grid");
    assert_eq!(client.ui().element(grid).children().len(), 200);
  }
}

fn viewport_rect(x: f64, y: f64, width: f64, height: f64) -> ViewportRect {
  ViewportRect {
    x,
    y,
    width,
    height,
    display_id: DisplayId(0),
  }
}

fn identity_projective() -> Projective2 {
  Projective2 {
    m11: 1.0,
    m12: 0.0,
    m13: 0.0,
    m21: 0.0,
    m22: 1.0,
    m23: 0.0,
    m31: 0.0,
    m32: 0.0,
    m33: 1.0,
  }
}

fn generation(value: u64) -> GeometryGeneration {
  GeometryGeneration(NonZeroU64::new(value).unwrap())
}

fn pointer_button_event() -> PointerButtonEvent {
  PointerButtonEvent {
    pointer_id: 4,
    position: PanelPoint::default(),
    delta: Vector::default(),
    button: PointerButton::Left,
    buttons: 1,
    pressure: 1.0,
    click_count: 1,
    modifiers: KeyModifiers::default(),
    pointer_type: PointerType::Mouse,
  }
}

fn added_observations(commands: &[ExecutedCommand]) -> Vec<GeometryObservation> {
  commands
    .iter()
    .filter_map(|entry| match &entry.command.body {
      battlement::CommandBody::GeometryObservationUpdate(update) => Some(&update.added),
      _ => None,
    })
    .flatten()
    .cloned()
    .collect()
}

fn sample_geometry(observation: &GeometryObservation) -> GeometryObservationValue {
  let result = match observation.target {
    GeometryObservationTarget::UiElement { object_id } => {
      GeometryObservationResult::Current(GeometryValue::Element(ElementGeometry {
        layout: Rect::new(0.0, 0.0, 520.0, 56.0),
        viewport_bound: self::viewport_rect(410.0, 250.0, 520.0, 56.0),
        viewport_from_local: self::identity_projective(),
        viewport_from_parent: self::identity_projective(),
        panel_id: object_id,
      }))
    }
    GeometryObservationTarget::Viewport { .. } => {
      GeometryObservationResult::Current(GeometryValue::Viewport(ViewportGeometry {
        viewport: self::viewport_rect(0.0, 0.0, 1_280.0, 720.0),
        safe_area: self::viewport_rect(0.0, 0.0, 1_280.0, 700.0),
        scale: 1.0,
        dpi: Some(96.0),
        orientation: DisplayOrientation::Landscape,
      }))
    }
    GeometryObservationTarget::WorldOrigin { .. } => {
      GeometryObservationResult::Current(GeometryValue::WorldPoint(WorldPointGeometry {
        point: ViewportPoint {
          x: 842.0,
          y: 446.0,
          display_id: DisplayId(0),
        },
        depth: 10.0,
        is_inside_viewport: true,
      }))
    }
    GeometryObservationTarget::WorldRenderedBounds { .. } => {
      GeometryObservationResult::Current(GeometryValue::WorldBounds(WorldBoundsGeometry {
        bound: self::viewport_rect(790.0, 394.0, 104.0, 104.0),
        nearest_depth: 9.3,
        farthest_depth: 10.7,
        is_inside_viewport: true,
      }))
    }
    GeometryObservationTarget::WorldAnchor { .. } => panic!("sample does not observe an anchor"),
  };
  GeometryObservationValue {
    observation_id: observation.observation_id,
    result,
  }
}

fn visible_text<E>(ui: &UiClient<'_, E>, root: ObjectId) -> Vec<String>
where
  E: Engine<Command = Command>,
{
  let mut pending = vec![root];
  let mut text = Vec::new();
  while let Some(object_id) = pending.pop() {
    let element = ui.element(object_id);
    if let Some(value) = element.text() {
      text.push(value.to_owned());
    }
    pending.extend(element.children().iter().rev());
  }
  text
}

fn assert_accessible_text(
  ui: &UiClient<'_, ReactantEngine>,
  object_id: ObjectId,
  inherited_color: Option<Color>,
  inherited_background: Option<Color>,
  inherited_size: Option<f32>,
) {
  let element = ui.element(object_id);
  let color = style_color(&element.style().color).or(inherited_color);
  let background = style_color(&element.style().background_color).or(inherited_background);
  let size = style_length(&element.style().font_size).or(inherited_size);
  if matches!(element.kind(), UiElementKind::Label | UiElementKind::Button) {
    assert!(size.expect("visible text must have a resolved size") >= 12.0);
    assert!(
      contrast(
        color.expect("visible text must have a resolved color"),
        background.expect("visible text must have a resolved background"),
      ) >= 4.5
    );
  }
  for child in element.children() {
    assert_accessible_text(ui, *child, color, background, size);
  }
}

fn style_color(value: &Prop<StyleValue<Color>>) -> Option<Color> {
  match value {
    Prop::Set(StyleValue::Value(color)) => Some(*color),
    _ => None,
  }
}

fn style_length(value: &Prop<StyleValue<Length>>) -> Option<f32> {
  match value {
    Prop::Set(StyleValue::Value(Length::Px(value))) => Some(*value),
    _ => None,
  }
}

fn style_length_or_auto(value: &Prop<StyleValue<LengthOrAuto>>) -> Option<f32> {
  match value {
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(value))) => Some(*value),
    _ => None,
  }
}

fn contrast(foreground: Color, background: Color) -> f64 {
  let foreground = relative_luminance(foreground);
  let background = relative_luminance(background);
  let bright = foreground.max(background);
  let dark = foreground.min(background);
  (bright + 0.05) / (dark + 0.05)
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

fn child_text<E>(ui: &UiClient<'_, E>, root: ObjectId) -> Vec<String>
where
  E: Engine<Command = Command>,
{
  ui.element(root)
    .children()
    .iter()
    .map(|object_id| {
      ui.element(*object_id)
        .children()
        .first()
        .and_then(|child| ui.element(*child).text())
        .expect("badge has text")
        .to_owned()
    })
    .collect()
}

fn motion_descriptor<E>(ui: &UiClient<'_, E>, object_id: ObjectId) -> battlement::MotionDescriptor
where
  E: Engine<Command = Command>,
{
  match &ui.element(object_id).element().visual_element().motion {
    Prop::Set(value) => value.clone(),
    value => panic!("expected a motion descriptor, received {value:?}"),
  }
}

fn motion_scalar(
  descriptor: &battlement::MotionDescriptor,
  property: battlement::MotionProperty,
) -> f32 {
  let value = descriptor.slots[0]
    .target
    .tracks
    .iter()
    .find(|track| track.property == property)
    .and_then(|track| track.values.last())
    .expect("motion property should be present");
  match value {
    battlement::MotionValue::Scalar(value) => *value,
    battlement::MotionValue::Length(value) if value.components()[1] == 0.0 => value.components()[0],
    value => panic!("motion property is not scalar-like: {value:?}"),
  }
}

fn count_sticky<E>(ui: &UiClient<'_, E>, root: ObjectId) -> usize
where
  E: Engine<Command = Command>,
{
  let mut count = 0;
  let mut pending = vec![root];
  while let Some(object_id) = pending.pop() {
    let element = ui.element(object_id);
    if matches!(element.element().visual_element().sticky, Prop::Set(_)) {
      count += 1;
    }
    pending.extend(element.children());
  }
  count
}

fn resize_viewport(
  client: &mut FakeClient<ReactantEngine>,
  generation: u64,
  width: f64,
  height: f64,
) {
  let observation = self::added_observations(client.commands())
    .into_iter()
    .find(|observation| {
      matches!(
        observation.target,
        GeometryObservationTarget::Viewport { .. }
      )
    })
    .expect("application viewport observation");
  let mut value = self::sample_geometry(&observation);
  if let GeometryObservationResult::Current(GeometryValue::Viewport(geometry)) = &mut value.result {
    geometry.viewport.width = width;
    geometry.viewport.height = height;
    geometry.safe_area = geometry.viewport;
  }
  client.submit_geometry(GeometryObservationBatch {
    generation: self::generation(generation),
    changed: vec![value],
  });
}
