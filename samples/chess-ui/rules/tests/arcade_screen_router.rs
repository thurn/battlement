use battlement::{
  AccessibilitySnapshot, CheckedState, CommandBody, GameObjectKind, KeyEvent, KeyModifiers,
  MotionDescriptor, MotionEventBatch, MotionEventKind, MotionLayer, MotionLifecycleEvent,
  MotionSequence, NavigationEvent, ObjectId, PhysicalKey, Prop, SemanticRole, StyleValue, UiEvent,
  UiEventBody, UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{
  action_button, background_music::BACKGROUND_MUSIC, engine, select_control, setting_row,
};

#[test]
fn full_screen_router_preserves_settings_then_closes_and_resets() {
  self::with_render_stack(self::full_screen_router_scenario);
}

fn full_screen_router_scenario() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-40");
  let launcher = self::named(&mut client, "arcade-app-launcher");
  self::click_named(&mut client, "arcade-app-launcher");

  self::semantic(&client, SemanticRole::Heading, "Chess Chess Revolution");
  let settings_heading = self::named(&mut client, "screen-header-heading");
  assert_eq!(
    client.ui().focused(),
    Some(settings_heading),
    "settings heading should receive route focus"
  );
  self::assert_gallery_inert(&mut client, true);

  self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
  client.poll();
  self::semantic(&client, SemanticRole::Heading, "Settings");
  assert!(
    client
      .ui()
      .focused()
      .is_some_and(|focused| focused != launcher),
    "settings route should keep focus inside the application"
  );
  self::choose(&mut client, "Text Size 100%", "200%");
  self::toggle(&mut client, "Reduce Motion");
  self::click_semantic(&mut client, SemanticRole::Button, "RETURN");
  self::semantic(&client, SemanticRole::Heading, "Chess Chess Revolution");
  self::assert_px(&mut client, "main-menu-actions", "top", 540.0);

  self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
  self::semantic(&client, SemanticRole::Button, "Text Size 200%");
  self::assert_checkbox(&client, "Reduce Motion", true);
  let return_button = self::semantic(&client, SemanticRole::Button, "RETURN");
  self::escape(&mut client, return_button);
  self::semantic(&client, SemanticRole::Heading, "Chess Chess Revolution");

  let play = self::semantic(&client, SemanticRole::Button, "PLAY");
  self::escape(&mut client, play);
  self::assert_gallery_inert(&mut client, false);
  assert_eq!(client.ui().focused(), Some(launcher));

  self::click_named(&mut client, "arcade-app-launcher");
  self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
  self::semantic(&client, SemanticRole::Button, "Text Size 100%");
  self::assert_checkbox(&client, "Reduce Motion", false);
}

#[test]
fn controller_cancel_obeys_route_precedence_and_then_restores_launcher_focus() {
  self::with_render_stack(self::controller_cancel_scenario);
}

fn controller_cancel_scenario() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-40");
  let launcher = self::named(&mut client, "arcade-app-launcher");
  self::click_named(&mut client, "arcade-app-launcher");
  self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");

  let return_button = self::semantic(&client, SemanticRole::Button, "RETURN");
  self::cancel(&mut client, return_button);
  let play = self::semantic(&client, SemanticRole::Button, "PLAY");
  self::cancel(&mut client, play);

  self::semantic(&client, SemanticRole::Button, "Launch Chess UI");
  self::assert_gallery_inert(&mut client, false);
  assert_eq!(client.ui().focused(), Some(launcher));
}

#[test]
fn play_and_quit_reach_terminal_black_before_the_layer_can_close() {
  self::with_render_stack(|| {
    for (sequence, action) in ["PLAY", "QUIT"].into_iter().enumerate() {
      let mut client = self::client();
      self::click_named(&mut client, "review-page-40");
      self::click_named(&mut client, "arcade-app-launcher");
      self::click_semantic(&mut client, SemanticRole::Button, action);
      self::complete_exit(&mut client, (sequence + 1) as u64);
      let black = self::semantic(&client, SemanticRole::Region, "Dismissed arcade stage");
      self::escape(&mut client, black);
      self::semantic(&client, SemanticRole::Button, "Launch Chess UI");
    }
  });
}

