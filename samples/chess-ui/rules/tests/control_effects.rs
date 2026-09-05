use battlement::{
  GameObjectKind, KeyEvent, KeyModifiers, ObjectId, PanelPoint, PhysicalKey, PointerButton,
  PointerButtonEvent, PointerCancelEvent, PointerType, Prop, UiEvent, UiEventBody,
  UiVisualElementProperties, Vector,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn controls_restart_keyed_bursts_cancel_cleanly_and_reset_without_residue() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-25");
  client.poll();

  let action = self::named(&mut client, "action-button");
  assert_eq!(
    self::decoration_keys(&mut client, action),
    Vec::<u64>::new()
  );
  self::click_named(&mut client, "effect-shine");
  client.poll();
  assert_eq!(self::decorations(&mut client, action), 1);
  self::click_named(&mut client, "effect-shine");
  client.poll();

  client.ui().click(action);
  client.poll();
  let first = self::decoration_keys(&mut client, action);
  assert_eq!(first.len(), 12);
  client.ui().click(action);
  client.poll();
  let second = self::decoration_keys(&mut client, action);
  assert_eq!(second.len(), 12);
  assert_ne!(first, second);

  self::click_named(&mut client, "effect-reset");
  client.poll();
  let action = self::named(&mut client, "action-button");
  self::pointer_down(&mut client, action);
  client.poll();
  self::pointer_cancel(&mut client, action);
  client.poll();
  assert_eq!(self::decorations(&mut client, action), 0);

  self::click_named(&mut client, "effect-family-tabs");
  client.poll();
  let graphics = self::named_by_text(&mut client, "Graphics");
  client.ui().click(graphics);
  client.poll();
  assert_eq!(self::decorations(&mut client, graphics), 12);

  self::click_named(&mut client, "effect-family-select");
  client.poll();
  let trigger = self::named(&mut client, "select-trigger");
  client.ui().click(trigger);
  client.poll();
  assert_eq!(self::decorations(&mut client, trigger), 12);

  self::click_named(&mut client, "effect-family-checkbox");
  client.poll();
  let checkbox = self::named(&mut client, "toggle-control-input");
  client.ui().toggle_click(checkbox);
  client.poll();
  let checkbox_surface = self::named(&mut client, "toggle-control-surface");
  assert_eq!(self::decorations(&mut client, checkbox_surface), 9);

  self::click_named(&mut client, "effect-family-slider");
  client.poll();
  let slider = self::named(&mut client, "volume-input");
  self::key_down(&mut client, slider, PhysicalKey::ArrowRight);
  client.poll();
  let slider_effect = self::named(&mut client, "volume-release-effect");
  assert_eq!(self::decorations(&mut client, slider_effect), 9);

  self::click_named(&mut client, "effect-reset");
  client.poll();
  let action = self::named(&mut client, "action-button");
  assert_eq!(self::decorations(&mut client, action), 0);
}

fn decorations(client: &mut FakeClient<App>, target: ObjectId) -> usize {
  self::decoration_keys(client, target).len()
}

fn decoration_keys(client: &mut FakeClient<App>, target: ObjectId) -> Vec<u64> {
  let ui = client.ui();
  let Prop::Set(motion) = &ui.element(target).element().visual_element().motion else {
    return Vec::new();
  };
  motion
    .decorations
    .iter()
    .map(|decoration| decoration.key)
    .collect()
}

fn pointer_down(client: &mut FakeClient<App>, target: ObjectId) {
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::PointerDown(PointerButtonEvent {
      pointer_id: 0,
      position: PanelPoint::default(),
      delta: Vector::default(),
      button: PointerButton::Left,
      buttons: 1,
      pressure: 1.0,
      click_count: 1,
      modifiers: KeyModifiers::default(),
      pointer_type: PointerType::Mouse,
    }),
  ));
}

fn pointer_cancel(client: &mut FakeClient<App>, target: ObjectId) {
  client.ui().send_event(UiEvent::new(
    target,
    false,
    false,
    UiEventBody::PointerCancel(PointerCancelEvent {
      pointer_id: 0,
      position: PanelPoint::default(),
      delta: Vector::default(),
      buttons: 0,
      pressure: 0.0,
      modifiers: KeyModifiers::default(),
      pointer_type: PointerType::Mouse,
    }),
  ));
}

fn key_down(client: &mut FakeClient<App>, target: ObjectId, key: PhysicalKey) {
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::KeyDown(KeyEvent {
      physical_key: Some(key),
      text: String::new(),
      modifiers: KeyModifiers::default(),
    }),
  ));
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().click(target);
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
  panic!("missing {name}")
}

fn named_by_text(client: &mut FakeClient<App>, text: &str) -> ObjectId {
  let snapshot = client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      battlement::CommandBody::AccessibilityUpdate(update) => update.snapshot.as_ref(),
      _ => None,
    })
    .expect("control effects semantics");
  snapshot
    .nodes
    .iter()
    .find(|node| node.label.as_deref() == Some(text))
    .expect("missing semantic control")
    .object_id
}
