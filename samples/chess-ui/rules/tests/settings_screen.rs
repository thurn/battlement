use battlement::{
  AccessibilitySnapshot, CheckedState, CommandBody, GameObjectKind, ObjectId, SemanticRole,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{
  action_button, background_music::BACKGROUND_MUSIC, engine, select_control, setting_row,
};

#[test]
fn complete_screen_keeps_settings_across_tabs_and_emits_return() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-38");
  self::semantic(&client, SemanticRole::Heading, "Settings");
  self::assert_selected_tab(&client, "Gameplay");
  self::semantic(&client, SemanticRole::Button, "RETURN");
  self::named(&mut client, "settings-panel");
  self::named(&mut client, "arcade-frame-pulse");

  self::choose(&mut client, "Language English", "Deutsch");
  self::toggle(&mut client, "Increase Move Duration");
  self::select_tab(&mut client, "Graphics");
  self::choose(&mut client, "Resolution 1920 × 1080", "3840 × 2160");
  self::toggle(&mut client, "VSync");
  self::select_tab(&mut client, "Sound");
  self::set_slider(&mut client, "Effects Volume", 28.0);
  self::select_tab(&mut client, "Gameplay");

  self::semantic(&client, SemanticRole::Button, "Language Deutsch");
  self::assert_checkbox(&client, "Increase Move Duration", false);
  self::select_tab(&mut client, "Graphics");
  self::semantic(&client, SemanticRole::Button, "Resolution 3840 × 2160");
  self::assert_checkbox(&client, "VSync", false);
  self::select_tab(&mut client, "Sound");
  self::assert_slider(&client, "Effects Volume", 28.0);

  self::click_semantic(&mut client, SemanticRole::Button, "RETURN");
  self::assert_text(&client, "Return requests: 1 · Privacy requests: 0");
}

#[test]
fn dialogs_input_and_reset_are_integrated_without_erasing_host_data() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-38");

  self::click_named(&mut client, "toggle-info");
  self::semantic(
    &client,
    SemanticRole::Dialog,
    "Crash report upload information",
  );
  self::click_semantic(&mut client, SemanticRole::Link, "Privacy Policy");
  self::assert_text(&client, "Return requests: 0 · Privacy requests: 1");
  self::click_semantic(&mut client, SemanticRole::Button, "OK");
  self::assert_no_dialog(&client);

  self::click_named(&mut client, "erase-control-button");
  self::semantic(&client, SemanticRole::Dialog, "Erase Saved Data?");
  self::click_semantic(&mut client, SemanticRole::Button, "Erase");
  self::assert_no_dialog(&client);

  self::select_tab(&mut client, "Input");
  self::semantic(&client, SemanticRole::Table, "Input bindings");
  self::click_semantic(
    &mut client,
    SemanticRole::Button,
    "Change Move Piece keyboard binding",
  );
  self::assert_text(&client, "Press a key for Move Piece");
  self::click_semantic(&mut client, SemanticRole::Button, "Cancel");

  self::click_named(&mut client, "settings-screen-reset");
  self::assert_selected_tab(&client, "Gameplay");
  self::semantic(&client, SemanticRole::Button, "Language English");
  self::assert_checkbox(&client, "Increase Move Duration", true);
  self::assert_text(&client, "Return requests: 0 · Privacy requests: 0");
  self::assert_no_dialog(&client);
}

fn choose(client: &mut FakeClient<App>, trigger: &str, option: &str) {
  self::click_semantic(client, SemanticRole::Button, trigger);
  self::click_semantic(client, SemanticRole::Option, option);
}

fn select_tab(client: &mut FakeClient<App>, label: &str) {
  self::click_semantic(client, SemanticRole::Tab, label);
  self::assert_selected_tab(client, label);
}

fn toggle(client: &mut FakeClient<App>, label: &str) {
  let target = self::semantic(client, SemanticRole::Checkbox, label);
  client.ui().toggle_click(target);
  client.poll();
}

fn set_slider(client: &mut FakeClient<App>, label: &str, value: f32) {
  let target = self::semantic(client, SemanticRole::Slider, label);
  client.ui().slider_begin(target);
  client.ui().slider_change(target, value);
  client.ui().slider_commit(target);
  client.poll();
}

fn assert_selected_tab(client: &FakeClient<App>, label: &str) {
  let node = self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Tab && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing tab {label}"));
  assert_eq!(node.state.selected, Some(true));
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

fn assert_slider(client: &FakeClient<App>, label: &str, expected: f64) {
  let node = self::snapshot(client)
    .nodes
    .iter()
    .find(|node| node.role == SemanticRole::Slider && node.label.as_deref() == Some(label))
    .unwrap_or_else(|| panic!("missing slider {label}"));
  assert_eq!(node.value.as_ref().expect("slider range").current, expected);
}

fn assert_text(client: &FakeClient<App>, label: &str) {
  self::semantic(client, SemanticRole::StaticText, label);
}

fn assert_no_dialog(client: &FakeClient<App>) {
  assert!(
    !self::snapshot(client)
      .nodes
      .iter()
      .any(|node| node.role == SemanticRole::Dialog)
  );
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
    .expect("settings screen semantics")
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