fn with_render_stack(scenario: fn()) {
  std::thread::Builder::new()
    .name("complete-chess-ui-render".to_owned())
    .stack_size(16 * 1024 * 1024)
    .spawn(scenario)
    .expect("complete UI test thread")
    .join()
    .expect("complete UI scenario");
}

fn complete_exit(client: &mut FakeClient<App>, sequence: u64) {
  let descriptor = self::descriptor_named(client, "arcade-exit-content-surface");
  let slot = descriptor
    .slots
    .iter()
    .find(|slot| slot.layer == MotionLayer::Animate)
    .expect("exit animation slot");
  let sequence = MotionSequence(sequence);
  client.submit_motion(MotionEventBatch {
    first_sequence: sequence,
    last_sequence: sequence,
    events: vec![MotionLifecycleEvent {
      sequence,
      descriptor_id: descriptor.descriptor_id,
      slot: slot.slot,
      generation: slot.generation,
      elapsed_micros: 620_000,
      kind: MotionEventKind::Completed,
    }],
    samples: Vec::new(),
    value_samples: Vec::new(),
    playback_events: Vec::new(),
    gesture_events: Vec::new(),
  });
  client.poll();
  client.poll();
}

fn descriptor_named(client: &mut FakeClient<App>, name: &str) -> MotionDescriptor {
  let id = self::named(client, name);
  let Prop::Set(descriptor) = client
    .ui()
    .element(id)
    .element()
    .visual_element()
    .motion
    .clone()
  else {
    panic!("missing motion descriptor")
  };
  descriptor
}

fn choose(client: &mut FakeClient<App>, trigger: &str, option: &str) {
  self::click_semantic(client, SemanticRole::Button, trigger);
  self::click_semantic(client, SemanticRole::Option, option);
}

fn toggle(client: &mut FakeClient<App>, label: &str) {
  let target = self::semantic(client, SemanticRole::Checkbox, label);
  client.ui().toggle_click(target);
  client.poll();
}

fn escape(client: &mut FakeClient<App>, target: ObjectId) {
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::KeyDown(KeyEvent {
      physical_key: Some(PhysicalKey::Escape),
      text: String::new(),
      modifiers: KeyModifiers::default(),
    }),
  ));
  client.poll();
}

fn cancel(client: &mut FakeClient<App>, target: ObjectId) {
  client.ui().send_event(UiEvent::new(
    target,
    true,
    false,
    UiEventBody::NavigationCancel(NavigationEvent::default()),
  ));
  client.poll();
}

fn assert_checkbox(client: &FakeClient<App>, label: &str, expected: bool) {
  let node = self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Checkbox && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing checkbox {label}"));
  assert_eq!(
    node.state.checked,
    Some(if expected {
      CheckedState::True
    } else {
      CheckedState::False
    })
  );
}

fn assert_gallery_inert(client: &mut FakeClient<App>, expected: bool) {
  let gallery = self::named(client, "gallery");
  assert_eq!(
    client
      .ui()
      .element(gallery)
      .element()
      .visual_element()
      .inert,
    Prop::Set(expected)
  );
}

fn assert_px(client: &mut FakeClient<App>, name: &str, property: &str, expected: f32) {
  let id = self::named(client, name);
  let ui = client.ui();
  let style = ui.element(id).style();
  let value = match property {
    "top" => &style.top,
    _ => panic!("unknown property"),
  };
  assert!(matches!(
    value,
    Prop::Set(StyleValue::Value(battlement::LengthOrAuto::Px(actual)))
      if (*actual - expected).abs() < 0.01
  ));
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
    .expect("arcade screen semantics")
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
  assets.add_audio_clip(BACKGROUND_MUSIC);
  assets.add_textures(asset_generator::registrations().map(|asset| asset.address));
  assets.add_ui_font(setting_row::DISPLAY_FONT);
  assets.add_ui_font(select_control::VALUE_FONT);
  assets.add_ui_font(action_button::ACTION_FONT);
  let mut client = FakeClient::connect(engine::create_engine(), assets);
  client.poll();
  client
}
