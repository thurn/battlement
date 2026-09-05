use battlement::{
  AccessibilitySnapshot, ClickEvent, CommandBody, GameObjectKind, KeyEvent, KeyModifiers, ObjectId,
  PhysicalKey, SemanticRole, UiEvent, UiEventBody,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn keyboard_capture_assigns_rejects_cancels_resets_and_accepts_escape() {
  let mut client = self::client();
  let page = self::named(&mut client, "review-page-21");
  client.ui().click(page);
  client.poll();

  self::open_binding(&mut client, 4);
  self::assert_capture(&client, "Move Piece", None);
  self::key_down(&mut client, PhysicalKey::ShiftLeft, "");
  self::assert_capture(&client, "Move Piece", None);
  self::key_down(&mut client, PhysicalKey::KeyM, "m");
  self::assert_cell(&client, "M");
  assert!(self::dialog(&client).is_none());

  self::open_binding(&mut client, 4);
  self::key_down(&mut client, PhysicalKey::KeyR, "r");
  self::assert_capture(&client, "Move Piece", Some("Already used by Restart"));
  assert_eq!(self::last_announcement(&client), "Already used by Restart");
  let cancel = self::semantic(&client, SemanticRole::Button, "Cancel");
  client.ui().click(cancel);
  client.poll();
  self::assert_cell(&client, "M");

  self::open_binding(&mut client, 4);
  let reset = self::semantic(&client, SemanticRole::Button, "Reset");
  client.ui().click(reset);
  client.poll();
  self::assert_cell(&client, "Space");

  self::open_binding(&mut client, 5);
  self::key_down(&mut client, PhysicalKey::KeyP, "p");
  self::assert_cell(&client, "P");
  self::open_binding(&mut client, 4);
  self::key_down(&mut client, PhysicalKey::Escape, "");
  self::assert_cell(&client, "Esc");
  assert!(self::dialog(&client).is_none());

  let page = self::named(&mut client, "review-page-21");
  client.ui().click(page);
  client.poll();
  self::assert_cell(&client, "Space");
  assert!(self::dialog(&client).is_none());
}

fn open_binding(client: &mut FakeClient<App>, index: usize) {
  let binding = self::named(client, &format!("keyboard-binding-{index}"));
  client
    .ui()
    .send_event(UiEvent::click(binding, ClickEvent::NavigationSubmit));
  client.poll();
}

fn key_down(client: &mut FakeClient<App>, key: PhysicalKey, text: &str) {
  let panel = self::named(client, "shortcut-waiting-marker");
  client.ui().send_event(UiEvent::new(
    panel,
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

fn assert_capture(client: &FakeClient<App>, action: &str, status: Option<&str>) {
  let snapshot = self::snapshot(client);
  assert!(snapshot.nodes.iter().any(|node| {
    node.role == SemanticRole::Dialog && node.label.as_deref() == Some("Change Shortcut")
  }));
  assert!(snapshot.nodes.iter().any(|node| {
    node.role == SemanticRole::StaticText
      && node.label.as_deref() == Some(&format!("Press a key for {action}"))
  }));
  if let Some(status) = status {
    assert!(snapshot.nodes.iter().any(|node| {
      node.role == SemanticRole::StaticText && node.label.as_deref() == Some(status)
    }));
  }
}

fn assert_cell(client: &FakeClient<App>, value: &str) {
  assert!(
    self::snapshot(client)
      .nodes
      .iter()
      .any(|node| { node.role == SemanticRole::Cell && node.label.as_deref() == Some(value) })
  );
}

fn dialog(client: &FakeClient<App>) -> Option<ObjectId> {
  self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Dialog)
    .map(|node| node.object_id)
}

fn last_announcement(client: &FakeClient<App>) -> String {
  client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AccessibilityUpdate(update) => update.announcements.last().cloned(),
      _ => None,
    })
    .expect("keyboard capture announcement")
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
    .expect("keyboard capture semantics")
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
