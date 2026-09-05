use battlement::{
  AccessibilitySnapshot, CommandBody, GameObjectKind, Length, ObjectId, Prop, SemanticRole,
  StyleValue, UiAccessibilityAction, UiAccessibilityActionEvent, UiElement, UiEvent, UiEventBody,
  Vector,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn text_sizes_reflow_controls_preserve_tabs_and_reveal_the_input_tail() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-22");
  client.poll();

  self::assert_width(&mut client, "select-control", 396.0);
  self::assert_height(&mut client, "setting-row", 159.0);

  self::click_named(&mut client, "font-scale-200");
  client.poll();
  self::assert_width(&mut client, "select-control", 696.0);
  self::assert_height(&mut client, "setting-row", 318.0);
  self::assert_width(&mut client, "toggle-control-box", 127.05);

  self::click_named(&mut client, "select-trigger");
  client.poll();
  assert!(self::snapshot(&client).nodes.iter().any(|node| {
    node.role == SemanticRole::ListBox && node.label.as_deref() == Some("Display Mode options")
  }));
  self::assert_height(&mut client, "select-option-fullscreen", 125.4);

  self::click_named(&mut client, "font-scale-navigation");
  client.poll();
  let gameplay = self::semantic(&client, SemanticRole::Tab, "Gameplay");
  assert_eq!(
    client.ui().element(gameplay).style().font_size,
    Prop::Set(StyleValue::Value(Length::Px(63.25)))
  );
  assert!(self::semantic_optional(&client, SemanticRole::Button, "RETURN").is_some());

  self::click_named(&mut client, "font-scale-headings");
  client.poll();
  self::assert_width(&mut client, "screen-header-artwork", 1024.8);
  assert!(self::semantic_optional(&client, SemanticRole::Heading, "Settings").is_some());
  assert!(
    self::semantic_optional(&client, SemanticRole::Heading, "Chess Chess Revolution").is_some()
  );

  self::click_named(&mut client, "font-scale-dialog");
  client.poll();
  self::click_named(&mut client, "font-scale-open-dialog");
  client.poll();
  assert!(self::semantic_optional(&client, SemanticRole::Dialog, "Pause").is_some());
  let cancel = self::semantic(&client, SemanticRole::Button, "Cancel");
  client.ui().click(cancel);
  client.poll();

  self::click_named(&mut client, "font-scale-input-table");
  client.poll();
  let scroll = self::named(&mut client, "input-bindings-scroll");
  self::accessibility_scroll(&mut client, scroll);
  let ui = client.ui();
  let UiElement::ScrollView(scroll) = ui.element(scroll).element() else {
    panic!("expected input bindings scroll")
  };
  assert_eq!(scroll.scroll_offset, Prop::Set(Vector::new(0.0, 1671.0)));

  self::click_named(&mut client, "review-page-22");
  client.poll();
  self::assert_width(&mut client, "select-control", 396.0);
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().click(target);
}

fn accessibility_scroll(client: &mut FakeClient<App>, target: ObjectId) {
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

fn assert_width(client: &mut FakeClient<App>, name: &str, expected: f32) {
  let element = self::named(client, name);
  let ui = client.ui();
  let Prop::Set(StyleValue::Value(battlement::LengthOrAuto::Px(actual))) =
    ui.element(element).style().width
  else {
    panic!("{name} has no pixel width")
  };
  assert!(
    (actual - expected).abs() < 0.01,
    "{name}: {actual} != {expected}"
  );
}

fn assert_height(client: &mut FakeClient<App>, name: &str, expected: f32) {
  let element = self::named(client, name);
  let ui = client.ui();
  let style = ui.element(element).style();
  let actual = match (&style.height, &style.min_height) {
    (Prop::Set(StyleValue::Value(battlement::LengthOrAuto::Px(value))), _) => *value,
    (_, Prop::Set(StyleValue::Value(battlement::LengthOrAuto::Px(value)))) => *value,
    _ => panic!("{name} has no pixel height"),
  };
  assert!(
    (actual - expected).abs() < 0.01,
    "{name}: {actual} != {expected}"
  );
}

fn semantic(client: &FakeClient<App>, role: SemanticRole, label: &str) -> ObjectId {
  self::semantic_optional(client, role, label).unwrap_or_else(|| panic!("missing {role:?} {label}"))
}

fn semantic_optional(
  client: &FakeClient<App>,
  role: SemanticRole,
  label: &str,
) -> Option<ObjectId> {
  self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == role && node.label.as_deref() == Some(label))
    .map(|node| node.object_id)
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
    .expect("font-scale semantics")
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
      return id;
    }
    pending.extend(element.children());
  }
  panic!("missing {name}");
}
