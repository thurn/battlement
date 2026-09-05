use battlement::{
  AccessibilitySnapshot, CommandBody, GameObjectKind, Length, ObjectId, Prop, SemanticRole,
  StyleValue, UiElement,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn input_glyphs_custom_scales_and_panel_padding_reset_as_one_specimen() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-24");
  client.poll();

  for name in [
    "keyboard-arrow-left",
    "keyboard-arrow-right",
    "keyboard-arrow-up",
    "keyboard-arrow-down",
    "d-pad-left",
    "d-pad-right",
    "d-pad-up",
    "d-pad-down",
    "controller-button-a",
    "controller-button-menu",
    "controller-button-y",
  ] {
    self::named(&mut client, name);
  }
  self::assert_width(&mut client, "keyboard-binding-0", 205.0);

  self::click_named(&mut client, "input-skin-long-custom");
  client.poll();
  for label in ["A", "D", "W", "S", "Backspace", "Tab", "Enter"] {
    assert!(
      self::snapshot(&client)
        .nodes
        .iter()
        .any(|node| { node.role == SemanticRole::Cell && node.label.as_deref() == Some(label) })
    );
  }
  assert!(self::named_optional(&mut client, "keyboard-arrow-left").is_none());

  self::click_named(&mut client, "input-skin-scale-150");
  client.poll();
  self::assert_width(&mut client, "keyboard-binding-4", 271.625);
  self::click_named(&mut client, "input-skin-scale-200");
  client.poll();
  self::assert_width(&mut client, "keyboard-binding-4", 338.25);

  self::click_named(&mut client, "input-skin-panel-surround");
  client.poll();
  let background = self::named(&mut client, "settings-panel-background");
  assert!(matches!(
    client.ui().element(background).element(),
    UiElement::Image(_)
  ));
  let content = self::named(&mut client, "settings-panel-content");
  let ui = client.ui();
  let style = ui.element(content).style();
  self::assert_length(&style.padding_top, 18.0);
  self::assert_length(&style.padding_right, 24.0);
  self::assert_length(&style.padding_bottom, 32.0);
  self::assert_length(&style.padding_left, 24.0);

  self::click_named(&mut client, "input-skin-reset");
  client.poll();
  self::named(&mut client, "keyboard-arrow-left");
  self::assert_width(&mut client, "keyboard-binding-0", 205.0);
}

fn assert_length(value: &Prop<StyleValue<Length>>, expected: f32) {
  let Prop::Set(StyleValue::Value(Length::Px(actual))) = value else {
    panic!("expected pixel length")
  };
  assert!((actual - expected).abs() < 0.01);
}

fn assert_width(client: &mut FakeClient<App>, name: &str, expected: f32) {
  let element = self::named(client, name);
  let ui = client.ui();
  let Prop::Set(StyleValue::Value(battlement::LengthOrAuto::Px(actual))) =
    ui.element(element).style().width
  else {
    panic!("{name} has no pixel width")
  };
  assert!((actual - expected).abs() < 0.01);
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().click(target);
}

fn snapshot(client: &FakeClient<App>) -> &AccessibilitySnapshot {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .expect("input skin semantics")
}

fn client() -> FakeClient<App> {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("chess-ui/content");
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(setting_row::DISPLAY_FONT);
  assets.add_ui_font(select_control::VALUE_FONT);
  assets.add_ui_font(action_button::ACTION_FONT);
  let mut client = FakeClient::connect(engine::create_engine(), assets);
  client.poll();
  client
}

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  self::named_optional(client, name).unwrap_or_else(|| panic!("missing {name}"))
}

fn named_optional(client: &mut FakeClient<App>, name: &str) -> Option<ObjectId> {
  let mut pending = client
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::UiDocument(document) => Some(document.root_id()),
      _ => None,
    })
    .collect::<Vec<_>>();
  let ui = client.ui();
  while let Some(id) = pending.pop() {
    let element = ui.element(id);
    if element.name() == Some(name) {
      return Some(id);
    }
    pending.extend(element.children());
  }
  None
}
