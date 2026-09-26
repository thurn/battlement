use std::collections::BTreeSet;

use battlement::{GameObjectKind, PreparedAsset, object_id};
use battlement_fake::assets::FakeAssetCatalog;
use battlement_hearts_rules::{
  self as hearts, assets, card_assets,
  domain::{HeartsState, Seat, cards},
};
use reactant_testing::Display;

#[test]
fn every_logical_card_has_its_own_imported_face_and_model_with_one_shared_back() {
  let declaration: serde_json::Value =
    serde_json::from_str(include_str!("../../project-assets.json")).unwrap();
  let addresses = declaration["addresses"].as_array().unwrap();
  let models = declaration["models"].as_array().unwrap();
  let materials = declaration["materials"].as_array().unwrap();
  let mut seen = BTreeSet::new();
  for card in cards::deck() {
    let model = card_assets::model(card);
    let face = card_assets::face(card);
    assert!(seen.insert(face.as_str().to_owned()));
    assert!(assets::ASSET_CATALOG.contains(&PreparedAsset::Prefab(model.clone())));
    assert!(assets::ASSET_CATALOG.contains(&PreparedAsset::Texture(face.clone())));
    let model_path = &addresses
      .iter()
      .find(|item| item["address"] == model.as_str())
      .unwrap()["path"];
    let face_path = &addresses
      .iter()
      .find(|item| item["address"] == face.as_str())
      .unwrap()["path"];
    let stem = format!("{:?}_{:?}", card.suit, card.rank).to_lowercase();
    assert!(
      model_path
        .as_str()
        .unwrap()
        .ends_with(&format!("/card_{stem}.fbx"))
    );
    assert!(
      face_path
        .as_str()
        .unwrap()
        .ends_with(&format!("/{stem}.png"))
    );
    let imported = models
      .iter()
      .find(|item| &item["path"] == model_path)
      .unwrap();
    let face_material = &imported["materials"][format!("card_{stem}")];
    assert_eq!(
      &materials
        .iter()
        .find(|item| &item["path"] == face_material)
        .unwrap()["texture"],
      face_path
    );
    assert_eq!(
      imported["materials"]["card_cardback_A"],
      "Assets/Generated/Imported/Card Back.mat"
    );
  }
  assert_eq!(seen.len(), 52);
  assert_eq!(
    addresses
      .iter()
      .filter(|item| item["address"] == "hearts/cards/back")
      .count(),
    1
  );
}

#[test]
fn restoring_then_resetting_displays_only_the_current_human_hand_and_shared_back() {
  let mut catalog = FakeAssetCatalog::new();
  catalog.add_scene(assets::hearts::CONTENT);
  catalog.add_ui_font(assets::hearts::fonts::CONTROL);
  catalog.add_texture(assets::hearts::cards::BACK);
  for card in cards::deck() {
    catalog.add_texture(card_assets::face(card));
  }
  let restored = HeartsState::new(73);
  let expected = self::visible_textures(&restored);
  let mut display = Display::mount(
    move || hearts::application_from_state(restored.clone()),
    catalog,
  );
  display.flush();
  assert_eq!(self::displayed_textures(&display), expected);
  let reset = display.find_ui(
    object_id!("6644ed66-12dc-4590-9af8-19d174a47000"),
    "new-game",
  );
  display.click_ui(reset);
  display.flush();
  let fresh = self::visible_textures(&HeartsState::new(43));
  assert_ne!(expected, fresh);
  assert_eq!(self::displayed_textures(&display), fresh);
}

fn visible_textures(state: &HeartsState) -> BTreeSet<String> {
  state
    .hand(Seat::South)
    .iter()
    .map(|card| card_assets::face(*card).into_string())
    .chain([assets::hearts::cards::BACK.into_string()])
    .collect()
}

fn displayed_textures(display: &Display) -> BTreeSet<String> {
  let textures: Vec<_> = display
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::Image { image } => Some(image.texture.as_str().to_owned()),
      _ => None,
    })
    .collect();
  assert_eq!(textures.len(), 14);
  textures.into_iter().collect()
}
