use std::{collections::BTreeSet, time::Duration};

use battlement::{Connect, GameObjectKind, PreparedAsset, ScreenSize, Vector3, object_id};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use battlement_hearts_rules::{
  self as hearts, assets, card_assets,
  domain::{HeartsState, IgnorePresentation, Intention, Seat, cards, transition},
};
use reactant_rules::ReducerGame;
use reactant_testing::Display;

#[test]
fn every_logical_card_has_its_own_imported_face_and_model_with_one_shared_back() {
  let declaration: serde_json::Value =
    serde_json::from_str(include_str!("../../../project-assets.json")).unwrap();
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
  let catalog = self::catalog();
  let restored = HeartsState::new(73);
  let expected = self::visible_models(&restored);
  let mut display = Display::mount(
    move || hearts::application_from_state(restored.clone()),
    catalog,
  );
  display.flush();
  assert_eq!(self::displayed_models(&display), expected);
  let reset = display.find_ui(
    object_id!("6644ed66-12dc-4590-9af8-19d174a47000"),
    "new-game",
  );
  display.click_ui(reset);
  display.flush();
  assert_eq!(self::displayed_models(&display), expected);
  display.activate_accessible("Start new game");
  display.flush();
  let fresh = self::visible_models(&HeartsState::new(43));
  assert_ne!(expected, fresh);
  assert_eq!(self::displayed_models(&display), fresh);
}

fn visible_models(state: &HeartsState) -> BTreeSet<String> {
  state
    .hand(Seat::South)
    .iter()
    .map(|card| card_assets::model(*card).into_string())
    .collect()
}

fn displayed_models(display: &Display) -> BTreeSet<String> {
  let models: Vec<_> = display
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::Prefab { address, .. } if address.as_str().starts_with("hearts/cards/") => {
        Some(address.as_str().to_owned())
      }
      _ => None,
    })
    .collect();
  assert_eq!(models.len(), 13);
  let backs: Vec<_> = display
    .world()
    .objects()
    .filter_map(|object| match object.kind() {
      GameObjectKind::Image { image } => Some(image.texture.as_str()),
      _ => None,
    })
    .collect();
  assert_eq!(backs, vec![assets::hearts::cards::BACK.as_str(); 39]);
  models.into_iter().collect()
}

#[test]
fn landscape_and_portrait_keep_every_human_card_inside_the_playing_area() {
  for (width, height) in [(1280, 720), (720, 1280)] {
    let mut display = Display::mount_with(
      hearts::application,
      self::catalog(),
      Connect::new("test", "test", ScreenSize::new(width, height)),
    );
    display.flush();
    display.activate_accessible("Got it");
    display.flush();
    display.settle();
    assert_eq!(
      self::displayed_models(&display),
      self::visible_models(&HeartsState::new(43))
    );
    for object in display.world().objects() {
      let GameObjectKind::Prefab { address, .. } = object.kind() else {
        continue;
      };
      if !address.as_str().starts_with("hearts/cards/") {
        continue;
      }
      for x in [-2.15731, 2.15731] {
        for y in [-3.0, 3.0] {
          let point = display
            .project_world(
              display
                .world()
                .world_point(object.id(), Vector3::new(x, y, 0.0)),
            )
            .unwrap();
          assert!(
            point.x >= 8.0 && point.x <= f64::from(width) - 8.0,
            "card clipped horizontally: {point:?}"
          );
          assert!(
            point.y >= 90.0 && point.y <= f64::from(height) - 8.0,
            "card clipped vertically: {point:?}"
          );
        }
      }
    }
  }
}

pub(crate) fn catalog() -> FakeAssetCatalog {
  let mut catalog = FakeAssetCatalog::new();
  for asset in assets::ASSET_CATALOG {
    match asset {
      PreparedAsset::Scene(address) => catalog.add_scene(address.clone()),
      PreparedAsset::UiFont(address) => catalog.add_ui_font(address.clone()),
      PreparedAsset::Texture(address) => catalog.add_texture(address.clone()),
      PreparedAsset::Prefab(address) => catalog.add_prefab(
        address.clone(),
        if address.as_str().starts_with("hearts/particles/") {
          FakePrefab::new().with_particle_systems()
        } else {
          FakePrefab::new()
        },
      ),
      PreparedAsset::AudioClip(address) => catalog.add_audio_clip(address.clone()),
      PreparedAsset::Material(address) => catalog.add_material(address.clone()),
      _ => panic!("unexpected Hearts asset: {asset:?}"),
    }
  }
  for address in [
    assets::hearts::particles::ACCENT,
    assets::hearts::particles::MOTES,
  ] {
    catalog.add_particle_effect(address);
  }
  catalog
}

#[test]
fn music_and_effects_controls_preserve_playhead_and_new_game_disposes_old_audio() {
  let mut initial = HeartsState::new(43);
  for seat in [Seat::West, Seat::North, Seat::East] {
    let cards = initial.hand(seat).iter().take(3).copied().collect();
    transition::apply(
      &mut initial,
      Intention::SubmitPass { seat, cards },
      &mut IgnorePresentation,
    )
    .unwrap();
  }
  let mut display = Display::mount(
    move || hearts::application_from_state(initial.clone()),
    self::catalog(),
  );
  display.flush();
  let music: Vec<_> = display
    .audio_occurrences()
    .iter()
    .filter(|s| s.looping)
    .collect();
  assert_eq!(music.len(), 1);
  let id = music[0].command_id;
  assert_eq!(display.world().audio_mix().music, 0.22);
  assert_eq!(display.world().audio_mix().effects, 0.65);
  display.activate_accessible("Music on");
  display.flush();
  assert_eq!(display.audio(id).unwrap().output_volume(), 0.0);
  assert_eq!(display.world().audio_mix().effects, 0.65);
  display.activate_accessible("Effects on");
  display.flush();
  assert_eq!(display.world().audio_mix().effects, 0.0);
  display.activate_accessible("Music off");
  display.flush();
  assert_eq!(display.audio(id).unwrap().output_volume(), 0.22);
  assert_eq!(
    display
      .audio_occurrences()
      .iter()
      .filter(|s| s.looping)
      .count(),
    1
  );
  display.set_application_state(battlement::application::ApplicationState {
    focused: false,
    paused: true,
  });
  display.flush();
  display.set_application_state(battlement::application::ApplicationState::default());
  display.flush();
  assert_eq!(
    display
      .audio_occurrences()
      .iter()
      .filter(|s| s.looping)
      .count(),
    1
  );
  for card in ["Seven of Clubs", "Queen of Clubs", "King of Clubs"] {
    display.activate_accessible(card);
    display.flush();
  }
  display.activate_accessible("Pass three cards");
  assert!(
    display.wait_for_game_output::<ReducerGame<hearts::HeartsReducer>>(Duration::from_secs(10))
  );
  display.flush();
  let pass_cues = |display: &Display| {
    display
      .audio_occurrences()
      .iter()
      .filter(|sound| sound.address == assets::hearts::audio::PASS)
      .count()
  };
  display.settle();
  assert_eq!(pass_cues(&display), 1);
  display.activate_accessible("Music on");
  display.flush();
  assert_eq!(pass_cues(&display), 1);
  display.activate_accessible("New game");
  display.flush();
  display.activate_accessible("Start new game");
  display.flush();
  assert!(display.audio(id).is_none());
  display.settle();
  display.flush();
  assert_eq!(
    display
      .audio_occurrences()
      .iter()
      .filter(|s| s.looping)
      .count(),
    2
  );
  assert_eq!(display.world().audio_mix().effects, 0.0);
}
