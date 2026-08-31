use std::{cell::RefCell, collections::BTreeSet, num::NonZeroU64, rc::Rc, sync::Arc};

use battlement::{
  ActionId, ClientMessage, Color, Command, Connect, CoreErrorCode, Display, DisplayId,
  DisplayOrientation, ElementGeometry, FlexDirection, FocusEvent, GeometryEvent,
  GeometryGeneration, GeometryObservation, GeometryObservationBatch, GeometryObservationResult,
  GeometryObservationTarget, GeometryObservationValue, GeometryUnavailable, GeometryValue,
  KeyModifiers, Length, LengthOrAuto, ObjectId, PanelPoint, PointerButton, PointerButtonEvent,
  PointerCrossingEvent, PointerType, PreparedAsset, Projective2, Prop, Rect, Response,
  ResponseMessage, ScreenSize, StaggerDirection, StyleValue, UiElementKind, UiEvent, UiEventBody,
  UiVisualElementProperties, VariantWhen, Vector, ViewportGeometry, ViewportPoint, ViewportRect,
  WorldBoundsGeometry, WorldPointGeometry,
};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::{FakeClient, ui::UiClient},
  journal::ExecutedCommand,
};
use battlement_native::{Engine, EngineError};
use battlement_rules::{
  CONTENT_SCENE, MOTION_MATERIAL, MOTION_TEXTURE, ROOT_ID, ReactantEngine, Screen, create_engine,
  generated_asset_addresses,
};

const SCREEN_WORD_BUDGET: usize = 15;
const EVENTS_WORD_BUDGET: usize = 20;
const STATE_WORD_BUDGET: usize = 24;
const CONTEXT_WORD_BUDGET: usize = 24;
const EFFECTS_WORD_BUDGET: usize = 22;
const RESOURCES_WORD_BUDGET: usize = 18;
const REFS_WORD_BUDGET: usize = 48;

type Correlations = Rc<RefCell<Vec<(ActionId, Vec<Option<ActionId>>)>>>;

struct CorrelationEngine {
  inner: ReactantEngine,
  correlations: Correlations,
}

impl Engine for CorrelationEngine {
  type ActionPayload = ();
  type ErrorCode = CoreErrorCode;
  type Command = Command;

  fn connect(&mut self, message: Connect) -> Result<Response, EngineError> {
    self.inner.connect(message)
  }

  fn submit(
    &mut self,
    message: ClientMessage<Self::ActionPayload, Self::ErrorCode>,
  ) -> Result<Response, EngineError> {
    let ClientMessage::Action(action) = &message else {
      panic!("the fake UI submits actions");
    };
    let action_id = action.action_id;
    let response = self.inner.submit(message)?;
    let causes = response
      .messages
      .iter()
      .filter_map(|message| match message {
        ResponseMessage::Batch(batch) => Some(batch.caused_by_action_id),
        _ => None,
      })
      .collect::<Vec<_>>();
    if !causes.is_empty() {
      self.correlations.borrow_mut().push((action_id, causes));
    }
    Ok(response)
  }

  fn poll(&mut self) -> Result<Option<Response>, EngineError> {
    self.inner.poll()
  }
}

#[test]
fn sample_opens_on_an_accessible_composition_screen() {
  let engine = create_engine().expect("Reactant sample engine should initialize");
  assert_eq!(engine.screen(), Screen::Composition);
  let mut client = FakeClient::connect(engine, catalog());
  let ui = client.ui();
  let shell = find_named(&ui, ROOT_ID, "sample-shell");
  let navigation = find_named(&ui, ROOT_ID, "navigation");
  let canvas = find_named(&ui, ROOT_ID, "composition-canvas");

  assert_eq!(ui.element(ROOT_ID).children(), &[shell]);
  assert_eq!(ui.element(shell).children(), &[navigation, canvas]);
  assert_eq!(
    ui.element(find_named(&ui, navigation, "composition-navigation"))
      .text(),
    Some("01  COMPOSITION")
  );
  assert_eq!(visible_word_count(&ui, canvas), SCREEN_WORD_BUDGET);
  assert_eq!(font_size(&ui, find_named(&ui, canvas, "page-title")), 44.0);
  assert!(font_size(&ui, find_named(&ui, canvas, "specimen-heading")) >= 28.0);
  assert_accessible_text(&ui, ROOT_ID, None, None, None);
}

