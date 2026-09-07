use battlement::{
  AccessibilitySnapshot, CommandBody, GameObjectKind, LengthOrAuto, MotionDescriptor,
  MotionEventBatch, MotionEventKind, MotionLayer, MotionLifecycleEvent, MotionSequence, ObjectId,
  Prop, SemanticRole, StyleValue, UiVisualElementProperties,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{
  action_button, background_music::BACKGROUND_MUSIC, engine, select_control, setting_row,
};

#[test]
fn complete_menu_places_actions_starts_music_and_emits_only_settings_navigation() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-39");
  self::semantic(
    &client,
    SemanticRole::Region,
    "Chess Chess Revolution main menu",
  );
  self::semantic(&client, SemanticRole::Heading, "Chess Chess Revolution");
  for label in ["PLAY", "SETTINGS", "ABOUT", "QUIT"] {
    self::semantic(&client, SemanticRole::Button, label);
  }
  self::named(&mut client, "arcade-attract-mode");
  self::named(&mut client, "arcade-frame-pulse");
  self::assert_px(&mut client, "main-menu-actions", "top", 476.0);
  self::assert_px(&mut client, "main-menu-actions", "width", 760.0);
  self::assert_px(&mut client, "main-menu-action-play", "height", 140.0);
  assert!(client.commands().iter().any(|entry| matches!(
    &entry.command.body,
    CommandBody::AudioPlay(play) if play.address == BACKGROUND_MUSIC && play.r#loop
  )));

  self::click_semantic(&mut client, SemanticRole::Button, "ABOUT");
  self::assert_text(&client, "Settings requests: 0");
  self::semantic(&client, SemanticRole::Button, "PLAY");
  self::click_semantic(&mut client, SemanticRole::Button, "SETTINGS");
  self::assert_text(&client, "Settings requests: 1");

  self::click_semantic(&mut client, SemanticRole::Button, "Mute background music");
  self::semantic(&client, SemanticRole::Button, "Enable background music");
  self::click_semantic(&mut client, SemanticRole::Button, "Enable background music");
  self::semantic(&client, SemanticRole::Button, "Mute background music");
}

#[test]
fn play_and_quit_share_terminal_black_exit_and_reset_restores_playback() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-39");
  self::click_semantic(&mut client, SemanticRole::Button, "PLAY");
  self::complete_exit(&mut client, 1);
  self::semantic(&client, SemanticRole::Region, "Dismissed arcade stage");
  assert!(
    !self::snapshot(&client)
      .nodes
      .iter()
      .any(|node| node.role == SemanticRole::Button && node.label.as_deref() == Some("PLAY"))
  );

  self::click_named(&mut client, "main-menu-reset");
  self::semantic(&client, SemanticRole::Button, "PLAY");
  self::assert_text(&client, "Settings requests: 0");
  self::click_semantic(&mut client, SemanticRole::Button, "QUIT");
  self::complete_exit(&mut client, 2);
  self::semantic(&client, SemanticRole::Region, "Dismissed arcade stage");

  self::click_named(&mut client, "main-menu-reset");
  self::semantic(&client, SemanticRole::Button, "Mute background music");
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

fn assert_px(client: &mut FakeClient<App>, name: &str, property: &str, expected: f32) {
  let id = self::named(client, name);
  let ui = client.ui();
  let style = ui.element(id).style();
  let value = match property {
    "top" => &style.top,
    "width" => &style.width,
    "height" => &style.height,
    _ => panic!("unknown property"),
  };
  assert!(matches!(
    value,
    Prop::Set(StyleValue::Value(LengthOrAuto::Px(actual))) if (*actual - expected).abs() < 0.01
  ));
}

fn assert_text(client: &FakeClient<App>, label: &str) {
  self::semantic(client, SemanticRole::StaticText, label);
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
    .expect("main menu semantics")
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
