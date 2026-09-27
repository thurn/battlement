use std::{collections::BTreeSet, time::Duration};

use battlement::{
  AudioClipAddress, CommandBody, GameObjectKind, MotionScopeCommand, ObjectId, PreparedAsset, Rect,
  ScreenSize,
};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use reactant_rules::ReducerGame;
use reactant_testing::Display;

use crate::{app, assets, motion_fixture, reducer::HeartsReducer, test_geometry};

#[test]
fn milestones_pause_and_hold_a_complete_trick_before_collection() {
  let mut display = self::mount();
  self::action(&mut display);
  assert_eq!(self::sounds(&display, assets::hearts::audio::PASS), 0);
  display.advance(Duration::from_millis(449));
  assert_eq!(self::sounds(&display, assets::hearts::audio::PASS), 0);
  display.advance(Duration::from_millis(1));
  assert_eq!(self::sounds(&display, assets::hearts::audio::PASS), 1);
  self::settle(&mut display);
  for _ in 0..3 {
    self::action(&mut display);
    self::settle(&mut display);
  }
  self::action(&mut display);
  display.advance(Duration::from_millis(125));

  display.set_application_state(battlement::application::ApplicationState {
    focused: false,
    paused: true,
  });
  display.flush();
  display.advance(Duration::from_secs(2));
  assert_eq!(self::sounds(&display, assets::hearts::audio::LAND), 3);
  assert_eq!(self::phase(&display), "Playing: 4 cards in trick");
  display.set_application_state(battlement::application::ApplicationState::default());
  display.flush();
  display.advance(Duration::from_millis(125));
  assert_eq!(self::sounds(&display, assets::hearts::audio::LAND), 4);
  let occurrences = self::sequences(&display);
  display.advance(Duration::from_millis(649));
  assert_eq!(self::phase(&display), "Playing: 4 cards in trick");
  assert_eq!(self::sequences(&display), occurrences);
  display.advance(Duration::from_millis(1));
  assert_eq!(self::phase(&display), "Playing: 0 cards in trick");
  assert_eq!(self::sounds(&display, assets::hearts::audio::COLLECT), 0);
  display.advance(Duration::from_millis(400));
  assert_eq!(self::sounds(&display, assets::hearts::audio::COLLECT), 1);
}

#[test]
fn reduced_motion_preserves_last_trick_collection_then_score_and_new_deal() {
  let mut display = self::mount();
  self::activate(&mut display, "Load final trick");
  display.flush();
  assert_eq!(
    display
      .world()
      .objects()
      .filter(|object| matches!(object.kind(), GameObjectKind::BoxHitRegion { .. }))
      .count(),
    52
  );
  self::activate(&mut display, "Toggle reduced motion");
  display.flush();
  for _ in 0..3 {
    self::action(&mut display);
    self::settle(&mut display);
  }
  self::action(&mut display);
  display.advance(Duration::from_millis(299));
  assert_eq!(self::phase(&display), "Playing: 4 cards in trick");
  assert_eq!(self::sounds(&display, assets::hearts::audio::RESULT), 0);
  display.advance(Duration::from_millis(101));
  assert_eq!(self::phase(&display), "Scored: 0 cards in trick");
  assert_eq!(self::sounds(&display, assets::hearts::audio::COLLECT), 1);
  assert_eq!(self::sounds(&display, assets::hearts::audio::RESULT), 0);
  display.advance(Duration::from_millis(100));
  assert_eq!(self::sounds(&display, assets::hearts::audio::RESULT), 1);
  assert_eq!(
    display
      .world()
      .objects()
      .filter(|object| matches!(object.kind(), GameObjectKind::BoxHitRegion { .. }))
      .count(),
    52
  );
  assert!(display.particle_occurrences().is_empty());
  self::settle(&mut display);
  let hosts = self::card_hosts(&display);
  self::action(&mut display);
  self::settle(&mut display);
  assert_eq!(self::card_hosts(&display), hosts);
  assert_eq!(self::phase(&display), "Passing: 0 cards in trick");
  assert_eq!(self::sounds(&display, assets::hearts::audio::DEAL), 2);
}

#[test]
fn deal_blocks_input_and_retargeting_does_not_repeat_occurrences() {
  let mut display = self::mount();
  self::activate(&mut display, "Replay deal");
  display.advance(Duration::from_millis(100));
  self::action(&mut display);
  assert_eq!(self::phase(&display), "Passing: 0 cards in trick");
  self::settle(&mut display);
  assert_eq!(self::sounds(&display, assets::hearts::audio::DEAL), 2);
  self::action(&mut display);
  display.advance(Duration::from_millis(150));
  let occurrences = self::sequences(&display);
  test_geometry::observe_viewport(
    &mut display,
    1,
    ScreenSize::new(720, 1280),
    Rect {
      x: 0.0,
      y: 0.0,
      width: 720.0,
      height: 1280.0,
    },
  );
  self::settle(&mut display);
  assert_eq!(self::sounds(&display, assets::hearts::audio::PASS), 1);
  assert_eq!(self::sequences(&display), occurrences + 1);
  let sounds = display.audio_occurrences().len();
  test_geometry::observe_viewport(
    &mut display,
    2,
    ScreenSize::new(1280, 720),
    Rect {
      x: 0.0,
      y: 0.0,
      width: 1280.0,
      height: 720.0,
    },
  );
  self::settle(&mut display);
  assert_eq!(display.audio_occurrences().len(), sounds);
  assert_eq!(self::sequences(&display), occurrences + 1);
}

fn phase(display: &Display) -> &str {
  display
    .ui_element(display.find_ui(app::ROOT, "motion-phase"))
    .text()
    .unwrap()
}

fn sounds(display: &Display, address: AudioClipAddress) -> usize {
  display
    .audio_occurrences()
    .iter()
    .filter(|sound| sound.address == address)
    .count()
}

fn sequences(display: &Display) -> usize {
  display
    .commands()
    .iter()
    .filter(|entry| {
      matches!(&entry.command.body,
    CommandBody::MotionScope(value) if matches!(value.command, MotionScopeCommand::Start { .. }))
    })
    .count()
}

fn action(display: &mut Display) {
  self::activate(display, "Advance Hearts action");
  for _ in 0..10 {
    display.flush();
    if display.game_status::<ReducerGame<HeartsReducer>>() == Some(reactant::GameStatus::Ready) {
      return;
    }
    assert!(display.wait_for_game_output::<ReducerGame<HeartsReducer>>(Duration::from_secs(10)));
  }
  panic!("rules publication did not complete");
}

fn settle(display: &mut Display) {
  display.flush();
  display.settle();
  display.flush();
}

fn mount() -> Display {
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
  let mut display = Display::mount(motion_fixture::application, catalog);
  self::settle(&mut display);
  display
}

fn activate(display: &mut Display, name: &str) {
  let event = display.button_event(name);
  display.deliver_ui_event(event);
  display.flush();
}

fn card_hosts(display: &Display) -> BTreeSet<ObjectId> {
  display
    .world()
    .objects()
    .filter(|object| matches!(object.kind(), GameObjectKind::BoxHitRegion { .. }))
    .map(|object| object.id())
    .collect()
}