#[test]
fn sample_uses_top_navigation_for_narrow_connections() {
  let engine = create_engine().expect("Reactant sample engine should initialize");
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
  let engine = create_engine().expect("Reactant sample engine should initialize");
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
  let pending = find_named(&client.ui(), group, "resource-pending");
  let status = client.ui().element(pending).children()[0];
  let resolve = find_named(&client.ui(), pending, "resource-resolve");
  assert_eq!(
    client.ui().element(group).style().flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::Column))
  );
  assert_eq!(font_size(&client.ui(), status), 24.0);
  assert_eq!(
    client.ui().element(resolve).style().width,
    Prop::Set(StyleValue::Value(LengthOrAuto::Percent(100.0)))
  );
}

#[test]
fn sample_recomposes_when_the_viewport_crosses_the_compact_breakpoint() {
  let engine = create_engine().expect("Reactant sample engine should initialize");
  let mut client = FakeClient::connect(engine, catalog());
  let shell = find_named(&client.ui(), ROOT_ID, "sample-shell");

  client.ui().send_event(UiEvent {
    target_id: shell,
    body: UiEventBody::GeometryChanged(GeometryEvent {
      previous: Rect::new(0.0, 0.0, 1_280.0, 720.0),
      current: Rect::new(0.0, 0.0, 500.0, 700.0),
    }),
  });
  assert_eq!(
    client.ui().element(shell).style().flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::Column))
  );
  let navigation = find_named(&client.ui(), shell, "navigation");
  let current = find_named(&client.ui(), navigation, "phone-current-screen");
  assert_eq!(client.ui().element(current).text(), Some("01 COMPOSITION"));

  client.ui().send_event(UiEvent {
    target_id: shell,
    body: UiEventBody::GeometryChanged(GeometryEvent {
      previous: Rect::new(0.0, 0.0, 500.0, 700.0),
      current: Rect::new(0.0, 0.0, 1_280.0, 720.0),
    }),
  });
  assert_eq!(
    client.ui().element(shell).style().flex_direction,
    Prop::Set(StyleValue::Value(FlexDirection::Row))
  );
  let navigation = find_named(&client.ui(), shell, "navigation");
  let items = find_named(&client.ui(), navigation, "navigation-items");
  assert_eq!(client.ui().element(items).children().len(), 8);
}

#[test]
fn assets_screen_prepares_later_paint_and_resizes_then_restores_the_nine_slice() {
  let engine = create_engine().expect("Reactant sample engine should initialize");
  let mut client = FakeClient::connect(engine, catalog());
  let addresses = generated_asset_addresses();
  assert_eq!(addresses.len(), 9);
  assert_eq!(addresses.iter().cloned().collect::<BTreeSet<_>>().len(), 9);
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
  assert!(!initial.iter().any(|text| text == "SKEW / LATER STATE"));
  assert_eq!(
    client.ui().element(action).text(),
    Some("RESIZE NINE-SLICE")
  );
  assert_eq!(
    style_length_or_auto(&client.ui().element(action).style().width),
    Some(300.0)
  );
  assert!(matches!(
    client.ui().element(action).style().unity_slice_top,
    Prop::Set(StyleValue::Value(value)) if value == 42
  ));

  client.ui().click(action);
  let canvas = find_named(&client.ui(), ROOT_ID, "assets-canvas");
  let action = find_named(&client.ui(), canvas, "assets-resize-action");
  assert_eq!(
    client.ui().element(action).text(),
    Some("RESTORE NINE-SLICE")
  );
  assert_eq!(
    style_length_or_auto(&client.ui().element(action).style().width),
    Some(520.0)
  );
  assert!(
    visible_text(&client.ui(), canvas)
      .iter()
      .any(|text| text == "SKEW / LATER STATE")
  );

  client.ui().click(action);
  let canvas = find_named(&client.ui(), ROOT_ID, "assets-canvas");
  assert_eq!(visible_text(&client.ui(), canvas), initial);
}

