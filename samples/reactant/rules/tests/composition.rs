use std::sync::Arc;

use battlement::{Color, Length, ObjectId, Prop, StyleValue, UiElementKind};
use battlement_fake::{
  assets::FakeAssetCatalog,
  client::{FakeClient, ui::UiClient},
};
use battlement_rules::{CONTENT_SCENE, ROOT_ID, ReactantEngine, Screen, create_engine};

const SCREEN_WORD_BUDGET: usize = 11;

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

fn catalog() -> Arc<FakeAssetCatalog> {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene(CONTENT_SCENE);
  Arc::new(catalog)
}

fn find_named(ui: &UiClient<'_, ReactantEngine>, root: ObjectId, expected: &str) -> ObjectId {
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
