use std::{
  panic::{self, AssertUnwindSafe},
  time::Duration,
};

use battlement::{
  AudioClipAddress, MaterialInstance, MaterialParameter, MaterialValue, ObjectId, ParentScene,
  object_id,
};
use battlement_fake::assets::{FakeAssetCatalog, FakePrefab};
use reactant::{hooks, host::ButtonHost, prelude::*, testing::App, world};
use reactant_testing::{Display, temporal::Clock};
use trox::ls;

const MATERIAL: ObjectId = object_id!("321b0000-0000-4000-8000-000000000001");
const MATERIAL_PEER: ObjectId = object_id!("321b0000-0000-4000-8000-000000000002");
const LIGHT: ObjectId = object_id!("321b0000-0000-4000-8000-000000000003");
const PARTICLES: ObjectId = object_id!("321b0000-0000-4000-8000-000000000004");
const AUDIO_HOST: ObjectId = object_id!("321b0000-0000-4000-8000-000000000005");
const AUDIO: ObjectId = object_id!("321b0000-0000-4000-8000-000000000006");
const CLIP: MaterialParameter<f64> = MaterialParameter::new("_Clip");

struct ParameterScene;

impl Component for ParameterScene {
  fn render(&self) -> impl Render {
    let transition = Transition::tween().duration_secs(1.0).ease(Easing::Linear);
    world::SceneRoot::new(ParentScene::PrimaryScene).child((
      world::Sprite::new()
        .id(*MATERIAL.as_uuid())
        .texture("motion/texture")
        .material(MaterialInstance::new("motion/material").parameter(CLIP, 0.2))
        .animate(StyleTarget::new().material_scalar(CLIP, 1.0))
        .transition(transition.clone()),
      world::Sprite::new()
        .id(*MATERIAL_PEER.as_uuid())
        .texture("motion/texture")
        .material(MaterialInstance::new("motion/material").parameter(CLIP, 0.4)),
      world::Light::new()
        .id(*LIGHT.as_uuid())
        .intensity(2.0)
        .animate(StyleTarget::new().light_intensity(6.0))
        .transition(transition.clone()),
      world::Prefab::at("motion/particles")
        .id(*PARTICLES.as_uuid())
        .animate(StyleTarget::new().particle_emission(12.0))
        .transition(transition),
    ))
  }
}

fn assets() -> FakeAssetCatalog {
  let mut assets = FakeAssetCatalog::new();
  assets.add_scene("motion/scene");
  assets.add_texture("motion/texture");
  assets.add_material_with_parameters("motion/material", [CLIP.value(0.0)]);
  assets.add_prefab(
    "motion/particles",
    FakePrefab::new().with_particle_systems(),
  );
  assets.add_audio_clip("motion/audio");
  assets
}

#[test]
fn continuous_world_parameters_share_motion_sampling_and_remain_instance_local() {
  let app = App::new("motion/scene").ui(ParameterScene);
  let mut display = Display::connect(app, assets());
  Clock::advance(&mut display, Duration::from_millis(500));

  let Some(MaterialValue::Float(material)) = display
    .object(MATERIAL)
    .unwrap()
    .material_parameter(0, "_Clip")
  else {
    panic!("material scalar is absent")
  };
  assert!((material - 0.6).abs() < 0.00001, "{material}");
  assert_eq!(
    display
      .object(MATERIAL_PEER)
      .unwrap()
      .material_parameter(0, "_Clip"),
    Some(&MaterialValue::Float(0.4))
  );
  assert_eq!(
    display.object(LIGHT).unwrap().light().unwrap().intensity,
    4.0
  );
  assert_eq!(
    display.object(PARTICLES).unwrap().particle_emission(),
    Some(6.0)
  );
}

struct AudioScene;

impl Component for AudioScene {
  fn render(&self) -> impl Render {
    let app = reactant::app_context::use_app();
    let playback = AudioPlayback::new(AUDIO);
    let (playing, play) = hooks::use_state(false);
    let (started, start) = hooks::use_state(false);
    let host = world::Group::new().id(*AUDIO_HOST.as_uuid());
    let host = if started {
      Node::new(
        host
          .animate(StyleTarget::new().audio_volume(playback, 0.2))
          .transition(Transition::tween().duration_secs(1.0).ease(Easing::Linear)),
      )
    } else {
      Node::new(host)
    };
    (
      ButtonHost::new(ls("Play audio"))
        .name("play-audio")
        .on_click(move || {
          app.send(playback.play_command(
            AudioClipAddress::from_static("motion/audio"),
            AudioPlaybackOptions::new().looping(true),
          ));
          play.set(true);
        }),
      ButtonHost::new(ls("Start audio motion"))
        .name("start-audio")
        .on_click(move || start.set(playing)),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(host),
    )
  }
}

#[test]
fn audio_volume_targets_one_live_playback_without_a_second_scheduler() {
  let app = App::new("motion/scene").ui(AudioScene);
  let root = app.root_document().root_id;
  let mut display = Display::connect(app, assets());
  let play = display.find_ui(root, "play-audio");
  let button = display.find_ui(root, "start-audio");
  display.click_ui(play);
  display.click_ui(button);
  Clock::advance(&mut display, Duration::from_millis(500));
  let audio = display
    .audio(battlement::CommandId::from_uuid(*AUDIO.as_uuid()).unwrap())
    .unwrap();
  assert!((audio.volume() - 0.6).abs() < 0.00001, "{audio:?}");
}

#[test]
fn material_motion_rejects_a_non_float_prepared_parameter() {
  const ACCENT: MaterialParameter<battlement::Color> = MaterialParameter::new("_Accent");
  let app = App::new("motion/scene").ui(
    world::SceneRoot::new(ParentScene::PrimaryScene).child(
      world::Sprite::new()
        .texture("motion/texture")
        .material(
          MaterialInstance::new("motion/color-material")
            .parameter(ACCENT, battlement::Color::WHITE),
        )
        .animate(StyleTarget::new().material_scalar(CLIP, 1.0)),
    ),
  );
  let mut assets = assets();
  assets.add_material_with_parameters(
    "motion/color-material",
    [ACCENT.value(battlement::Color::BLACK)],
  );
  assert!(panic::catch_unwind(AssertUnwindSafe(|| Display::connect(app, assets))).is_err());
}
