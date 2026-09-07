use battlement::{AccessibilitySnapshot, CheckedState, CommandBody, ObjectId, SemanticRole};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{
  action_button, background_music::BACKGROUND_MUSIC, engine, select_control, setting_row,
};

#[test]
fn provider_combines_volume_and_background_mute_without_pausing() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-31");
  self::assert_label(&client, "Playback: stopped");
  self::assert_label(&client, "Visibility: visible");
  self::assert_label(&client, "Effective: 52%");
  self::assert_label(&client, "Output: audible");
  self::assert_slider(&client, "Master Volume", 80.0);
  self::assert_slider(&client, "Music Volume", 65.0);
  self::assert_checkbox(&client, "Mute in Background", false);

  let command_start = client.commands().len();
  self::click_named(&mut client, "background-music-start");
  let play = client.commands()[command_start..]
    .iter()
    .find_map(|entry| match &entry.command.body {
      CommandBody::AudioPlay(payload) => Some(payload),
      _ => None,
    })
    .expect("start emits an audio play command");
  assert_eq!(play.address, BACKGROUND_MUSIC);
  assert_eq!(play.volume, 0.52);
  assert!(play.r#loop);
  self::assert_label(&client, "Playback: playing");

  let master = self::semantic(&client, SemanticRole::Slider, "Master Volume");
  client.ui().slider_begin(master);
  client.ui().slider_change(master, 50.0);
  client.ui().slider_commit(master);
  client.poll();
  self::assert_slider(&client, "Master Volume", 50.0);
  self::assert_label(&client, "Effective: 32%");
  self::assert_volume_command(&client, 0.325);

  let mute = self::semantic(&client, SemanticRole::Checkbox, "Mute in Background");
  client.ui().toggle_click(mute);
  client.poll();
  self::assert_checkbox(&client, "Mute in Background", true);
  self::assert_label(&client, "Output: audible");

  self::click_named(&mut client, "background-music-visibility");
  self::assert_label(&client, "Playback: playing");
  self::assert_label(&client, "Visibility: hidden");
  self::assert_label(&client, "Output: muted");
  self::assert_volume_command(&client, 0.0);

  self::click_named(&mut client, "background-music-visibility");
  self::assert_label(&client, "Visibility: visible");
  self::assert_label(&client, "Output: audible");
  self::assert_volume_command(&client, 0.325);

  self::click_named(&mut client, "background-music-reset");
  self::assert_label(&client, "Playback: stopped");
  self::assert_label(&client, "Effective: 52%");
  self::assert_slider(&client, "Master Volume", 80.0);
  self::assert_slider(&client, "Music Volume", 65.0);
  self::assert_checkbox(&client, "Mute in Background", false);
  assert!(
    client
      .commands()
      .iter()
      .any(|entry| { matches!(entry.command.body, CommandBody::AudioStop(_)) })
  );
}

#[test]
fn unavailable_playback_is_explicit_and_reselection_resets_the_provider() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-31");
  self::click_named(&mut client, "background-music-availability");
  self::assert_label(&client, "Playback: unavailable");

  let command_start = client.commands().len();
  self::click_named(&mut client, "background-music-start");
  assert!(
    !client.commands()[command_start..]
      .iter()
      .any(|entry| { matches!(entry.command.body, CommandBody::AudioPlay(_)) })
  );
  self::assert_label(&client, "Playback: unavailable");

  self::click_named(&mut client, "review-page-31");
  self::assert_label(&client, "Playback: stopped");
  self::assert_label(&client, "Visibility: visible");
  self::assert_label(&client, "Output: audible");
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

fn assert_label(client: &FakeClient<App>, label: &str) {
  self::semantic(client, SemanticRole::StaticText, label);
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
    .expect("background music semantics")
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
      battlement::GameObjectKind::UiDocument(document) => Some(document.root_id()),
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
