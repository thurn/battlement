use super::*;

#[test]
fn events_screen_runs_and_restores_one_logical_event_path() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "events-navigation");
  client.ui().click(navigation);

  let canvas = find_named(&client.ui(), ROOT_ID, "events-canvas");
  let action = find_named(&client.ui(), ROOT_ID, "events-action");
  let source = find_named(&client.ui(), canvas, "event-source");
  let layer = find_named(&client.ui(), ROOT_ID, "portal-layer");
  let overlay = find_named(&client.ui(), ROOT_ID, "portal-overlay");
  let status = find_named(&client.ui(), canvas, "events-status");
  let initial = self::visible_text(&client.ui(), canvas);
  assert!(visible_word_count(&client.ui(), canvas) <= EVENTS_WORD_BUDGET);
  assert_eq!(client.ui().element(action).text(), Some("RUN EVENT"));
  assert_eq!(client.ui().element(status).text(), Some("READY"));
  assert!(!client.ui().element(source).children().contains(&action));
  assert_eq!(client.ui().element(layer).kind(), UiElementKind::Stack);
  assert_eq!(
    client.ui().element(layer).picking_mode(),
    Some(battlement::PickingMode::Ignore)
  );
  assert_eq!(client.ui().element(layer).children(), &[overlay]);
  assert!(client.ui().element(overlay).children().contains(&action));

  client.ui().click(action);
  assert_eq!(client.ui().element(action).text(), Some("RESTORE"));
  let active_status = find_named(&client.ui(), canvas, "events-status");
  assert_eq!(
    self::visible_text(&client.ui(), active_status),
    ["CAPTURE", ">", "TARGET", ">", "BUBBLE"]
  );
  assert!(visible_word_count(&client.ui(), canvas) <= EVENTS_WORD_BUDGET);
  assert_accessible_text(&client.ui(), ROOT_ID, None, None, None);

  client.ui().click(action);
  assert_eq!(self::visible_text(&client.ui(), canvas), initial);
  assert_eq!(client.ui().element(action).text(), Some("RUN EVENT"));
  let restored_status = find_named(&client.ui(), canvas, "events-status");
  assert_eq!(client.ui().element(restored_status).text(), Some("READY"));
}

#[test]
fn state_screen_batches_updates_preserves_keyed_state_and_restores() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "state-navigation");
  client.ui().click(navigation);

  let canvas = find_named(&client.ui(), ROOT_ID, "state-canvas");
  let action = find_named(&client.ui(), canvas, "state-action");
  let tokens = find_named(&client.ui(), canvas, "identity-tokens");
  let initial = self::visible_text(&client.ui(), canvas);
  assert!(visible_word_count(&client.ui(), canvas) <= STATE_WORD_BUDGET);
  assert_eq!(client.ui().element(action).text(), Some("QUEUE +3"));
  assert_eq!(
    self::identity_labels(&client.ui(), tokens),
    ["01  ALPHA", "02  BRAVO", "03  CHARLIE"]
  );
  assert_eq!(
    self::identity_states(&client.ui(), tokens),
    ["REDUCER 0", "REDUCER 0", "REDUCER 0"]
  );
  let token_ids = client.ui().element(tokens).children().to_vec();
  assert!(token_ids.iter().all(|token| {
    style_length_or_auto(&client.ui().element(*token).style().width) == Some(180.0)
  }));

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
  let value = find_named(&client.ui(), canvas, "state-value");
  assert_eq!(client.ui().element(value).text(), Some("BATCHED VALUE  3"));
  assert_eq!(
    self::identity_states(&client.ui(), tokens),
    ["REDUCER 1", "REDUCER 1", "REDUCER 1"]
  );

  client.ui().click(action);
  assert_eq!(client.ui().element(action).text(), Some("RESTORE"));
  assert_eq!(
    self::identity_labels(&client.ui(), tokens),
    ["03  CHARLIE", "02  BRAVO", "01  ALPHA"]
  );
  assert_eq!(
    self::identity_states(&client.ui(), tokens),
    ["REDUCER 1", "REDUCER 1", "REDUCER 1"]
  );
  assert_eq!(
    client.ui().element(tokens).children(),
    token_ids.iter().rev().copied().collect::<Vec<_>>()
  );

  client.ui().click(action);
  assert_eq!(self::visible_text(&client.ui(), canvas), initial);
  let restored_ids = client.ui().element(tokens).children().to_vec();
  assert!(
    restored_ids
      .iter()
      .all(|restored| !token_ids.contains(restored))
  );
  assert_eq!(
    self::identity_states(&client.ui(), tokens),
    ["REDUCER 0", "REDUCER 0", "REDUCER 0"]
  );
  assert_accessible_text(&client.ui(), ROOT_ID, None, None, None);
}

