use battlement::{
  AccessibilitySnapshot, CheckedState, CommandBody, GameObjectKind, ObjectId, Prop, SemanticRole,
  UiAccessibilityAction, UiAccessibilityActionEvent, UiElement, UiEvent, UiEventBody, Vector,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{
  action_button, background_music::BACKGROUND_MUSIC, engine, select_control, setting_row,
};

#[test]
fn sound_values_drive_shared_audio_and_reset_as_one_composition() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-36");
  self::assert_slider(&client, "Master Volume", 80.0);
  self::assert_slider(&client, "Music Volume", 65.0);
  self::assert_slider(&client, "Effects Volume", 75.0);
  self::assert_checkbox(&client, "Mute in Background", false);

  self::click_named(&mut client, "sound-settings-start");
  assert!(client.commands().iter().any(|entry| matches!(
    &entry.command.body,
    CommandBody::AudioPlay(play) if play.address == BACKGROUND_MUSIC && play.volume == 0.52
  )));
  self::set_slider(&mut client, "Master Volume", 50.0);
  self::set_slider(&mut client, "Music Volume", 40.0);
  self::set_slider(&mut client, "Effects Volume", 30.0);
  self::toggle(&mut client, "Mute in Background");
  self::assert_slider(&client, "Master Volume", 50.0);
  self::assert_slider(&client, "Music Volume", 40.0);
  self::assert_slider(&client, "Effects Volume", 30.0);
  self::assert_checkbox(&client, "Mute in Background", true);
  self::assert_volume_command(&client, 0.2);

  self::click_named(&mut client, "sound-settings-visibility");
  self::assert_volume_command(&client, 0.0);
  self::click_named(&mut client, "sound-settings-visibility");
  self::assert_volume_command(&client, 0.2);

  self::click_named(&mut client, "sound-settings-reset");
  self::assert_slider(&client, "Master Volume", 80.0);
  self::assert_slider(&client, "Music Volume", 65.0);
  self::assert_slider(&client, "Effects Volume", 75.0);
  self::assert_checkbox(&client, "Mute in Background", false);
  assert!(
    client
      .commands()
      .iter()
      .any(|entry| matches!(entry.command.body, CommandBody::AudioStop(_)))
  );
}

#[test]
fn large_text_reflows_and_scrolls_to_the_multiline_background_setting() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-36");
  self::click_named(&mut client, "sound-settings-text-size");
  let rows = self::all_ids(&mut client)
    .into_iter()
    .filter(|id| client.ui().element(*id).name() == Some("setting-row"))
    .collect::<Vec<_>>();
  assert_eq!(rows.len(), 4);
  assert!(rows.iter().all(|id| {
    matches!(
      client.ui().element(*id).style().min_height,
      Prop::Set(battlement::StyleValue::Value(battlement::LengthOrAuto::Px(value)))
        if value >= 318.0
    )
  }));

  let scroll = self::named(&mut client, "sound-settings-scroll");
  client.ui().send_event(UiEvent::new(
    scroll,
    true,
    false,
    UiEventBody::AccessibilityAction(UiAccessibilityActionEvent {
      backend_generation: 1,
      action: UiAccessibilityAction::ScrollForward,
    }),
  ));
  client.poll();
  let ui = client.ui();
  let UiElement::ScrollView(view) = ui.element(scroll).element() else {
    panic!("expected sound settings scroll view")
  };
  let Prop::Set(Vector { y, .. }) = view.scroll_offset else {
    panic!("sound settings scroll offset was not authored")
  };
  assert!(y > 0.0);
  self::assert_checkbox(&client, "Mute in Background", false);

  self::click_named(&mut client, "sound-settings-reset");
  let ui = client.ui();
  let UiElement::ScrollView(view) = ui.element(scroll).element() else {
    panic!("expected sound settings scroll view")
  };
  assert!(matches!(
    view.scroll_offset,
    Prop::Set(Vector { y: 0.0, .. })
  ));
}

fn set_slider(client: &mut FakeClient<App>, label: &str, value: f32) {
  let target = self::semantic(client, SemanticRole::Slider, label);
  client.ui().slider_begin(target);
  client.ui().slider_change(target, value);
  client.ui().slider_commit(target);
  client.poll();
}

fn toggle(client: &mut FakeClient<App>, label: &str) {
  let target = self::semantic(client, SemanticRole::Checkbox, label);
  client.ui().toggle_click(target);
  client.poll();
}

fn assert_slider(client: &FakeClient<App>, label: &str, expected: f64) {
  let node = self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Slider && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing slider {label}"));
  assert_eq!(node.value.as_ref().expect("slider range").current, expected);
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

fn assert_volume_command(client: &FakeClient<App>, expected: f64) {
  let actual = client
    .commands()
    .iter()
    .rev()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AudioSetVolume(command) => Some(command.payload.volume),
      _ => None,
    })
    .expect("audio volume command");
  assert_eq!(actual, expected);
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
    .expect("sound settings semantics")
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().click(target);
  client.poll();
}

fn named(client: &mut FakeClient<App>, name: &str) -> ObjectId {
  self::all_ids(client)
    .into_iter()
    .find(|id| client.ui().element(*id).name() == Some(name))
    .unwrap_or_else(|| panic!("missing {name}"))
}

fn all_ids(client: &mut FakeClient<App>) -> Vec<ObjectId> {
  let mut pending = client
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::UiDocument(document) => Some(document.root_id()),
      _ => None,
    })
    .collect::<Vec<_>>();
  let mut ids = Vec::new();
  while let Some(id) = pending.pop() {
    ids.push(id);
    pending.extend(client.ui().element(id).children());
  }
  ids
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
