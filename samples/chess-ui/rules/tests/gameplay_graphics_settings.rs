use battlement::{
  AccessibilitySnapshot, CheckedState, CommandBody, GameObjectKind, LengthOrAuto, ObjectId, Prop,
  SemanticRole, StyleValue, UiAccessibilityAction, UiAccessibilityActionEvent, UiElement, UiEvent,
  UiEventBody, Vector,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn gameplay_and_graphics_keep_every_value_controlled_and_reset_together() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-35");

  self::assert_button(&client, "Language English");
  self::assert_button(&client, "Text Size 100%");
  self::assert_checkbox(&client, "Reduce Motion", false);
  self::assert_checkbox(&client, "Increase Move Duration", true);
  self::assert_checkbox(&client, "Upload Crash Reports", true);
  self::assert_button(&client, "Erase Saved Data");

  self::choose(&mut client, "Language English", "Deutsch");
  self::toggle(&mut client, "Reduce Motion");
  self::toggle(&mut client, "Increase Move Duration");
  self::toggle(&mut client, "Upload Crash Reports");
  self::click_named(&mut client, "toggle-info");
  self::click_named(&mut client, "erase-control-button");
  self::assert_button(&client, "Language Deutsch");
  self::assert_checkbox(&client, "Reduce Motion", true);
  self::assert_checkbox(&client, "Increase Move Duration", false);
  self::assert_checkbox(&client, "Upload Crash Reports", false);
  self::assert_label(&client, "Gameplay · Text 100% · Help 1 · Erase 1");

  self::click_named(&mut client, "settings-specimen-graphics");
  self::assert_button(&client, "Resolution 1920 × 1080");
  self::assert_button(&client, "Max Framerate 144 FPS");
  self::assert_button(&client, "Display Mode Borderless");
  self::assert_checkbox(&client, "Screenshake", true);
  self::assert_checkbox(&client, "VSync", true);

  self::choose(&mut client, "Resolution 1920 × 1080", "3840 × 2160");
  self::choose(&mut client, "Max Framerate 144 FPS", "240 FPS");
  self::choose(&mut client, "Display Mode Borderless", "Windowed");
  self::toggle(&mut client, "Screenshake");
  self::toggle(&mut client, "VSync");
  self::assert_button(&client, "Resolution 3840 × 2160");
  self::assert_button(&client, "Max Framerate 240 FPS");
  self::assert_button(&client, "Display Mode Windowed");
  self::assert_checkbox(&client, "Screenshake", false);
  self::assert_checkbox(&client, "VSync", false);

  self::click_named(&mut client, "gameplay-graphics-reset");
  self::assert_button(&client, "Language English");
  self::assert_button(&client, "Text Size 100%");
  self::assert_checkbox(&client, "Increase Move Duration", true);
  self::assert_checkbox(&client, "Upload Crash Reports", true);
  self::assert_label(&client, "Gameplay · Text 100% · Help 0 · Erase 0");

  self::click_named(&mut client, "settings-specimen-graphics");
  self::assert_button(&client, "Resolution 1920 × 1080");
  self::assert_button(&client, "Max Framerate 144 FPS");
  self::assert_button(&client, "Display Mode Borderless");
  self::assert_checkbox(&client, "Screenshake", true);
  self::assert_checkbox(&client, "VSync", true);
}

#[test]
fn text_size_reflows_rows_controls_and_the_live_panel_scroll() {
  let mut client = self::client();
  self::click_named(&mut client, "review-page-35");
  self::assert_size(&mut client, "setting-row", 159.0, true);
  self::assert_size(&mut client, "select-control", 396.0, false);

  self::choose(&mut client, "Text Size 100%", "150%");
  self::assert_label(&client, "Gameplay · Text 150% · Help 0 · Erase 0");
  self::assert_size(&mut client, "setting-row", 238.5, true);
  self::assert_size(&mut client, "select-control", 546.0, false);

  self::choose(&mut client, "Text Size 150%", "200%");
  self::assert_label(&client, "Gameplay · Text 200% · Help 0 · Erase 0");
  self::assert_size(&mut client, "setting-row", 318.0, true);
  self::assert_size(&mut client, "select-control", 696.0, false);
  let ids = self::all_ids(&mut client);
  let ui = client.ui();
  let scaled_multiline_wrappers = ids
    .into_iter()
    .filter(|id| {
      let element = ui.element(*id);
      element.name() == Some("toggle-control-label")
        && matches!(
          element.style().height,
          Prop::Set(StyleValue::Value(LengthOrAuto::Px(value))) if (value - 422.0).abs() < 0.01
        )
    })
    .count();
  assert_eq!(scaled_multiline_wrappers, 2);

  let scroll = self::named(&mut client, "gameplay-graphics-scroll");
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
    panic!("expected settings scroll view")
  };
  let Prop::Set(Vector { y, .. }) = view.scroll_offset else {
    panic!("settings scroll offset was not authored")
  };
  assert!(y > 0.0);
}

fn choose(client: &mut FakeClient<App>, trigger: &str, option: &str) {
  let trigger = self::semantic(client, SemanticRole::Button, trigger);
  client.ui().click(trigger);
  client.poll();
  let option = self::semantic(client, SemanticRole::Option, option);
  client.ui().click(option);
  client.poll();
}

fn toggle(client: &mut FakeClient<App>, label: &str) {
  let target = self::semantic(client, SemanticRole::Checkbox, label);
  client.ui().toggle_click(target);
  client.poll();
}

fn assert_button(client: &FakeClient<App>, label: &str) {
  self::semantic(client, SemanticRole::Button, label);
}

fn assert_label(client: &FakeClient<App>, label: &str) {
  self::semantic(client, SemanticRole::StaticText, label);
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

fn assert_size(client: &mut FakeClient<App>, name: &str, expected: f32, height: bool) {
  let id = self::named(client, name);
  let ui = client.ui();
  let style = ui.element(id).style();
  let property = if height {
    &style.min_height
  } else {
    &style.width
  };
  let Prop::Set(StyleValue::Value(LengthOrAuto::Px(actual))) = property else {
    panic!("{name} has no pixel size")
  };
  assert!(
    (actual - expected).abs() < 0.01,
    "{name}: {actual} != {expected}"
  );
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
    .expect("settings semantics")
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
  let ui = client.ui();
  while let Some(id) = pending.pop() {
    let element = ui.element(id);
    ids.push(id);
    pending.extend(element.children());
  }
  ids
}