#[test]
fn context_screen_overrides_only_the_nested_descendant_and_restores() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "context-navigation");
  client.ui().click(navigation);

  let canvas = find_named(&client.ui(), ROOT_ID, "context-canvas");
  let action = find_named(&client.ui(), canvas, "context-action");
  let unrelated_action = find_named(&client.ui(), canvas, "context-unrelated-action");
  let outer = find_named(&client.ui(), canvas, "context-outer");
  let nested = find_named(&client.ui(), canvas, "context-nested");
  let initial = self::visible_text(&client.ui(), canvas);
  assert!(visible_word_count(&client.ui(), canvas) <= CONTEXT_WORD_BUDGET);
  assert_eq!(client.ui().element(action).text(), Some("OVERRIDE NESTED"));
  assert_eq!(
    client.ui().element(unrelated_action).text(),
    Some("CHANGE VALUE")
  );
  assert_eq!(
    self::visible_text(&client.ui(), outer),
    ["OUTER", "DEFAULT"]
  );
  assert_eq!(
    self::visible_text(&client.ui(), nested),
    ["NESTED", "DEFAULT"]
  );

  client.ui().click(unrelated_action);
  let canvas = find_named(&client.ui(), ROOT_ID, "context-canvas");
  let action = find_named(&client.ui(), canvas, "context-action");
  let unrelated_action = find_named(&client.ui(), canvas, "context-unrelated-action");
  let outer = find_named(&client.ui(), canvas, "context-outer");
  let nested = find_named(&client.ui(), canvas, "context-nested");
  assert_eq!(client.ui().element(action).text(), Some("OVERRIDE NESTED"));
  assert_eq!(
    client.ui().element(unrelated_action).text(),
    Some("RESET VALUE")
  );
  assert!(
    self::visible_text(&client.ui(), canvas)
      .iter()
      .any(|text| text == "VALUE  1")
  );
  assert_eq!(
    self::visible_text(&client.ui(), outer),
    ["OUTER", "DEFAULT"]
  );
  assert_eq!(
    self::visible_text(&client.ui(), nested),
    ["NESTED", "DEFAULT"]
  );

  client.ui().click(action);
  let canvas = find_named(&client.ui(), ROOT_ID, "context-canvas");
  let action = find_named(&client.ui(), canvas, "context-action");
  let outer = find_named(&client.ui(), canvas, "context-outer");
  let nested = find_named(&client.ui(), canvas, "context-nested");
  assert_eq!(client.ui().element(action).text(), Some("RESTORE DEFAULT"));
  assert_eq!(
    self::visible_text(&client.ui(), outer),
    ["OUTER", "DEFAULT"]
  );
  assert_eq!(
    self::visible_text(&client.ui(), nested),
    ["NESTED", "OVERRIDDEN"]
  );
  assert_accessible_text(&client.ui(), ROOT_ID, None, None, None);

  client.ui().click(action);
  let canvas = find_named(&client.ui(), ROOT_ID, "context-canvas");
  let unrelated_action = find_named(&client.ui(), canvas, "context-unrelated-action");
  client.ui().click(unrelated_action);
  let canvas = find_named(&client.ui(), ROOT_ID, "context-canvas");
  assert_eq!(self::visible_text(&client.ui(), canvas), initial);
}

#[test]
fn effects_screen_defers_connection_until_poll_and_restores() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "effects-navigation");
  client.ui().click(navigation);

  let canvas = find_named(&client.ui(), ROOT_ID, "effects-canvas");
  let action = find_named(&client.ui(), canvas, "effects-action");
  let status = find_named(&client.ui(), canvas, "effect-status");
  let initial = self::visible_text(&client.ui(), canvas);
  assert!(visible_word_count(&client.ui(), canvas) <= EFFECTS_WORD_BUDGET);
  assert_eq!(client.ui().element(action).text(), Some("CONNECT"));
  assert_eq!(client.ui().element(status).text(), Some("DISCONNECTED"));

  client.ui().click(action);
  assert_eq!(client.ui().element(action).text(), Some("RESTORE"));
  assert_eq!(client.ui().element(status).text(), Some("DISCONNECTED"));
  client.poll();
  assert_eq!(client.ui().element(status).text(), Some("CONNECTED"));

  client.ui().click(action);
  assert_eq!(client.ui().element(action).text(), Some("CONNECT"));
  assert_eq!(client.ui().element(status).text(), Some("CONNECTED"));
  client.poll();
  assert_eq!(self::visible_text(&client.ui(), canvas), initial);
  assert_accessible_text(&client.ui(), ROOT_ID, None, None, None);
}