#[test]
fn variants_screen_propagates_ordered_snapshotted_targets_and_reverses_cleanly() {
  let engine = create_engine().expect("Reactant sample engine should initialize");
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
    inner: create_engine().expect("Reactant sample engine should initialize"),
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
fn events_screen_runs_and_restores_one_logical_event_path() {
  let engine = create_engine().expect("Reactant sample engine should initialize");
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "events-navigation");
  client.ui().click(navigation);

  let canvas = find_named(&client.ui(), ROOT_ID, "events-canvas");
  let action = find_named(&client.ui(), canvas, "events-action");
  let source = find_named(&client.ui(), canvas, "event-source");
  let overlay = find_named(&client.ui(), canvas, "portal-overlay");
  let status = find_named(&client.ui(), canvas, "events-status");
  let initial = self::visible_text(&client.ui(), canvas);
  assert!(visible_word_count(&client.ui(), canvas) <= EVENTS_WORD_BUDGET);
  assert_eq!(client.ui().element(action).text(), Some("RUN EVENT"));
  assert_eq!(client.ui().element(status).text(), Some("READY"));
  assert!(!client.ui().element(source).children().contains(&action));
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
  let engine = create_engine().expect("Reactant sample engine should initialize");
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
  let engine = create_engine().expect("Reactant sample engine should initialize");
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
  let engine = create_engine().expect("Reactant sample engine should initialize");
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
  let engine = create_engine().expect("Reactant sample engine should initialize");
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
  let engine = create_engine().expect("Reactant sample engine should initialize");
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
  let engine = create_engine().expect("Reactant sample engine should initialize");
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

#[test]
fn buttons_render_distinct_hover_pressed_and_focus_states() {
  let engine = create_engine().expect("Reactant sample engine should initialize");
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "composition-navigation");
  let selected = style_color(&client.ui().element(navigation).style().background_color)
    .expect("selected navigation background should be authored");
  client.ui().send_event(UiEvent {
    target_id: navigation,
    body: UiEventBody::FocusIn(FocusEvent::default()),
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
    body: UiEventBody::FocusOut(FocusEvent::default()),
  });
  let action = find_named(&client.ui(), ROOT_ID, "composition-action");
  let resting = style_color(&client.ui().element(action).style().background_color)
    .expect("resting action background should be authored");
  let resting_border = client.ui().element(action).style().border_top_width;

  client.ui().send_event(UiEvent {
    target_id: action,
    body: UiEventBody::PointerOver(PointerCrossingEvent {
      related_target_id: None,
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
    body: UiEventBody::PointerDown(self::pointer_button_event()),
  });
  let pressed = style_color(&client.ui().element(action).style().background_color)
    .expect("pressed action background should be authored");
  assert_ne!(pressed, hovered);

  client.ui().send_event(UiEvent {
    target_id: action,
    body: UiEventBody::PointerUp(self::pointer_button_event()),
  });
  client.ui().send_event(UiEvent {
    target_id: action,
    body: UiEventBody::FocusIn(FocusEvent::default()),
  });
  assert_ne!(
    client.ui().element(action).style().border_top_width,
    resting_border
  );

  client.ui().send_event(UiEvent {
    target_id: action,
    body: UiEventBody::FocusOut(FocusEvent::default()),
  });
  client.ui().send_event(UiEvent {
    target_id: action,
    body: UiEventBody::PointerOut(PointerCrossingEvent {
      related_target_id: None,
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

fn catalog() -> Arc<FakeAssetCatalog> {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene(CONTENT_SCENE);
  catalog.add_textures(generated_asset_addresses());
  catalog.add_material(MOTION_MATERIAL);
  catalog.add_texture(MOTION_TEXTURE);
  Arc::new(catalog)
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

fn find_named<E>(ui: &UiClient<'_, E>, root: ObjectId, expected: &str) -> ObjectId
where
  E: Engine<Command = Command>,
{
  let mut pending = vec![root];
  while let Some(object_id) = pending.pop() {
    let element = ui.element(object_id);
    if element.name() == Some(expected) {
      return object_id;
    }
    pending.extend(element.children());
  }
  panic!("missing UI element named {expected}");
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
    battlement::MotionValue::Length(value) if value.percent == 0.0 => value.px,
    value => panic!("motion property is not scalar-like: {value:?}"),
  }
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
    assert!(size.expect("visible text must have a resolved size") >= 24.0);
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

fn font_size(ui: &UiClient<'_, ReactantEngine>, object_id: ObjectId) -> f32 {
  style_length(&ui.element(object_id).style().font_size).expect("font size should be authored")
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
