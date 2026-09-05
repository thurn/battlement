use std::collections::BTreeSet;

use battlement::{
  AccessibilitySnapshot, CommandBody, GameObjectKind, ObjectId, SemanticRole, UiElement,
};
use battlement_fake::{assets::FakeAssetCatalog, client::FakeClient};
use battlement_reactant::{app::App, asset_generator};
use battlement_rules::{action_button, engine, select_control, setting_row};

#[test]
fn audit_uses_only_retained_assets_and_exposes_every_runtime_treatment() {
  assert_eq!(
    asset_generator::registrations()
      .map(|asset| asset.source_symbol)
      .collect::<BTreeSet<_>>(),
    BTreeSet::from([
      "battlement_rules::assets::ACTION_LABEL_ABOUT",
      "battlement_rules::assets::ACTION_LABEL_PLAY",
      "battlement_rules::assets::ACTION_LABEL_QUIT",
      "battlement_rules::assets::ACTION_LABEL_RETURN",
      "battlement_rules::assets::ACTION_LABEL_SETTINGS",
      "battlement_rules::assets::SETTINGS_PANEL_FRAME",
      "battlement_rules::header_artwork::GAME_LOGO",
      "battlement_rules::header_artwork::SETTINGS_TITLE",
      "battlement_rules::header_artwork::STRIPE_LEFT",
      "battlement_rules::header_artwork::STRIPE_RIGHT",
    ])
  );

  let mut client = self::client();
  self::click_named(&mut client, "review-page-23");
  client.poll();

  for (label, slug) in [
    ("PLAY", "play"),
    ("SETTINGS", "settings"),
    ("ABOUT", "about"),
    ("QUIT", "quit"),
    ("RETURN", "return"),
  ] {
    assert!(self::semantic_optional(&client, SemanticRole::Button, label).is_some());
    let artwork = self::named(&mut client, &format!("action-label-artwork-{slug}"));
    assert!(matches!(
      client.ui().element(artwork).element(),
      UiElement::Image(_)
    ));
  }

  self::select(&mut client, "tabs");
  assert!(self::semantic_optional(&client, SemanticRole::Tab, "Gameplay").is_some());
  assert!(self::semantic_optional(&client, SemanticRole::Tab, "Graphics").is_some());

  self::select(&mut client, "checkboxes");
  assert!(self::semantic_optional(&client, SemanticRole::Checkbox, "Checked").is_some());
  assert!(self::semantic_optional(&client, SemanticRole::Checkbox, "Unchecked").is_some());

  self::select(&mut client, "sliders");
  assert!(self::semantic_optional(&client, SemanticRole::Slider, "Minimum").is_some());
  assert!(self::semantic_optional(&client, SemanticRole::Slider, "Maximum").is_some());

  self::select(&mut client, "headings");
  assert!(
    self::semantic_optional(&client, SemanticRole::Heading, "Chess Chess Revolution").is_some()
  );
  assert!(self::semantic_optional(&client, SemanticRole::Heading, "Settings").is_some());

  self::select(&mut client, "outer-frame");
  assert!(self::named_optional(&mut client, "concept-frame").is_some());

  self::select(&mut client, "panel");
  let panel = self::named(&mut client, "settings-panel-artwork");
  assert!(matches!(
    client.ui().element(panel).element(),
    UiElement::Image(_)
  ));

  self::click_named(&mut client, "review-page-23");
  client.poll();
  assert!(self::semantic_optional(&client, SemanticRole::Button, "PLAY").is_some());
}

fn select(client: &mut FakeClient<App>, specimen: &str) {
  self::click_named(client, &format!("rendering-specimen-{specimen}"));
  client.poll();
}

fn click_named(client: &mut FakeClient<App>, name: &str) {
  let target = self::named(client, name);
  client.ui().click(target);
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
    .expect("rendering audit semantics")
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
  self::named_optional(client, name).unwrap_or_else(|| panic!("missing {name}"))
}

fn named_optional(client: &mut FakeClient<App>, name: &str) -> Option<ObjectId> {
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
      return Some(id);
    }
    pending.extend(element.children());
  }
  None
}
