use std::time::Duration;

use battlement::{
  AudioClipAddress, ObjectId, ParentScene, PrefabAddress, Prop, Vector3, object_id,
};
use reactant::{animation_controls, app::App, hooks, prelude::*, world};
use serde::Deserialize;
use trox::ls;

use crate::{CONTENT_SCENE, Game, ROOT_ID, model};

const TARGET: ObjectId = object_id!("26260000-0000-4000-8000-000000000001");
const PROJECTILE: ObjectId = object_id!("26260000-0000-4000-8000-000000000002");

#[derive(Default, Deserialize)]
struct EffectConfig {
  draw_sound: Option<String>,
  reveal_burst: Option<String>,
}

struct EffectOccurrenceProof;

pub(crate) fn app() -> App<Game> {
  App::with_model(CONTENT_SCENE, model::new())
    .ui(EffectOccurrenceProof)
    .document(|mut document| {
      document.root_id = ROOT_ID;
      document.element.picking_mode = Prop::Set(battlement::PickingMode::Ignore);
      document
    })
    .camera(|camera| {
      world::Camera::new()
        .orthographic(4.0)
        .clipping(0.1, 50.0)
        .background(Color::rgb(0.025, 0.04, 0.075))
        .position(Vector3::new(0.0, 0.0, -10.0))
        .into_object(camera.object_id)
    })
}

impl Component for EffectOccurrenceProof {
  fn render(&self) -> impl Render {
    let scope = animation_controls::use_animation_scope();
    let retained_playback = hooks::use_ref(None::<AnimationPlayback>);
    let (run, set_run) = hooks::use_state(0_u32);
    let start_scope = scope.clone();
    let retain = retained_playback.clone();
    let start = move || {
      let config: EffectConfig = ron::from_str(include_str!("effect_occurrences.ron"))
        .expect("valid effect-occurrence fixture config");
      let mut sequence = AnimationSequence::new()
        .animate(
          MotionSelector::name("projectile"),
          SequenceTarget::new(StyleTarget::new())
            .position(MotionPositionRef::identified(TARGET).follow()),
          Transition::tween().duration_secs(1.0).ease(Easing::Linear),
        )
        .at(SequencePosition::Absolute(Duration::ZERO))
        .label_at(
          "reveal",
          SequencePosition::Absolute(Duration::from_millis(500)),
        );
      if let Some(address) = config.draw_sound {
        sequence = sequence
          .play_sound(AudioClipAddress::from(address))
          .at(SequencePosition::Label("reveal".to_owned(), 0.0));
      }
      if let Some(address) = config.reveal_burst {
        sequence = sequence
          .particle_for(
            PrefabAddress::from(address),
            MotionPositionRef::identified(TARGET).follow(),
            Duration::from_millis(700),
          )
          .at(SequencePosition::Label("reveal".to_owned(), 0.0));
      }
      let playback = start_scope.start(sequence);
      set_run.set(run + 1);
      let complete = set_run.clone();
      playback.on_complete(move || complete.set(run + 2));
      retain.replace(Some(playback));
    };
    (
      View::new()
        .style(
          Style::new()
            .width(360.px())
            .padding(24.px())
            .color(Color::WHITE)
            .background_color(Color::rgb(0.05, 0.08, 0.14)),
        )
        .child((
          Heading::new(ls("Effect occurrences"), 1),
          Heading::new(
            ls(match run {
              0 => "Effect sequence: ready",
              1 => "Effect sequence: playing",
              _ => "Effect sequence: complete",
            }),
            2,
          ),
          Button::new(ls("Launch effect sequence")).on_press(start),
        )),
      world::SceneRoot::new(ParentScene::PrimaryScene).child(
        world::Group::new()
          .child((
            world::Group::new()
              .id(*PROJECTILE.as_uuid())
              .position(Vector3::new(-2.5, 0.0, 0.0))
              .child(
                world::Sprite::new()
                  .texture("reactant/assets/texture")
                  .size(0.45, 0.45),
              )
              .with_motion(MotionProps::new().motion_name("projectile")),
            world::Group::new()
              .id(*TARGET.as_uuid())
              .position(Vector3::new(2.0, 0.0, 0.0))
              .child((
                world::Sprite::new()
                  .texture("reactant/assets/texture")
                  .size(0.8, 0.8),
                world::Group::new().child(
                  world::Sprite::new()
                    .texture("reactant/assets/texture")
                    .size(1.25, 1.25),
                ),
              ))
              .with_motion(MotionProps::new().motion_name("anchor")),
          ))
          .with_motion(MotionProps::new().animation_scope(scope)),
      ),
    )
  }
}
