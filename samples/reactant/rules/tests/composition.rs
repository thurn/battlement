use std::{cell::RefCell, rc::Rc, sync::Arc};

use battlement::{
  ActionId, ClientMessage, Color, Command, Connect, CoreErrorCode, FocusEvent, KeyModifiers,
  Length, ObjectId, PanelPoint, PointerButton, PointerButtonEvent, PointerCrossingEvent,
  PointerType, Prop, Response, ResponseMessage, StyleValue, UiElementKind, UiEvent, UiEventBody,
  Vector,
};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::{FakeClient, ui::UiClient},
};
use battlement_native::{Engine, EngineError};
use battlement_rules::{CONTENT_SCENE, ROOT_ID, ReactantEngine, Screen, create_engine};

const SCREEN_WORD_BUDGET: usize = 15;
const EVENTS_WORD_BUDGET: usize = 16;

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

  client.ui().click(action);
  assert_eq!(client.ui().element(action).text(), Some("REORDER"));
  assert_eq!(self::child_text(&client.ui(), badges), initial);
  for (action_id, causes) in correlations.borrow().iter() {
    assert_eq!(causes, &[Some(*action_id)]);
  }
  assert_eq!(correlations.borrow().len(), 2);
}

#[test]
fn events_screen_runs_and_restores_one_logical_event_path() {
  let engine = create_engine().expect("Reactant sample engine should initialize");
  let mut client = FakeClient::connect(engine, catalog());
  let navigation = find_named(&client.ui(), ROOT_ID, "events-navigation");
  client.ui().click(navigation);

  let canvas = find_named(&client.ui(), ROOT_ID, "events-canvas");
  let action = find_named(&client.ui(), canvas, "events-action");
  let status = find_named(&client.ui(), canvas, "events-status");
  let initial = self::visible_text(&client.ui(), canvas);
  assert!(visible_word_count(&client.ui(), canvas) <= EVENTS_WORD_BUDGET);
  assert_eq!(client.ui().element(action).text(), Some("RUN EVENT"));
  assert_eq!(client.ui().element(status).text(), Some("READY"));

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
fn buttons_render_distinct_hover_pressed_and_focus_states() {
  let engine = create_engine().expect("Reactant sample engine should initialize");
  let mut client = FakeClient::connect(engine, catalog());
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
  Arc::new(catalog)
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