#[test]
fn effects_store_swaps_updates_and_restores_its_external_snapshot() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "effects-navigation");
  client.ui().click(navigation);

  let canvas = find_named(&client.ui(), ROOT_ID, "effects-canvas");
  let action = find_named(&client.ui(), canvas, "store-action");
  let status = find_named(&client.ui(), canvas, "store-status");
  let initial = self::visible_text(&client.ui(), canvas);
  assert_eq!(client.ui().element(action).text(), Some("SWAP SOURCE"));
  assert_eq!(client.ui().element(status).text(), Some("SOURCE A  12"));

  client.ui().click(action);
  assert_eq!(client.ui().element(action).text(), Some("PUBLISH UPDATE"));
  assert_eq!(client.ui().element(status).text(), Some("SOURCE B  40"));

  client.ui().click(action);
  assert_eq!(client.ui().element(action).text(), Some("RESTORE"));
  assert_eq!(client.ui().element(status).text(), Some("SOURCE B  41"));

  client.ui().click(action);
  assert_eq!(self::visible_text(&client.ui(), canvas), initial);
  assert_eq!(client.ui().element(action).text(), Some("SWAP SOURCE"));
  assert_eq!(client.ui().element(status).text(), Some("SOURCE A  12"));
}

#[test]
fn resources_screen_catches_resets_and_restores() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "resources-navigation");
  client.ui().click(navigation);

  let canvas = find_named(&client.ui(), ROOT_ID, "resources-canvas");
  let group = find_named(&client.ui(), canvas, "resources-card-group");
  let pending = find_named(&client.ui(), canvas, "resource-pending");
  let resolve = find_named(&client.ui(), pending, "resource-resolve");
  let primary = find_named(&client.ui(), canvas, "boundary-primary");
  let action = find_named(&client.ui(), primary, "boundary-action");
  let boundary_initial = visible_text(&client.ui(), primary);
  assert_eq!(
    client.ui().element(group).style().flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::Row))
  );
  assert!(visible_word_count(&client.ui(), canvas) <= RESOURCES_WORD_BUDGET);
  assert_eq!(client.ui().element(action).text(), Some("TRIGGER ERROR"));
  assert_ne!(
    style_color(&client.ui().element(resolve).style().background_color),
    style_color(&client.ui().element(action).style().background_color)
  );
  assert_eq!(client.ui().element(pending).text(), None);
  assert_eq!(
    visible_text(&client.ui(), pending),
    ["RESOURCE PENDING", "RESOLVE RESOURCE"]
  );

  client.ui().click(resolve);
  let ready = find_named(&client.ui(), canvas, "resource-ready");
  let refetch = find_named(&client.ui(), ready, "resource-refetch");
  assert_eq!(
    visible_text(&client.ui(), ready),
    ["RESOURCE READY", "REFETCH RESOURCE"]
  );
  assert_eq!(
    style_color(&client.ui().element(refetch).style().background_color),
    style_color(&client.ui().element(action).style().background_color)
  );
  client.ui().click(refetch);
  let repeated_pending = find_named(&client.ui(), canvas, "resource-pending");
  let repeated_resolve = find_named(&client.ui(), repeated_pending, "resource-resolve");
  assert_eq!(
    visible_text(&client.ui(), repeated_pending),
    ["RESOURCE PENDING", "RESOLVE RESOURCE"]
  );
  assert_ne!(
    client.ui().element(canvas).style().display,
    Prop::Set(StyleValue::Value(Display::None))
  );
  assert_ne!(
    client.ui().element(repeated_pending).style().display,
    Prop::Set(StyleValue::Value(Display::None))
  );
  assert_eq!(
    visible_text(&client.ui(), repeated_pending),
    ["RESOURCE PENDING", "RESOLVE RESOURCE",]
  );
  client.ui().click(repeated_resolve);
  assert_eq!(find_named(&client.ui(), canvas, "resource-ready"), ready);

  client.ui().click(action);
  let fallback = find_named(&client.ui(), canvas, "boundary-fallback");
  let reset = find_named(&client.ui(), fallback, "boundary-reset");
  let error = find_named(&client.ui(), fallback, "boundary-error");
  assert_eq!(client.ui().element(reset).text(), Some("RESET BOUNDARY"));
  assert_ne!(
    style_color(&client.ui().element(reset).style().background_color),
    style_color(&client.ui().element(refetch).style().background_color)
  );
  assert_eq!(
    client.ui().element(error).text(),
    Some("resource preview failed")
  );
  assert!(visible_word_count(&client.ui(), canvas) <= RESOURCES_WORD_BUDGET);

  client.ui().click(reset);
  let restored = find_named(&client.ui(), canvas, "boundary-primary");
  assert_ne!(restored, primary);
  assert_eq!(visible_text(&client.ui(), restored), boundary_initial);
  assert_accessible_text(&client.ui(), ROOT_ID, None, None, None);
}

