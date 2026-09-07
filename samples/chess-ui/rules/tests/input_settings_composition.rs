use battlement::{
  AccessibilitySnapshot, ClickEvent, CommandBody, GameObjectKind, KeyEvent, KeyModifiers,
  LengthOrAuto, ObjectId, PhysicalKey, Prop, SemanticRole, StyleValue, UiAccessibilityAction,
  UiAccessibilityActionEvent, UiElement, UiEvent, UiEventBody, Vector,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn composed_input_panel_rebinds_conflicts_resets_scrolls_and_gallery_resets() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-37");
  let panel = self::named(&mut client, "settings-panel");
  assert!(matches!(
    client.ui().element(panel).style().height,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(1021.0)))
  ));
  let scroll = self::named(&mut client, "input-bindings-scroll");
  assert!(matches!(
    client.ui().element(scroll).style().height,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(971.0)))
  ));

  self::open_binding(&mut client, 4);
  self::key_down(&mut client, PhysicalKey::KeyM, "m");
  self::assert_cell(&client, "M");
  self::open_binding(&mut client, 4);
  self::key_down(&mut client, PhysicalKey::KeyR, "r");
  self::assert_text(&client, "Already used by Restart");
  self::click_semantic(&mut client, SemanticRole::Button, "Cancel");
  self::assert_cell(&client, "M");
  self::open_binding(&mut client, 4);
  self::click_semantic(&mut client, SemanticRole::Button, "Reset");
  self::assert_cell(&client, "Space");

  self::scroll_forward(&mut client, scroll);
  let ui = client.ui();
  let UiElement::ScrollView(view) = ui.element(scroll).element() else {
    panic!("expected input scroll view")
  };
  assert_eq!(view.scroll_offset, Prop::Set(Vector::new(0.0, 470.0)));
  self::open_binding(&mut client, 6);
  self::assert_text(&client, "Press a key for Restart");

  self::click_named(&mut client, "review-page-37");
  self::assert_cell(&client, "Space");
  assert!(
    !self::snapshot(&client)
      .nodes
      .iter()
      .any(|node| node.role == SemanticRole::Dialog)
  );
  let reset_scroll = self::named(&mut client, "input-bindings-scroll");
  let ui = client.ui();
  let UiElement::ScrollView(view) = ui.element(reset_scroll).element() else {
    panic!("expected reset input scroll view")
  };
  assert_eq!(view.scroll_offset, Prop::Set(Vector::new(0.0, 0.0)));
}

#[test]
fn composition_text_size_reflows_the_real_table_and_reset_returns_to_default() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-37");
  self::click_named(&mut client, "input-composition-text-size");
  let restart = self::named(&mut client, "input-binding-restart");
  assert!(matches!(
    client.ui().element(restart).style().height,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(318.0)))
  ));
  self::click_named(&mut client, "input-composition-reset");
  let restart = self::named(&mut client, "input-binding-restart");
  assert!(matches!(
    client.ui().element(restart).style().height,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(159.0)))
  ));
}

fn open_binding(client: &mut FakeClient<App>, index: usize) {
  let binding = self::named(client, &format!("keyboard-binding-{index}"));
  client
    .ui()
    .send_event(UiEvent::click(binding, ClickEvent::NavigationSubmit));
  client.poll();
}

fn key_down(client: &mut FakeClient<App>, key: PhysicalKey, text: &str) {
  let target = self::named(client, "shortcut-waiting-marker");
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::KeyDown(KeyEvent {
      physical_key: Some(key),
      text: text.to_owned(),
      modifiers: KeyModifiers::default(),
    }),
  ));
  client.poll();
}

fn scroll_forward(client: &mut FakeClient<App>, target: ObjectId) {
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 1,
      action: UiAccessibilityAction::ScrollForward,
    }),
  ));
  client.poll();
}

fn assert_cell(client: &FakeClient<App>, value: &str) {
  self::semantic(client, SemanticRole::Cell, value);
}

fn assert_text(client: &FakeClient<App>, value: &str) {
  self::semantic(client, SemanticRole::StaticText, value);
}

fn click_semantic(client: &mut FakeClient<App>, role: SemanticRole, label: &str) {
  let target = self::semantic(client, role, label);
  client.ui().click(target);
  client.poll();
}

fn semantic(client: &FakeClient<App>, role: SemanticRole, label: &str) -> ObjectId {
  self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == role && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing {role:?} {label}"))
    .object_id
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
    .expect("input composition semantics")
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().click(target);
  client.poll();
}

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  let mut pending = client
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::UiDocument(document) => Some(document.root_id()),
      _ => None,
    })
    .collect::<Vec<_>>();
  while let Some(id) = pending.pop() {
    let ui = client.ui();
    let element = ui.element(id);
    if element.name() == Some(name) {
      return id;
    }
    pending.extend(element.children());
  }
  panic!("missing {name}");
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