#[test]
fn refs_screen_samples_world_geometry_and_restores_an_unavailable_target() {
  let engine = create_engine();
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "refs-navigation");
  client.ui().click(navigation);

  let canvas = find_named(&client.ui(), ROOT_ID, "refs-canvas");
  let field = find_named(&client.ui(), canvas, "refs-field");
  let action = find_named(&client.ui(), canvas, "refs-action");
  let status = find_named(&client.ui(), canvas, "refs-status");
  let initial_observations = self::added_observations(client.commands());
  assert_eq!(initial_observations.len(), 4);
  assert!(visible_word_count(&client.ui(), canvas) <= REFS_WORD_BUDGET);
  assert_eq!(client.ui().focused(), None);
  assert_eq!(client.ui().selection(field), None);
  assert_eq!(client.ui().element(status).text(), Some("MEASURING"));

  client.submit_geometry(GeometryObservationBatch {
    generation: self::generation(1),
    changed: initial_observations
      .iter()
      .map(self::sample_geometry)
      .collect(),
  });
  assert_eq!(client.ui().element(status).text(), Some("GEOMETRY CURRENT"));
  let point = find_named(&client.ui(), canvas, "geometry-point");
  assert_eq!(client.ui().element(point).children().len(), 2);
  client.poll();
  let effect_runs = find_named(&client.ui(), canvas, "geometry-effect-runs");
  assert_eq!(
    client.ui().element(effect_runs).text(),
    Some("Effect runs · 1")
  );

  let command_start = client.commands().len();
  client.ui().click(action);
  assert_eq!(client.ui().focused(), Some(field));
  assert_eq!(client.ui().selection(field), Some((16, 0)));
  assert_eq!(client.ui().element(action).text(), Some("RESTORE TARGET"));
  assert_eq!(client.ui().element(status).text(), Some("MEASURING"));
  let unavailable = self::added_observations(&client.commands()[command_start..]);
  assert_eq!(unavailable.len(), 2);
  client.submit_geometry(GeometryObservationBatch {
    generation: self::generation(2),
    changed: unavailable
      .iter()
      .map(|observation| GeometryObservationValue {
        observation_id: observation.observation_id,
        result: GeometryObservationResult::Unavailable(GeometryUnavailable::ObjectMissing),
      })
      .collect(),
  });
  assert_eq!(
    client.ui().element(status).text(),
    Some("TARGET UNAVAILABLE")
  );

  let command_start = client.commands().len();
  client.ui().click(action);
  assert_eq!(client.ui().focused(), Some(action));
  assert_eq!(client.ui().selection(field), Some((0, 0)));
  assert_eq!(client.ui().element(action).text(), Some("SHOW UNAVAILABLE"));
  let restored = self::added_observations(&client.commands()[command_start..]);
  assert_eq!(restored.len(), 2);
  client.submit_geometry(GeometryObservationBatch {
    generation: self::generation(3),
    changed: restored.iter().map(self::sample_geometry).collect(),
  });
  assert_eq!(client.ui().element(status).text(), Some("GEOMETRY CURRENT"));
  assert_accessible_text(&client.ui(), ROOT_ID, None, None, None);
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

fn visible_word_count(ui: &UiClient<'_, ReactantEngine>, root: ObjectId) -> usize {
  let mut pending = vec![root];
  let mut words = 0;
  while let Some(object_id) = pending.pop() {
    let element = ui.element(object_id);
    words += element
      .text()
      .map_or(0, |text| text.split_whitespace().count());
    pending.extend(element.children());
  }
  words
}

fn identity_labels<E>(ui: &UiClient<'_, E>, root: ObjectId) -> Vec<String>
where
  E: Engine<Command = Command>,
{
  ui.element(root)
    .children()
    .iter()
    .map(|token| {
      ui.element(*ui.element(*token).children().first().expect("token label"))
        .text()
        .expect("token label text")
        .to_owned()
    })
    .collect()
}

fn identity_states<E>(ui: &UiClient<'_, E>, root: ObjectId) -> Vec<String>
where
  E: Engine<Command = Command>,
{
  ui.element(root)
    .children()
    .iter()
    .map(|token| {
      ui.element(*ui.element(*token).children().get(1).expect("token state"))
        .text()
        .expect("token state text")
        .to_owned()
    })
    .collect()
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
